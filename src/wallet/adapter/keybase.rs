/// Keybase 'plugin' implementation

use failure::Error;
use grin_api::client;
use grin_api::Error as APIError;
use serde::Serialize;
use serde_json::Value;
use crate::api::listener::ListenerInterface;
use crate::broker::{GrinboxPublisher, Publisher};
use crate::contacts::{Address, GrinboxAddress};
use crate::common::{Arc, Keychain, Mutex};
use crate::wallet::types::{NodeClient, VersionedSlate, WalletBackend};
use crate::wallet::{Container, ErrorKind};
use super::Adapter;

#[derive(Clone)]
pub struct KeybaseAdapter<'a, W, C, K>
	where
		W: WalletBackend<C, K>,
		C: NodeClient,
		K: Keychain,
{
    container: &'a Arc<Mutex<Container<W, C, K>>>,
}

impl<'a, W, C, K> KeybaseAdapter<'a, W, C, K>
	where
		W: WalletBackend<C, K>,
		C: NodeClient,
		K: Keychain,
{
	/// Create
	pub fn new(container: &'a Arc<Mutex<Container<W, C, K>>>) -> Box<Self> {
        Box::new(Self {
            container,
        })
	}
}

impl<'a, W, C, K> Adapter for KeybaseAdapter<'a, W, C, K>
	where
		W: WalletBackend<C, K>,
		C: NodeClient,
		K: Keychain,
{
	fn supports_sync(&self) -> bool {
		false
	}

	fn send_tx_sync(&self, _dest: &str, _slate: &VersionedSlate) -> Result<VersionedSlate, Error> {
		unimplemented!();
	}

	fn send_tx_async(&self, dest: &str, slate: &VersionedSlate) -> Result<(), Error> {
		let address = GrinboxAddress::from_str(dest)?;
		let c = self.container.lock();
		c.listener(ListenerInterface::Keybase)?
			.publish(slate, &dest.to_owned())
	}
}
