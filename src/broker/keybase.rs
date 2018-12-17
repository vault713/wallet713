use std::process::{Command, Stdio};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::borrow::Borrow;
use std::iter::FromIterator;

use serde::Serialize;
use serde_json::{json, Value};
use grin_core::libtx::slate::Slate;

use common::{Error, Wallet713Error};
use contacts::{Address, KeybaseAddress};
use super::types::{Publisher, Subscriber, SubscriptionHandler, CloseReason};

pub const TOPIC_SLATE_NEW: &str = "grin_slate_new";
pub const TOPIC_WALLET713_SLATES: &str = "wallet713_grin_slate";
const TOPIC_SLATE_SIGNED: &str = "grin_slate_signed";
const DEFAULT_TTL: &str = "24h";
const SLEEP_DURATION: Duration = Duration::from_millis(5000);

#[derive(Clone)]
pub struct KeybasePublisher {}

impl KeybasePublisher {
    pub fn new() -> Result<Self, Error> {
        let _broker = KeybaseBroker::new()?;
        Ok(Self {})
    }
}

#[derive(Clone)]
pub struct KeybaseSubscriber {
    stop_signal: Arc<Mutex<bool>>
}

impl KeybaseSubscriber {
    pub fn new() -> Result<Self, Error> {
        let _broker = KeybaseBroker::new()?;
        Ok(Self {
            stop_signal: Arc::new(Mutex::new(true))
        })
    }
}

impl Publisher for KeybasePublisher {
    fn post_slate(&self, slate: &Slate, to: &Address) -> Result<(), Error> {
        let keybase_address = KeybaseAddress::from_str(&to.to_string()).unwrap();
        if let Some(topic) = keybase_address.topic {
            KeybaseBroker::send(&slate, &to.stripped(), &topic, DEFAULT_TTL)?;
        } else {
            KeybaseBroker::send(&slate, &to.stripped(), TOPIC_WALLET713_SLATES, DEFAULT_TTL)?;
        }
        Ok(())
    }
}

impl Subscriber for KeybaseSubscriber {
    fn start(&mut self, handler: Box<SubscriptionHandler + Send>) -> Result<(), Error> {
        if let Ok(mut guard) = self.stop_signal.lock() {
            *guard = false;
        }
        let mut subscribed = false;
        let mut dropped = false;
        let result: Result<(), Error> = loop {
            if *self.stop_signal.lock().unwrap() {
                break Ok(())
            };
            let result = KeybaseBroker::get_unread(HashSet::from_iter(vec![
                TOPIC_WALLET713_SLATES,
                TOPIC_SLATE_NEW,
                TOPIC_SLATE_SIGNED,
            ]));
            if let Ok(unread) = result {
                if !subscribed {
                    subscribed = true;
                    handler.on_open();
                }
                if dropped {
                    dropped = false;
                    handler.on_reestablished();
                }
                for (sender, topic, msg) in &unread {
                    let reply_topic = match topic.as_ref() {
                        TOPIC_SLATE_NEW => TOPIC_SLATE_SIGNED.to_string(),
                        _ => TOPIC_WALLET713_SLATES.to_string(),
                    };
                    let mut slate: Slate = serde_json::from_str(msg)?;
                    let address = KeybaseAddress {
                        username: sender.to_string(),
                        topic: Some(reply_topic),
                    };
                    handler.on_slate(address.borrow(), &mut slate);
                }
            } else {
                if !dropped {
                    dropped = true;
                    if subscribed {
                        handler.on_dropped();
                    } else {
                        break Err(Error::from(Wallet713Error::KeybaseNotFound));
                    }
                }
            }
            std::thread::sleep(SLEEP_DURATION);
        };
        match result {
            Err(e) => handler.on_close(CloseReason::Abnormal(e)),
            _ => handler.on_close(CloseReason::Normal),
        }
        Ok(())
    }

    fn stop(&self) {
        let mut guard = self.stop_signal.lock().unwrap();
        *guard = true;
    }

    fn is_running(&self) -> bool {
        let guard = self.stop_signal.lock().unwrap();
        !*guard
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

    pub fn read_from_channel(channel: &str, topic: &str) -> Result<Vec<(String, String, String)>, Error> {
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
        let mut unread: Vec<(String, String, String)> = Vec::new();
        let messages = response["result"]["messages"].as_array();
        if let Some(messages) = messages {
            for msg in messages.iter() {
                if (msg["msg"]["content"]["type"] == "text") && (msg["msg"]["unread"] == true) {
                    let message = msg["msg"]["content"]["text"]["body"].as_str().unwrap_or("");
                    let sender: &str = msg["msg"]["sender"]["username"].as_str().unwrap_or("");
                    if !message.is_empty() && !sender.is_empty() {
                        unread.push((sender.to_string(), topic.to_string(), message.to_string()));
                    }
                }
            }
        }
        Ok(unread)
    }

    pub fn get_unread(topics: HashSet<&str>) -> Result<Vec<(String, String, String)>, Error> {
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
                let topic = msg["channel"]["topic_name"].as_str().unwrap();
                if (msg["unread"] == true) && topics.contains(topic) {
                    let channel = msg["channel"]["name"].as_str().unwrap();
                    channels.insert((channel.to_string(), topic));
                }
            }
        }

        let mut unread: Vec<(String, String, String)> = Vec::new();
        for (channel, topic) in channels.iter() {
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