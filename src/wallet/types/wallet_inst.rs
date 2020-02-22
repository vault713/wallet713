// Copyright 2018 The Grin Developers
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

use super::{Keychain, NodeClient, WalletBackend};

pub trait WalletInst<C, K>: WalletBackend<C, K> + Send + Sync + 'static
where
	C: NodeClient,
	K: Keychain,
{
}
impl<T, C, K> WalletInst<C, K> for T
where
	T: WalletBackend<C, K> + Send + Sync + 'static,
	C: NodeClient,
	K: Keychain,
{
}
