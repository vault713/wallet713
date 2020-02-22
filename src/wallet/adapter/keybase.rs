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

/// Keybase 'plugin' implementation
use super::Adapter;
use crate::api::listener::ListenerInterface;
use crate::common::{Arc, Keychain, Mutex};
use crate::wallet::types::{NodeClient, VersionedSlate, WalletBackend};
use crate::wallet::Container;
use failure::Error;

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
		Box::new(Self { container })
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
		let c = self.container.lock();
		c.listener(ListenerInterface::Keybase)?
			.publish(slate, &dest.to_owned())
	}
}
