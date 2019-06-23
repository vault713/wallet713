pub mod v0;
pub mod v1;
pub mod v2;

pub const CURRENT_SLATE_VERSION: u16 = 2;
pub const GRIN_BLOCK_HEADER_VERSION: u16 = 2;

/// Existing versions of the slate
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum SlateVersion {
	/// V2 (most current)
	V2,
}