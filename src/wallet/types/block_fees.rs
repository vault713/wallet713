use super::Identifier;

/// Fees in block to use for coinbase amount calculation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockFees {
    /// fees
    pub fees: u64,
    /// height
    pub height: u64,
    /// key id
    pub key_id: Option<Identifier>,
}

impl BlockFees {
    /// return key id
    pub fn key_id(&self) -> Option<Identifier> {
        self.key_id.clone()
    }
}
