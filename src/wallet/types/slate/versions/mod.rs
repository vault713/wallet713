pub mod v2;

use super::Slate;
use crate::wallet::error::ErrorKind;
use std::convert::TryFrom;
use v2::SlateV2;

pub const CURRENT_SLATE_VERSION: u16 = 2;
pub const GRIN_BLOCK_HEADER_VERSION: u16 = 2;

/// Existing versions of the slate
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum SlateVersion {
	/// V2 (most current)
	V2,
}

impl Default for SlateVersion {
	fn default() -> Self {
		SlateVersion::try_from(CURRENT_SLATE_VERSION).unwrap()
	}
}

impl TryFrom<u16> for SlateVersion {
	type Error = ErrorKind;

	fn try_from(value: u16) -> Result<Self, Self::Error> {
		match value {
			2 => Ok(SlateVersion::V2),
			v => Err(ErrorKind::SlateVersion(v)),
		}
	}
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
/// Versions are ordered newest to oldest so serde attempts to
/// deserialize newer versions first, then falls back to older versions.
pub enum VersionedSlate {
	/// Current (Grin 1.1.0 - 2.x (current))
	V2(SlateV2),
}

impl VersionedSlate {
	/// Return slate version
	pub fn version(&self) -> SlateVersion {
		match *self {
			VersionedSlate::V2(_) => SlateVersion::V2,
		}
	}

	/// convert this slate type to a specified older version
	pub fn into_version(slate: Slate, version: SlateVersion) -> VersionedSlate {
		match version {
			SlateVersion::V2 => VersionedSlate::V2(slate.into()),
		}
	}
}

impl From<VersionedSlate> for Slate {
	fn from(slate: VersionedSlate) -> Slate {
		match slate {
			VersionedSlate::V2(s) => {
				let s = SlateV2::from(s);
				Slate::from(s)
			}
		}
	}
}

impl From<&VersionedSlate> for Slate {
	fn from(slate: &VersionedSlate) -> Slate {
		match slate {
			VersionedSlate::V2(s) => Slate::from(s),
		}
	}
}
