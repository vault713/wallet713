use std::sync::{Arc, Mutex};
use std::thread;
use ws::{connect, Sender, Handler, Handshake, Message, CloseCode, Result as WsResult, ErrorKind as WsErrorKind, Error as WsError};
use ws::util::Token;
use colored::*;

use grin_core::libtx::slate::Slate;

use common::{ErrorKind, Result};
use common::crypto::{SecretKey, Signature, verify_signature, sign_challenge, Hex, EncryptedMessage};
use contacts::{Address, GrinboxAddress, DEFAULT_GRINBOX_PORT};

use super::types::{Publisher, Subscriber, SubscriptionHandler, CloseReason};
use super::protocol::{ProtocolResponse, ProtocolRequest};

const KEEPALIVE_TOKEN: Token = Token(1);
const KEEPALIVE_INTERVAL_MS: u64 = 30_000;

#[derive(Clone)]
pub struct GrinboxPublisher {
    address: GrinboxAddress,
    secret_key: SecretKey,
    protocol_unsecure: bool,
    use_encryption: bool,
}

impl GrinboxPublisher {
    pub fn new(address: &GrinboxAddress, secert_key: &SecretKey, protocol_unsecure: bool, use_encryption: bool) -> Result<Self> {
        Ok(Self {
            address: address.clone(),
            secret_key: secert_key.clone(),
            protocol_unsecure,
            use_encryption
        })
    }
}

