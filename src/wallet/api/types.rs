use crate::wallet::types::SlateVersion;

/// Version request result
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VersionInfo {
	/// API version
	pub foreign_api_version: u16,
	/// Slate version
	pub supported_slate_versions: Vec<SlateVersion>,
}