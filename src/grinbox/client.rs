use std::sync::{Arc, Mutex};
use std::str::FromStr;

use ws::{connect, Sender};

use grin_core::libtx::slate::Slate;
use common::{Wallet713Error, Result};
use common::crypto::{SecretKey, PublicKey, Signature, public_key_from_secret_key, verify_signature, sign_challenge, Hex, Base58, BASE58_CHECK_VERSION_GRIN_TX};
use super::protocol::{ProtocolRequest, ProtocolResponse};

pub struct GrinboxClientOut {
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
        let mut to = to.to_string();
        if to == "self" {
            to = self.public_key.clone();
        }

        let str = serde_json::to_string(&slate).unwrap();
        let mut challenge = String::new();
        challenge.push_str(&str);
        challenge.push_str(&self.challenge);
        let signature = self.generate_signature(&challenge);
        self.send(&ProtocolRequest::PostSlate {
            from: self.public_key.to_string(),
            to,
            str,
            signature
        }).expect("could not send slate!");
        Ok(())
    }

    pub fn get_challenge(&self) -> String {
        self.challenge.clone()
    }

    fn generate_signature(&self, challenge: &str) -> String {
        let secret_key = SecretKey::from_str(&self.private_key).expect("could not construct secret key!");
        let signature = sign_challenge(challenge, &secret_key).expect("could not sign challenge!");
        signature.to_hex()
    }

    fn verify_slate_signature(&self, from: &str, str: &str, challenge: &str, signature: &str) -> Result<()> {
        let public_key = PublicKey::from_base58_check(from, 2)?;
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

    fn process_incoming(&mut self, msg: ws::Message, handler: Box<GrinboxClientHandler + Send>) -> Result<()> {
        let response = serde_json::from_str::<ProtocolResponse>(&msg.to_string())?;
        match response {
            ProtocolResponse::Challenge { str } => {
                cli_message!("subscribing to [{}]", self.public_key.bright_green());
                self.challenge = str;
                self.subscribe()?;
            },
            ProtocolResponse::Slate { from, str, challenge, signature } => {
                if let Ok(_) = self.verify_slate_signature(&from, &str, &challenge, &signature) {
                    handler.on_response(&ProtocolResponse::Slate { from, str, challenge, signature }, &self);
                } else {
                    cli_message!("{}: received slate with invalid signature!", "ERROR".bright_red());
                }
            },
            _ => handler.on_response(&response, &self),
        }

        Ok(())
    }
}

pub struct GrinboxClient {
    out: Arc<Mutex<Option<GrinboxClientOut>>>,
}

impl GrinboxClient {
    pub fn new() -> Self {
        GrinboxClient {
            out: Arc::new(Mutex::new(None)),
        }
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

    pub fn start(&mut self, uri: &str, private_key: &str, handler: Box<GrinboxClientHandler + Send>) -> Result<()> {
        let key = SecretKey::from_hex(private_key)?;
        let public_key = public_key_from_secret_key(&key).to_base58_check(BASE58_CHECK_VERSION_GRIN_TX.to_vec());
        let private_key = private_key.to_string();
        let uri = uri.to_string();
        let out = self.out.clone();
        let out2 = self.out.clone();
        std::thread::spawn(move || {
            if let Err(_) = connect(uri, move |sender| {
                cli_message!("connected to grinbox");
                let handler = handler.clone();
                let client = GrinboxClientOut {
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
                        client.process_incoming(msg, handler.clone()).expect("failed processing incoming message!");
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
