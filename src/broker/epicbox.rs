// Copyright 2019 The vault713 Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::protocol::{ProtocolRequest, ProtocolResponse};
use super::types::{CloseReason, Controller, Publisher, Subscriber, SubscriptionHandler};
use crate::cli_message;
use crate::common::crypto::{sign_challenge, Hex, SecretKey};
use crate::common::message::EncryptedMessage;
use crate::common::{Arc, ErrorKind, Keychain, Mutex, Result};
use crate::contacts::{Address, EpicboxAddress, DEFAULT_EPICBOX_PORT};
use crate::wallet::types::{NodeClient, TxProof, VersionedSlate, WalletBackend};
use colored::Colorize;
use log::error;
use ws::util::Token;
use ws::{
	connect, CloseCode, Error as WsError, ErrorKind as WsErrorKind, Handler, Handshake, Message,
	Result as WsResult, Sender,
};

const KEEPALIVE_TOKEN: Token = Token(1);
const KEEPALIVE_INTERVAL_MS: u64 = 30_000;

#[derive(Clone)]
pub struct EpicboxPublisher {
	address: EpicboxAddress,
	broker: EpicboxBroker,
	secret_key: SecretKey,
}

impl EpicboxPublisher {
	pub fn new(
		address: &EpicboxAddress,
		secret_key: &SecretKey,
		protocol_unsecure: bool,
	) -> Result<Self> {
		Ok(Self {
			address: address.clone(),
			broker: EpicboxBroker::new(protocol_unsecure)?,
			secret_key: secret_key.clone(),
		})
	}
}

impl Publisher for EpicboxPublisher {
	fn post_slate(&self, slate: &VersionedSlate, to: &dyn Address) -> Result<()> {


		let to = EpicboxAddress::from_str(&to.to_string())?;
		self.broker
			.post_slate(slate, &to, &self.address, &self.secret_key)?;
		Ok(())
	}
}

#[derive(Clone)]
pub struct EpicboxSubscriber {
	address: EpicboxAddress,
	broker: EpicboxBroker,
	secret_key: SecretKey,
}

impl EpicboxSubscriber {
	pub fn new(publisher: &EpicboxPublisher) -> Result<Self> {
		Ok(Self {
			address: publisher.address.clone(),
			broker: publisher.broker.clone(),
			secret_key: publisher.secret_key.clone(),
		})
	}
}

