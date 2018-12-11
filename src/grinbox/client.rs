use std::sync::{Arc, Mutex};
use std::str::FromStr;

use ws::{connect, Sender};

use grin_core::libtx::slate::Slate;
use common::{Wallet713Error, Result};
use common::crypto::{SecretKey, PublicKey, Signature, public_key_from_secret_key, verify_signature, sign_challenge, Hex, Base58, BASE58_CHECK_VERSION_GRIN_TX};
use super::protocol::{ProtocolRequest, ProtocolResponse};
use super::types::GrinboxAddress;

pub struct GrinboxClientOut {
    domain: String,
    port: u16,
    challenge: String,
    sender: Sender,
    public_key: String,
    private_key: String,
}

impl GrinboxClientOut {
    pub fn subscribe(&self) -> Result<()> {
        let signature = self.generate_signature(&self.challenge);
        let request = ProtocolRequest::Subscribe { address: self.public_key.to_string(), signature };
        self.send(&request).expect("could not send subscribe request!");
        Ok(())
    }

    pub fn unsubscribe(&self) -> Result<()> {
        let request = ProtocolRequest::Unsubscribe { address: self.public_key.to_string() };
        self.send(&request).expect("could not send unsubscribe request!");
        Ok(())
    }

    pub fn post_slate(&self, to: &str, slate: &Slate) -> Result<()> {
        let to = GrinboxAddress::from_str(to)?;
        let from = GrinboxAddress {
            public_key: self.public_key.clone(),
            domain: Some(self.domain.clone()),
            port: Some(self.port)
        };

        let str = serde_json::to_string(&slate).unwrap();

        let is_local = from.local_to(&to);

        if !is_local {
            self.post_slate_federated(from, to, str)?;
        } else {
            self.post_slate_direct(from, to, str)?;
        }
        Ok(())
    }

    fn post_slate_direct(&self, from: GrinboxAddress, to: GrinboxAddress, str: String) -> Result<()> {
        let mut challenge = String::new();
        challenge.push_str(&str);
        challenge.push_str(&self.challenge);
        let signature = self.generate_signature(&challenge);
        let response = ProtocolRequest::PostSlate {
            from: from.to_string(),
            to: to.to_string(),
            str,
            signature
        };
        self.send(&response).expect("could not send slate!");
        Ok(())
    }

    fn post_slate_federated(&self, from: GrinboxAddress, to: GrinboxAddress, str: String) -> Result<()> {
        let from_clone = from.clone();
        let to_clone = to.clone();
        let str_clone = str.clone();
        let uri = format!("ws://{}:{}", &to.domain.unwrap(), to.port.unwrap());
        let public_key = self.public_key.clone();
        let private_key = self.private_key.clone();
        let domain = self.domain.clone();
        let port = self.port;
        std::thread::spawn(move || {
            if let Err(_) = connect(uri, move |sender| {
                let str_clone = str_clone.clone();
                let from_clone = from_clone.clone();
                let to_clone = to_clone.clone();
                let client = GrinboxClientOut {
                    domain: domain.clone(),
                    port,
                    sender,
                    public_key: public_key.clone(),
                    private_key: private_key.clone(),
                    challenge: "".to_string(),
                };

                let client = Arc::new(Mutex::new(client));
                move |msg| {
                    let mut guard = client.lock().unwrap();
                    let ref mut client = *guard;
                    client.process_incoming(msg, None, false).expect("failed processing incoming message!");
                    client.post_slate_direct(from_clone.clone(), to_clone.clone(), str_clone.clone()).expect("failed posting slate!");
                    client.sender.close(ws::CloseCode::Normal)?;
                    Ok(())
                }
            }) {
                cli_message!("{}: could not connect to grinbox!", "ERROR".bright_red());
            };
        });
        Ok(())
    }

    pub fn get_challenge(&self) -> String {
        self.challenge.clone()
    }

    pub fn get_private_key(&self) -> String {
        self.private_key.clone()
    }

    fn generate_signature(&self, challenge: &str) -> String {
        let secret_key = SecretKey::from_str(&self.private_key).expect("could not construct secret key!");
        let signature = sign_challenge(challenge, &secret_key).expect("could not sign challenge!");
        signature.to_hex()
    }

    fn verify_slate_signature(&self, from: &str, str: &str, challenge: &str, signature: &str) -> Result<()> {
        let from = GrinboxAddress::from_str(from)?;
        let public_key = PublicKey::from_base58_check(&from.public_key, 2)?;
        let signature = Signature::from_hex(signature)?;
        let mut challenge_builder = String::new();
        challenge_builder.push_str(str);
        challenge_builder.push_str(challenge);
        verify_signature(&challenge_builder, &signature, &public_key)?;
        Ok(())
    }

    fn send(&self, request: &ProtocolRequest) -> Result<()> {
        let request = serde_json::to_string(&request).unwrap();
        self.sender.send(request)?;
        Ok(())
    }