impl Publisher for GrinboxPublisher {
    fn post_slate(&self, slate: &Slate, to: &Address) -> Result<()> {
        let broker = GrinboxBroker::new(self.protocol_unsecure, self.use_encryption)?;
        let to = GrinboxAddress::from_str(&to.to_string())?;
        broker.post_slate(slate, &to, &self.address, &self.secret_key)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct GrinboxSubscriber {
    address: GrinboxAddress,
    broker: GrinboxBroker,
    secret_key: SecretKey,
}

impl GrinboxSubscriber {
    pub fn new(address: &GrinboxAddress, secret_key: &SecretKey, protocol_unsecure: bool, use_encryption: bool) -> Result<Self> {
        Ok(Self {
            address: address.clone(),
            broker: GrinboxBroker::new(protocol_unsecure, use_encryption)?,
            secret_key: secret_key.clone(),
        })
    }
}

impl Subscriber for GrinboxSubscriber {
    fn start(&mut self, handler: Box<SubscriptionHandler + Send>) -> Result<()> {
        self.broker.subscribe(&self.address, &self.secret_key, handler)?;
        Ok(())
    }

    fn stop(&self) {
        self.broker.stop();
    }

    fn is_running(&self) -> bool {
        self.broker.is_running()
    }
}

#[derive(Clone)]
struct GrinboxBroker {
    inner: Arc<Mutex<Option<Sender>>>,
    protocol_unsecure: bool,
    use_encryption: bool,
}

struct ConnectionMetadata {
    retries: u32,
    connected_at_least_once: bool
}

impl ConnectionMetadata {
    pub fn new() -> Self {
        Self {
            retries: 0,
            connected_at_least_once: false,
        }
    }
}

impl GrinboxBroker {
    fn new(protocol_unsecure: bool, use_encryption: bool) -> Result<Self> {
        Ok(Self {
            inner: Arc::new(Mutex::new(None)),
            protocol_unsecure,
            use_encryption
        })
    }

    fn post_slate(&self, slate: &Slate, to: &GrinboxAddress, from: &GrinboxAddress, secret_key: &SecretKey) -> Result<()> {
        let url = {
            let to = to.clone();
            match self.protocol_unsecure {
                true => format!("ws://{}:{}", to.domain, to.port.unwrap_or(DEFAULT_GRINBOX_PORT)),
                false => format!("wss://{}:{}", to.domain, to.port.unwrap_or(DEFAULT_GRINBOX_PORT)),
            }
        };
        let pkey = to.public_key()?;
        let skey = secret_key.clone();
        connect(url, move |sender| {
            move |msg: Message| {
                let response = serde_json::from_str::<ProtocolResponse>(&msg.to_string()).expect("could not parse response!");
                match response {
                    ProtocolResponse::Challenge { str } => {
                        let slate_str = match self.use_encryption {
                            true => {
                                let message = EncryptedMessage::new(serde_json::to_string(&slate).unwrap(), &pkey, &skey).map_err(|_|
                                    WsError::new(WsErrorKind::Protocol, "could not encrypt slate!")
                                )?;
                                serde_json::to_string(&message).unwrap()
                            },
                            false => serde_json::to_string(&slate).unwrap(),
                        };

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
                        sender.close(CloseCode::Normal).is_ok();
                    },
                    _ => {}
                }
                Ok(())
            }
        })?;
        Ok(())
    }

    fn subscribe(&mut self, address: &GrinboxAddress, secret_key: &SecretKey, handler: Box<SubscriptionHandler + Send>) -> Result<()> {
        let handler = Arc::new(Mutex::new(handler));
        let url = {
            let cloned_address = address.clone();
            match self.protocol_unsecure {
                true => format!("ws://{}:{}", cloned_address.domain, cloned_address.port.unwrap_or(DEFAULT_GRINBOX_PORT)),
                false => format!("wss://{}:{}", cloned_address.domain, cloned_address.port.unwrap_or(DEFAULT_GRINBOX_PORT)),
            }
        };
        let secret_key = secret_key.clone();
        let cloned_address = address.clone();
        let cloned_inner = self.inner.clone();
        let cloned_handler = handler.clone();
        let use_encryption = self.use_encryption;
        thread::spawn(move || {
            let connection_meta_data = Arc::new(Mutex::new(ConnectionMetadata::new()));
            loop {
                let cloned_address = cloned_address.clone();
                let cloned_handler = cloned_handler.clone();
                let cloned_cloned_inner = cloned_inner.clone();
                let cloned_connection_meta_data = connection_meta_data.clone();
                let result = connect(url.clone(), move |sender| {
                    if let Ok(mut guard) = cloned_cloned_inner.lock() {
                        *guard = Some(sender.clone());
                    };

                    let client = GrinboxClient {
                        sender,
                        handler: cloned_handler.clone(),
                        challenge: None,
                        address: cloned_address.clone(),
                        secret_key,
                        use_encryption,
                        connection_meta_data: cloned_connection_meta_data.clone(),
                    };
                    client
                });

                let is_stopped = if let Ok(mut guard) = cloned_inner.lock() {
                    if guard.is_none() {
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };

                if is_stopped {
                    match result {
                        Err(_) => handler.lock().unwrap().on_close(CloseReason::Abnormal(ErrorKind::GrinboxWebsocketAbnormalTermination.into())),
                        _ => handler.lock().unwrap().on_close(CloseReason::Normal),
                    }
                    break;
                } else {
                    let mut guard = connection_meta_data.lock().unwrap();
                    if guard.retries == 0 && guard.connected_at_least_once {
                        handler.lock().unwrap().on_dropped();
                    }
                    let secs = std::cmp::min(32, 2u64.pow(guard.retries));
                    let duration = std::time::Duration::from_secs(secs);
                    std::thread::sleep(duration);
                    guard.retries += 1;
                }
            }
            let mut guard = cloned_inner.lock().unwrap();
            *guard = None;
        });
        Ok(())
    }

    fn stop(&self) {
        let mut guard = self.inner.lock().unwrap();
        if let Some(ref sender) = *guard {
            sender.close(CloseCode::Normal).is_ok();
        }
        *guard = None;
    }

    fn is_running(&self) -> bool {
        let guard = self.inner.lock().unwrap();
        guard.is_some()
    }
}

struct GrinboxClient {
    sender: Sender,
    handler: Arc<Mutex<Box<SubscriptionHandler + Send>>>,
    challenge: Option<String>,
    address: GrinboxAddress,
    secret_key: SecretKey,
    use_encryption: bool,
    connection_meta_data: Arc<Mutex<ConnectionMetadata>>,
}

impl GrinboxClient {
    fn generate_signature(challenge: &str, secret_key: &SecretKey) -> String {
        let signature = sign_challenge(challenge, secret_key).expect("could not sign challenge!");
        signature.to_hex()
    }

    fn subscribe(&self, challenge: &str) -> Result<()> {
        let signature = GrinboxClient::generate_signature(challenge, &self.secret_key);
        let request = ProtocolRequest::Subscribe { address: self.address.public_key.to_string(), signature };
        self.send(&request).expect("could not send subscribe request!");
        Ok(())
    }

    fn verify_slate_signature(&self, from: &str, str: &str, challenge: &str, signature: &str) -> Result<()> {
        let from = GrinboxAddress::from_str(from)?;
        let public_key = from.public_key()?;
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
}

impl Handler for GrinboxClient {
    fn on_open(&mut self, _shake: Handshake) -> WsResult<()> {
        let mut guard = self.connection_meta_data.lock().unwrap();

        if guard.connected_at_least_once {
            self.handler.lock().unwrap().on_reestablished();
        } else {
            self.handler.lock().unwrap().on_open();
            guard.connected_at_least_once = true;
        }

        guard.retries = 0;

        try!(self.sender.timeout(KEEPALIVE_INTERVAL_MS, KEEPALIVE_TOKEN));
        Ok(())
    }

    fn on_timeout(&mut self, event: Token) -> WsResult<()> {
        match event {
            KEEPALIVE_TOKEN => {
                self.sender.ping(vec![])?;
                self.sender.timeout(KEEPALIVE_INTERVAL_MS, KEEPALIVE_TOKEN)
            }
            _ => Err(WsError::new(WsErrorKind::Internal, "Invalid timeout token encountered!")),
        }
    }

    fn on_message(&mut self, msg: Message) -> WsResult<()> {
        let response = serde_json::from_str::<ProtocolResponse>(&msg.to_string()).map_err(|_| {
            WsError::new(WsErrorKind::Protocol, "could not parse response!")
        })?;
        match response {
            ProtocolResponse::Challenge { str } => {
                self.challenge = Some(str.clone());
                self.subscribe(&str).map_err(|_| {
                    WsError::new(WsErrorKind::Protocol, "error attempting to subscribe!")
                })?;
            },
            ProtocolResponse::Slate { from, str, challenge, signature } => {
                if let Ok(_) = self.verify_slate_signature(&from, &str, &challenge, &signature) {
                    let from = match GrinboxAddress::from_str(&from) {
                        Ok(x) => x,
                        Err(_) => {
                            cli_message!("could not parse address!");
                            return Ok(());
                        },
                    };

                    let mut slate: Slate = match self.use_encryption {
                        true => {
                            let encrypted_message: EncryptedMessage = match serde_json::from_str(&str) {
                                Ok(x) => x,
                                Err(_) => {
                                    cli_message!("could not parse encrypted message!");
                                    return Ok(());
                                },
                            };
                            let pkey = match from.public_key() {
                                Ok(x) => x,
                                Err(_) => {
                                    cli_message!("could not parse public key!");
                                    return Ok(());
                                },
                            };

                            let decrypted_message = match encrypted_message.decrypt(&pkey, &self.secret_key) {
                                Ok(x) => x,
                                Err(_) => {
                                    cli_message!("could not decrypt message!");
                                    return Ok(());
                                },
                            };

                            let slate: Slate = match serde_json::from_str(&decrypted_message) {
                                Ok(x) => x,
                                Err(_) => {
                                    cli_message!("could not parse slate!");
                                    return Ok(());
                                },
                            };

                            slate
                        },
                        false => match serde_json::from_str(&str) {
                            Ok(x) => x,
                            Err(_) => {
                                cli_message!("could not parse slate!");
                                return Ok(());
                            },
                        },
                    };

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
}