impl Subscriber for EpicboxSubscriber {
	fn start<W, C, K, P>(&mut self, handler: Controller<W, C, K, P>) -> Result<()>
	where
		W: WalletBackend<C, K>,
		C: NodeClient,
		K: Keychain,
		P: Publisher,
	{
		self.broker
			.subscribe(&self.address, &self.secret_key, handler)?;
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
struct EpicboxBroker {
	inner: Arc<Mutex<Option<Sender>>>,
	protocol_unsecure: bool,
}

struct ConnectionMetadata {
	retries: u32,
	connected_at_least_once: bool,
}

impl ConnectionMetadata {
	pub fn new() -> Self {
		Self {
			retries: 0,
			connected_at_least_once: false,
		}
	}
}

impl EpicboxBroker {
	fn new(protocol_unsecure: bool) -> Result<Self> {
		Ok(Self {
			inner: Arc::new(Mutex::new(None)),
			protocol_unsecure,
		})
	}

	fn post_slate(
		&self,
		slate: &VersionedSlate,
		to: &EpicboxAddress,
		from: &EpicboxAddress,
		secret_key: &SecretKey,
	) -> Result<()> {
		if !self.is_running() {
			return Err(ErrorKind::ClosedListener("epicbox".to_string()).into());
		}

		let pkey = to.public_key()?;
		let skey = secret_key.clone();
		let message = EncryptedMessage::new(serde_json::to_string(&slate)?, &to, &pkey, &skey)
			.map_err(|_| WsError::new(WsErrorKind::Protocol, "could not encrypt slate!"))?;
		let message_ser = serde_json::to_string(&message)?;

		let mut challenge = String::new();
		challenge.push_str(&message_ser);

		let signature = sign_challenge(&challenge, secret_key)?.to_hex();
		let request = ProtocolRequest::PostSlate {
			from: from.stripped(),
			to: to.stripped(),
			str: message_ser,
			signature,
		};

		cli_message!("####################### post slate ###################### {}", serde_json::to_string(&request).unwrap());

		if let Some(ref sender) = *self.inner.lock() {
			sender
				.send(serde_json::to_string(&request).unwrap())
				.map_err(|_| ErrorKind::GenericError("failed posting slate!".to_string()).into())
		} else {
			Err(ErrorKind::GenericError("failed posting slate!".to_string()).into())
		}
	}

	fn subscribe<W, C, K, P>(
		&mut self,
		address: &EpicboxAddress,
		secret_key: &SecretKey,
		handler: Controller<W, C, K, P>,
	) -> Result<()>
	where
		W: WalletBackend<C, K>,
		C: NodeClient,
		K: Keychain,
		P: Publisher,
	{
		let handler = Arc::new(Mutex::new(handler));
		let url = {
			let cloned_address = address.clone();
			match self.protocol_unsecure {
				true => format!(
					"ws://{}:{}",
					cloned_address.domain,
					cloned_address.port.unwrap_or(DEFAULT_EPICBOX_PORT)
				),
				false => format!(
					"wss://{}:{}",
					cloned_address.domain,
					cloned_address.port.unwrap_or(DEFAULT_EPICBOX_PORT)
				),
			}
		};
		let cloned_address = address.clone();
		let cloned_inner = self.inner.clone();
		let cloned_handler = handler.clone();
		let connection_meta_data = Arc::new(Mutex::new(ConnectionMetadata::new()));
		loop {
			let cloned_address = cloned_address.clone();
			let cloned_handler = cloned_handler.clone();
			let cloned_cloned_inner = cloned_inner.clone();
			let cloned_connection_meta_data = connection_meta_data.clone();
			let result = connect(url.clone(), |sender| {
				{
					let mut guard = cloned_cloned_inner.lock();
					*guard = Some(sender.clone());
				}

				let client = EpicboxClient {
					sender,
					handler: cloned_handler.clone(),
					challenge: None,
					address: cloned_address.clone(),
					secret_key: secret_key.clone(),
					connection_meta_data: cloned_connection_meta_data.clone(),
				};
				client
			});

			let is_stopped = cloned_inner.lock().is_none();

			if is_stopped {
				match result {
					Err(_) => handler.lock().on_close(CloseReason::Abnormal(
						ErrorKind::EpicboxWebsocketAbnormalTermination.into(),
					)),
					_ => handler.lock().on_close(CloseReason::Normal),
				}
				break;
			} else {
				let mut guard = connection_meta_data.lock();
				if guard.retries == 0 && guard.connected_at_least_once {
					handler.lock().on_dropped();
				}
				let secs = std::cmp::min(32, 2u64.pow(guard.retries));
				let duration = std::time::Duration::from_secs(secs);
				std::thread::sleep(duration);
				guard.retries += 1;
			}
		}
		let mut guard = cloned_inner.lock();
		*guard = None;
		Ok(())
	}

	fn stop(&self) {
		let mut guard = self.inner.lock();
		if let Some(ref sender) = *guard {
			let _ = sender.close(CloseCode::Normal);
		}
		*guard = None;
	}

	fn is_running(&self) -> bool {
		let guard = self.inner.lock();
		guard.is_some()
	}
}

struct EpicboxClient<W, C, K, P>
where
	W: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
	P: Publisher,
{
	sender: Sender,
	handler: Arc<Mutex<Controller<W, C, K, P>>>,
	challenge: Option<String>,
	address: EpicboxAddress,
	secret_key: SecretKey,
	connection_meta_data: Arc<Mutex<ConnectionMetadata>>,
}

impl<W, C, K, P> EpicboxClient<W, C, K, P>
where
	W: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
	P: Publisher,
{
	fn subscribe(&self, challenge: &str) -> Result<()> {
		let signature = sign_challenge(&challenge, &self.secret_key)?.to_hex();
		let request = ProtocolRequest::Subscribe {
			address: self.address.public_key.to_string(),
			signature,
		};
		self.send(&request)
			.expect("could not send subscribe request!");
		Ok(())
	}

	fn send(&self, request: &ProtocolRequest) -> Result<()> {



		let request = serde_json::to_string(&request).unwrap();





		self.sender.send(request)?;
		Ok(())
	}
}

impl<W, C, K, P> Handler for EpicboxClient<W, C, K, P>
where
	W: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
	P: Publisher,
{
	fn on_open(&mut self, _shake: Handshake) -> WsResult<()> {
		let mut guard = self.connection_meta_data.lock();

		if guard.connected_at_least_once {
			self.handler.lock().on_reestablished();
		} else {
			self.handler.lock().on_open();
			guard.connected_at_least_once = true;
		}

		guard.retries = 0;

		self.sender
			.timeout(KEEPALIVE_INTERVAL_MS, KEEPALIVE_TOKEN)?;
		Ok(())
	}

	fn on_timeout(&mut self, event: Token) -> WsResult<()> {
		match event {
			KEEPALIVE_TOKEN => {
				self.sender.ping(vec![])?;
				self.sender.timeout(KEEPALIVE_INTERVAL_MS, KEEPALIVE_TOKEN)
			}
			_ => Err(WsError::new(
				WsErrorKind::Internal,
				"Invalid timeout token encountered!",
			)),
		}
	}

	fn on_message(&mut self, msg: Message) -> WsResult<()> {
		let response = match serde_json::from_str::<ProtocolResponse>(&msg.to_string()) {
			Ok(x) => x,
			Err(_) => {
				cli_message!("{} Could not parse response", "ERROR:".bright_red());
				return Ok(());
			}
		};




		match response {
			ProtocolResponse::Challenge { str } => {
				self.challenge = Some(str.clone());
				self.subscribe(&str).map_err(|_| {
					WsError::new(WsErrorKind::Protocol, "error attempting to subscribe!")
				})?;
			}
			ProtocolResponse::Slate {
				from,
				str,
				challenge,
				signature,
			} => {
				let (slate, mut tx_proof) = match TxProof::from_response(
					from,
					str,
					challenge,
					signature,
					&self.secret_key,
					Some(&self.address),
				) {
					Ok(x) => x,
					Err(e) => {
						cli_message!("{} {}", "ERROR:".bright_red(), e);
						return Ok(());
					}
				};

				let address = tx_proof.address.clone();
				self.handler
					.lock()
					.on_slate(&address, &slate, Some(&mut tx_proof));
			}
			ProtocolResponse::Error {
				kind: _,
				description: _,
			} => {
				cli_message!("{} {}", "ERROR:".bright_red(), response);
			}
			_ => {}
		}
		Ok(())
	}

	fn on_error(&mut self, err: WsError) {
		// Ignore connection reset errors by default
		if let WsErrorKind::Io(ref err) = err.kind {
			if let Some(104) = err.raw_os_error() {
				return;
			}
		}

		error!("{:?}", err);
	}
}
