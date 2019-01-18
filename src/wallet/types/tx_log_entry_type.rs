use std::fmt;

/// Types of transactions that can be contained within a TXLog entry
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum TxLogEntryType {
    /// A coinbase transaction becomes confirmed
    ConfirmedCoinbase,
    /// Outputs created when a transaction is received
    TxReceived,
    /// Inputs locked + change outputs when a transaction is created
    TxSent,
    /// Received transaction that was rolled back by user
    TxReceivedCancelled,
    /// Sent transaction that was rolled back by user
    TxSentCancelled,
}

impl fmt::Display for TxLogEntryType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TxLogEntryType::ConfirmedCoinbase => write!(f, "Confirmed \nCoinbase"),
            TxLogEntryType::TxReceived => write!(f, "Received Tx"),
            TxLogEntryType::TxSent => write!(f, "Sent Tx"),
            TxLogEntryType::TxReceivedCancelled => write!(f, "Received Tx\n- Cancelled"),
            TxLogEntryType::TxSentCancelled => write!(f, "Send Tx\n- Cancelled"),
        }
    }
}