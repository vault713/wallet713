use std::sync::{Arc, Mutex};
use std::thread;
use ws::{connect, Sender, Handler, Handshake, Message, CloseCode, Result as WsResult, ErrorKind as WsErrorKind, Error as WsError};

use grin_core::libtx::slate::Slate;

use common::{Error, Wallet713Error};
use common::crypto::{SecretKey, PublicKey, Signature, verify_signature, sign_challenge, Hex, Base58};
use contacts::{Address, GrinboxAddress};

use super::types::{Publisher, Subscriber, SubscriptionHandler};
use super::protocol::{ProtocolResponse, ProtocolRequest};

pub struct GrinboxPublisher {
    address: GrinboxAddress,
    secret_key: SecretKey,
}

impl GrinboxPublisher {
    pub fn new(address: &GrinboxAddress, secert_key: &SecretKey) -> Result<Self, Error> {
        let broker = GrinboxBroker::new()?;
        Ok(Self {
            address: address.clone(),
            secret_key: secert_key.clone(),
        })
    }
}

impl Publisher for GrinboxPublisher {
    fn post_slate(&self, slate: &Slate, to: &Address) -> Result<(), Error> {
        let broker = GrinboxBroker::new()?;
        let to = GrinboxAddress::from_str(&to.to_string())?;
        broker.post_slate(slate, &to, &self.address, &self.secret_key)?;
        Ok(())
    }
}

pub struct GrinboxSubscriber {
    address: GrinboxAddress,
    broker: GrinboxBroker,
    secret_key: SecretKey,
}

impl GrinboxSubscriber {
    pub fn new(address: &GrinboxAddress, secret_key: &SecretKey) -> Result<Self, Error> {
        let broker = GrinboxBroker::new()?;
        Ok(Self {
            address: address.clone(),
            broker: GrinboxBroker::new()?,
            secret_key: secret_key.clone(),
        })
    }
}

impl Subscriber for GrinboxSubscriber {
    fn subscribe(&self, handler: Box<SubscriptionHandler + Send>) -> Result<(), Error> {
        self.broker.subscribe(&self.address, &self.secret_key, handler);
        Ok(())
    }

    fn unsubscribe(&self) -> Result<(), Error> {
        Ok(())
    }
}

struct GrinboxBroker {
    inner: Option<Arc<Mutex<GrinboxClient>>>,
}

impl GrinboxBroker {
    fn new() -> Result<Self, Error> {
        Ok(Self {
            inner: None
        })
    }

    fn post_slate(&self, slate: &Slate, to: &GrinboxAddress, from: &GrinboxAddress, secret_key: &SecretKey) -> Result<(), Error> {
        let url = {
            let to = to.clone();
            format!("ws://{}:{}", to.domain, to.port)
        };
        connect(url, move |sender| {
            move |msg: Message| {
                let response = serde_json::from_str::<ProtocolResponse>(&msg.to_string()).expect("could not parse response!");
                match response {
                    ProtocolResponse::Challenge { str } => {
                        let slate_str = serde_json::to_string(&slate).unwrap();
                        let mut challenge = String::new();
                        challenge.push_str(&slate_str);
                        challenge.push_str(&str);
                        let signature = GrinboxClient::generate_signature(&challenge, secret_key);
                        let request = ProtocolRequest::PostSlate {
                            from: from.stripped(),
                            to: to.public_key.clone(),
                            str: slate_str,
                            signature,
                        };
                        sender.send(serde_json::to_string(&request).unwrap()).unwrap();
                        sender.close(CloseCode::Normal);
                    },
                    _ => {}
                }
                Ok(())
            }
        });
        Ok(())
    }

    fn subscribe(&self, address: &GrinboxAddress, secret_key: &SecretKey, handler: Box<SubscriptionHandler + Send>) -> Result<(), Error> {
        let handler = Arc::new(Mutex::new(handler));
        let url = {
            let cloned_address = address.clone();
            format!("ws://{}:{}", cloned_address.domain, cloned_address.port)
        };
        let secret_key = secret_key.clone();
        let cloned_address = address.clone();
        thread::spawn(move || {
            connect(url, move |sender| GrinboxClient {
                sender,
                handler: handler.clone(),
                challenge: None,
                address: cloned_address.clone(),
                secret_key,
            });
        });
        Ok(())
    }
}

struct GrinboxClient {
    sender: Sender,
    handler: Arc<Mutex<Box<SubscriptionHandler + Send>>>,
    challenge: Option<String>,
    address: GrinboxAddress,
    secret_key: SecretKey,
}

impl GrinboxClient {
    fn generate_signature(challenge: &str, secret_key: &SecretKey) -> String {
        let signature = sign_challenge(challenge, secret_key).expect("could not sign challenge!");
        signature.to_hex()
    }

    fn subscribe(&self, challenge: &str) -> Result<(), Error> {
        let signature = GrinboxClient::generate_signature(challenge, &self.secret_key);
        let request = ProtocolRequest::Subscribe { address: self.address.public_key.to_string(), signature };
        self.send(&request).expect("could not send subscribe request!");
        Ok(())
    }

    fn verify_slate_signature(&self, from: &str, str: &str, challenge: &str, signature: &str) -> Result<(), Error> {
        let from = GrinboxAddress::from_str(from)?;
        let public_key = PublicKey::from_base58_check(&from.public_key, 2)?;
        let signature = Signature::from_hex(signature)?;
        let mut challenge_builder = String::new();
        challenge_builder.push_str(str);
        challenge_builder.push_str(challenge);
        verify_signature(&challenge_builder, &signature, &public_key)?;
        Ok(())
    }

    fn send(&self, request: &ProtocolRequest) -> Result<(), Error> {
        let request = serde_json::to_string(&request).unwrap();
        self.sender.send(request)?;
        Ok(())
    }
}

impl Handler for GrinboxClient {
    fn on_open(&mut self, shake: Handshake) -> WsResult<()> {
        self.handler.lock().unwrap().on_open();
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> WsResult<()> {
        //TODO: map err
        let response = serde_json::from_str::<ProtocolResponse>(&msg.to_string()).expect("could not parse response!");
        match response {
            ProtocolResponse::Challenge { str } => {
                self.challenge = Some(str.clone());
                self.subscribe(&str);
            },
            ProtocolResponse::Slate { from, str, challenge, signature } => {
                if let Ok(_) = self.verify_slate_signature(&from, &str, &challenge, &signature) {
                    //TODO: map err
                    let mut slate: Slate = serde_json::from_str(&str).expect("could not parse slate!");
                    //TODO: map err
                    let from = GrinboxAddress::from_str(&from).expect("could not parse address!");
                    self.handler.lock().unwrap().on_slate(&from, &mut slate);
                } else {
                    cli_message!("{}: received slate with invalid signature!", "ERROR".bright_red());
                }
            },
            ProtocolResponse::Error { kind: _, description: _ } => {
                cli_message!("{}", response);
            },
            _ => {}
        }
        Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        self.handler.lock().unwrap().on_close();
    }
}