    fn process_incoming(&mut self, msg: ws::Message, handler: Option<Box<GrinboxClientHandler + Send>>, subscribe: bool) -> Result<()> {
        let response = serde_json::from_str::<ProtocolResponse>(&msg.to_string())?;
        match response {
            ProtocolResponse::Challenge { str } => {
                self.challenge = str;
                if subscribe {
                    cli_message!("subscribing to [{}]", self.public_key.bright_green());
                    self.subscribe()?;
                }
            },
            ProtocolResponse::Slate { from, str, challenge, signature } => {
                if let Some(handler) = handler {
                    if let Ok(_) = self.verify_slate_signature(&from, &str, &challenge, &signature) {
                        handler.on_response(&ProtocolResponse::Slate { from, str, challenge, signature }, &self);
                    } else {
                        cli_message!("{}: received slate with invalid signature!", "ERROR".bright_red());
                    }
                }
            },
            _ => if let Some(handler) = handler {
                handler.on_response(&response, &self)
            }
        }

        Ok(())
    }
}

pub struct GrinboxClient {
    domain: String,
    port: u16,
    out: Arc<Mutex<Option<GrinboxClientOut>>>,
}

impl GrinboxClient {
    pub fn new() -> Self {
        GrinboxClient {
            domain: "".to_string(),
            port: 0,
            out: Arc::new(Mutex::new(None)),
        }
    }

    pub fn get_domain(&self) -> &String {
        &self.domain
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub fn is_started(&self) -> bool {
        if let Some(ref _out) = *self.out.lock().unwrap() {
            true
        } else {
            false
        }
    }

    pub fn get_challenge(&self) -> String {
        let guard = self.out.lock().unwrap();
        if let Some(ref out) = *guard {
            out.get_challenge()
        } else {
            "".to_string()
        }
    }

    pub fn post_slate(&self, to: &str, slate: &Slate) -> Result<()> {
        let guard = self.out.lock().unwrap();
        if let Some(ref out) = *guard {
            out.post_slate(to, slate)?;
            Ok(())
        } else {
            Err(Wallet713Error::PostSlate)?
        }
    }

    pub fn subscribe(&self) -> Result<()> {
        let guard = self.out.lock().unwrap();
        if let Some(ref out) = *guard {
            out.subscribe()?;
            Ok(())
        } else {
            Err(Wallet713Error::Subscribe)?
        }
    }

    pub fn unsubscribe(&self) -> Result<()> {
        let guard = self.out.lock().unwrap();
        if let Some(ref out) = *guard {
            out.unsubscribe()?;
            Ok(())
        } else {
            Err(Wallet713Error::Unsubscribe)?
        }
    }

    pub fn get_listening_address(&self) -> Result<String> {
        let guard = self.out.lock().unwrap();
        if let Some(ref out) = *guard {
            Ok(out.public_key.clone())
        } else {
            Err(Wallet713Error::ClosedListener)?
        }
    }

    pub fn start(&mut self, domain: &str, port: u16, private_key: &str, handler: Box<GrinboxClientHandler + Send>) -> Result<()> {
        let key = SecretKey::from_hex(private_key)?;
        let public_key = public_key_from_secret_key(&key).to_base58_check(BASE58_CHECK_VERSION_GRIN_TX.to_vec());
        let private_key = private_key.to_string();
        self.domain = domain.to_string();
        self.port = port;
        let uri = format!("ws://{}:{}", domain, port);
        let out = self.out.clone();
        let out2 = self.out.clone();
        let domain = domain.to_string();
        std::thread::spawn(move || {
            if let Err(_) = connect(uri, move |sender| {
                cli_message!("connected to grinbox");
                let handler = handler.clone();
                let client = GrinboxClientOut {
                    domain: domain.clone(),
                    port,
                    sender,
                    public_key: public_key.clone(),
                    private_key: private_key.clone(),
                    challenge: "".to_string(),
                };

                let mut guard = out.lock().unwrap();
                *guard = Some(client);

                let out = out.clone();
                move |msg| {
                    let mut guard = out.lock().unwrap();
                    if let Some(ref mut client) = *guard {
                        client.process_incoming(msg, Some(handler.clone()), true).expect("failed processing incoming message!");
                    }
                    Ok(())
                }
            }) {
                cli_message!("{}: could not connect to grinbox!", "ERROR".bright_red());
            };
            cli_message!("{}: grinbox listener stopped.", "WARNING".bright_yellow());
            let mut guard = out2.lock().unwrap();
            *guard = None;
        });
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        if let Some(ref out) = *self.out.lock().unwrap() {
            out.sender.close(ws::CloseCode::Normal)?;
        }
        Ok(())
    }
}

pub trait GrinboxClientHandler: GrinboxClientHandlerWithBoxClone {
    fn on_response(&self, response: &ProtocolResponse, out: &GrinboxClientOut);
    fn on_close(&self, reason: &str);
}

pub trait GrinboxClientHandlerWithBoxClone {
    fn clone_box(&self) -> Box<GrinboxClientHandler + Send>;
}

impl<T> GrinboxClientHandlerWithBoxClone for T where T: 'static + GrinboxClientHandler + Clone + Send {
    fn clone_box(&self) -> Box<GrinboxClientHandler + Send> {
        Box::new(self.clone())
    }
}

impl Clone for Box<GrinboxClientHandler + Send> {
    fn clone(&self) -> Box<GrinboxClientHandler + Send> {
        self.clone_box()
    }
}
