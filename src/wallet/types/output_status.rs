use std::fmt;

/// Status of an output that's being tracked by the wallet. Can either be
/// unconfirmed, spent, unspent, or locked (when it's been used to generate
/// a transaction but we don't have confirmation that the transaction was
/// broadcasted or mined).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
pub enum OutputStatus {
    /// Unconfirmed
    Unconfirmed,
    /// Unspent
    Unspent,
    /// Locked
    Locked,
    /// Spent
    Spent,
}

impl fmt::Display for OutputStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            OutputStatus::Unconfirmed => write!(f, "Unconfirmed"),
            OutputStatus::Unspent => write!(f, "Unspent"),
            OutputStatus::Locked => write!(f, "Locked"),
            OutputStatus::Spent => write!(f, "Spent"),
        }
    }
}
