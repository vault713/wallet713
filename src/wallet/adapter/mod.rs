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

mod grinbox;
mod http;
mod keybase;
//mod null;

pub use self::grinbox::GrinboxAdapter;
pub use self::http::HTTPAdapter;
pub use self::keybase::KeybaseAdapter;

use failure::Error;
use super::types::VersionedSlate;

/// Encapsulate wallet to wallet communication functions
pub trait Adapter {
	/// Whether this adapter supports sync mode
	fn supports_sync(&self) -> bool;

	/// Send a transaction slate to another listening wallet and return result
	fn send_tx_sync(&self, addr: &str, slate: &VersionedSlate) -> Result<VersionedSlate, Error>;

	/// Send a transaction asynchronously (result will be returned via the listener)
	fn send_tx_async(&self, addr: &str, slate: &VersionedSlate) -> Result<(), Error>;
}
