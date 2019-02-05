/// a contained wallet info struct, so automated tests can parse wallet info
/// can add more fields here over time as needed
#[derive(Serialize, Eq, PartialEq, Deserialize, Debug, Clone)]
pub struct WalletInfo {
    /// height from which info was taken
    pub last_confirmed_height: u64,
    /// Minimum number of confirmations for an output to be treated as "spendable".
    pub minimum_confirmations: u64,
    /// total amount in the wallet
    pub total: u64,
    /// amount awaiting confirmation
    pub amount_awaiting_confirmation: u64,
    /// coinbases waiting for lock height
    pub amount_immature: u64,
    /// amount currently spendable
    pub amount_currently_spendable: u64,
    /// amount locked via previous transactions
    pub amount_locked: u64,
}
