/// Response to build a coinbase output.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CbData {
    /// Output
    pub output: String,
    /// Kernel
    pub kernel: String,
    /// Key Id
    pub key_id: String,
}