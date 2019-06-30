pub mod foreign;
pub mod owner;
pub mod types;

pub use self::foreign::Foreign;
pub use self::owner::Owner;
pub use self::types::*;

use failure::Error;
use crate::wallet::types::{NodeVersionInfo, Slate, CURRENT_SLATE_VERSION, GRIN_BLOCK_HEADER_VERSION};
use crate::wallet::ErrorKind;
use self::foreign::ForeignCheckMiddlewareFn;

pub fn check_middleware(
	name: ForeignCheckMiddlewareFn,
	node_version_info: Option<NodeVersionInfo>,
	slate: Option<&Slate>,
) -> Result<(), Error> {
	match name {
		// allow coinbases to be built regardless
		ForeignCheckMiddlewareFn::BuildCoinbase => Ok(()),
		_ => {
			/*let mut bhv = 1;
			if let Some(n) = node_version_info {
				bhv = n.block_header_version;
			}
			if let Some(s) = slate {
				if s.version_info.version < CURRENT_SLATE_VERSION
					|| (bhv == 1 && s.version_info.block_header_version != 1)
					|| (bhv > 1 && s.version_info.block_header_version < GRIN_BLOCK_HEADER_VERSION)
				{
					return Err(ErrorKind::Compatibility.into());
				}
			}*/
			Ok(())
		}
	}
}
