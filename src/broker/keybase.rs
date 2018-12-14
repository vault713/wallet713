use std::process::{Command, Stdio};
use std::collections::{HashSet, HashMap};
use std::time::Duration;
use std::borrow::Borrow;

use serde::Serialize;
use serde_json::{json, Value};
use grin_core::libtx::slate::Slate;

use common::{Error, Wallet713Error};
use contacts::{Address, KeybaseAddress};
use super::types::{Publisher, Subscriber, SubscriptionHandler};

const TOPIC_WALLET713_SLATES: &str = "wallet713_grin_slate";
const DEFAULT_TTL: &str = "24h";
const SLEEP_DURATION: Duration = Duration::from_millis(5000);

pub struct KeybasePublisher {}

impl KeybasePublisher {
    pub fn new() -> Result<Self, Error> {
        let _broker = KeybaseBroker::new()?;
        Ok(Self {})
    }
}

pub struct KeybaseSubscriber {}

impl KeybaseSubscriber {
    pub fn new() -> Result<Self, Error> {
        let _broker = KeybaseBroker::new()?;
        Ok(Self {})
    }
}

impl Publisher for KeybasePublisher {
    fn post_slate(&self, slate: &Slate, to: &Address) -> Result<(), Error> {
        KeybaseBroker::send(&slate, &to.stripped(), TOPIC_WALLET713_SLATES, DEFAULT_TTL)?;
        Ok(())
    }
}

impl Subscriber for KeybaseSubscriber {
    fn subscribe(&self, handler: Box<SubscriptionHandler + Send>) -> Result<(), Error> {
        let mut subscribed = false;
        loop {
            let unread = KeybaseBroker::get_unread(TOPIC_WALLET713_SLATES).expect("could not retrieve messages!");
            if !subscribed {
                subscribed = true;
                handler.on_open();
            }
            for (sender, msg) in &unread {
                let mut slate: Slate = serde_json::from_str(msg)?;
                let address = KeybaseAddress::from_str(&sender)?;
                handler.on_slate(address.borrow(), &mut slate);
            }
            std::thread::sleep(SLEEP_DURATION);
        }
        handler.on_close();
        Ok(())
    }

    fn unsubscribe(&self) -> Result<(), Error> {
        Ok(())
    }
}

struct KeybaseBroker {}

impl KeybaseBroker {
    pub fn new() -> Result<Self, Error> {
        let mut proc = if cfg!(target_os = "windows") {
            Command::new("where")
        } else {
            Command::new("which")
        };

        let status = proc.arg("keybase")
            .stdout(Stdio::null())
            .status()?;

        if status.success() {
            Ok(Self{})
        } else {
            Err(Wallet713Error::KeybaseNotFound)?
        }
    }

    pub fn api_send(payload: &str) -> Result<Value, Error> {
        let mut proc = Command::new("keybase");
        proc.args(&["chat", "api", "-m", &payload]);
        let output = proc.output().expect("No output").stdout;
        let response = std::str::from_utf8(&output)?;
        let response: Value = serde_json::from_str(response)?;
        Ok(response)
    }

    pub fn read_from_channel(channel: &str, topic: &str) -> Result<Vec<(String, String)>, Error> {
        let payload = json!({
            "method": "read",
            "params": {
                "options": {
                    "channel": {
                        "name": channel,
                        "topic_type": "dev",
                        "topic_name": topic
                    },
                    "unread_only": true,
                    "peek": false
                },
            }
        });
        let payload = serde_json::to_string(&payload)?;
        let response = KeybaseBroker::api_send(&payload)?;
        let mut unread: Vec<(String, String)> = Vec::new();
        let messages = response["result"]["messages"].as_array();
        if let Some(messages) = messages {
            for msg in messages.iter() {
                if (msg["msg"]["content"]["type"] == "text") && (msg["msg"]["unread"] == true) {
                    let message = msg["msg"]["content"]["text"]["body"].as_str().unwrap_or("");
                    let sender: &str = msg["msg"]["sender"]["username"].as_str().unwrap_or("");
                    if !message.is_empty() && !sender.is_empty() {
                        unread.push((sender.to_owned(), message.to_owned()));
                    }
                }
            }
        }
        Ok(unread)
    }

    pub fn get_unread(topic: &str) -> Result<Vec<(String, String)>, Error> {
        let payload = json!({
		    "method": "list",
		    "params": {
			    "options": {
					"topic_type": "dev",
				},
			}
		});
        let payload = serde_json::to_string(&payload)?;
        let response = KeybaseBroker::api_send(&payload)?;

        let mut channels = HashSet::new();
        let messages = response["result"]["conversations"].as_array();
        if let Some(messages) = messages {
            for msg in messages.iter() {
                if (msg["unread"] == true) && (msg["channel"]["topic_name"] == topic) {
                    let channel = msg["channel"]["name"].as_str().unwrap();
                    channels.insert(channel.to_string());
                }
            }
        }

        let mut unread: Vec<(String, String)> = Vec::new();
        for channel in channels.iter() {
            let mut messages = KeybaseBroker::read_from_channel(channel, topic)?;
            unread.append(&mut messages);
        }
        Ok(unread)
    }

    pub fn send<T: Serialize>(message: &T, channel: &str, topic: &str, ttl: &str) -> Result<(), Error> {
        let payload = json!({
    		"method": "send",
	    	"params": {
		    	"options": {
			    	"channel": {
						"name": channel,
						"topic_name": topic,
						"topic_type": "dev"
					},
                    "message": {
                        "body": serde_json::to_string(&message)?
                    },
                    "exploding_lifetime": ttl
                }
            }
        });

        let payload = serde_json::to_string(&payload)?;
        let response = KeybaseBroker::api_send(&payload)?;
        match response["result"]["message"].as_str() {
            Some("message sent") => Ok(()),
            _ => Err(Wallet713Error::KeybaseMessageSendError)?,
        }
    }
}