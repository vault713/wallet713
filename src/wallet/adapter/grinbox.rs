/// Grinbox 'plugin' implementation

use failure::Error;
use grin_api::client;
use grin_api::Error as APIError;
use serde::Serialize;
use serde_json::Value;

use crate::broker::{GrinboxPublisher, Publisher};
use crate::contacts::{Address, GrinboxAddress};
use crate::common::{Keychain, MutexGuard};
use crate::wallet::types::{NodeClient, VersionedSlate, WalletBackend};
use crate::wallet::{Container, ErrorKind};
use super::Adapter;

#[derive(Clone)]
pub struct GrinboxAdapter<'a, W, C, K>
	where
		W: WalletBackend<C, K>,
		C: NodeClient,
		K: Keychain,
{
    container: &'a MutexGuard<'a, Container<W, C, K>>,
}

impl<'a, W, C, K> GrinboxAdapter<'a, W, C, K>
	where
		W: WalletBackend<C, K>,
		C: NodeClient,
		K: Keychain,
{
	/// Create
	pub fn new(container: &'a MutexGuard<Container<W, C, K>>) -> Self {
        Self {
            container,
        }
	}

	fn publisher(&self) -> Option<&GrinboxPublisher> {
		self.container.grinbox.as_ref().map(|g| &g.1)
	}
}

impl<'a, W, C, K> Adapter for GrinboxAdapter<'a, W, C, K>
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
		let publisher = self.publisher()
			.ok_or(ErrorKind::GrinboxNoListener)?;
        let address = GrinboxAddress::from_str(dest)?;
        publisher.post_slate(slate, &address)
	}

	fn receive_tx_async(&self, _params: &str) -> Result<VersionedSlate, Error> {
		unimplemented!();
	}
}
