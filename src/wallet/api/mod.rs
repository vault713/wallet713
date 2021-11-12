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

pub mod foreign;
pub mod owner;
pub mod types;

pub use self::foreign::Foreign;
use self::foreign::ForeignCheckMiddlewareFn;
pub use self::owner::Owner;
pub use self::types::*;
use crate::wallet::types::{
	NodeVersionInfo, Slate, CURRENT_SLATE_VERSION, EPIC_BLOCK_HEADER_VERSION,
};
use crate::wallet::ErrorKind;
use failure::Error;

pub fn check_middleware(
	name: ForeignCheckMiddlewareFn,
	node_version_info: Option<NodeVersionInfo>,
	slate: Option<&Slate>,
) -> Result<(), Error> {
	match name {
		// allow coinbases to be built regardless
		ForeignCheckMiddlewareFn::BuildCoinbase => Ok(()),
		_ => {
			let mut bhv = 1;
			if let Some(n) = node_version_info {
				bhv = n.block_header_version;
			}
			if let Some(s) = slate {
				if s.version_info.version < CURRENT_SLATE_VERSION
					|| (bhv == 1 && s.version_info.block_header_version != 1)
					|| (bhv > 1 && s.version_info.block_header_version < EPIC_BLOCK_HEADER_VERSION)
				{
					return Err(ErrorKind::Compatibility.into());
				}
			}
			Ok(())
		}
	}
}
