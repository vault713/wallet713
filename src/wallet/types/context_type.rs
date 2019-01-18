use std::fmt;

/// Types of transactions that can be contained within a TXLog entry
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum ContextType {
    /// A transaction context
    Tx,
}

impl fmt::Display for ContextType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ContextType::Tx => write!(f, "Tx"),
        }
    }
}