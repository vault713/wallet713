use colored::*;

#[derive(Debug)]
pub enum Error {
    Generic { description: String },
    GrinLibWallet { e: grin_wallet::libwallet::Error },
    GrinWallet { e: grin_wallet::Error },
    Toml,
    IO { e: std::io::Error },
    Cli { e: clap::Error },
    Secp { e: secp256k1::Error },
    WebSocket { e: ws::Error },
    Json { e: serde_json::error::Error },
    InvalidBase58Character(char, usize),
    InvalidBase58Length,
    InvalidBase58Checksum,
}

impl Error {
    pub fn generic(description: &str) -> Self {
        Error::Generic { description: description.to_owned() }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Generic { description } => write!(f, "{}: {}", "ERROR".bright_red(), description),
            Error::Cli { e } => write!(f, "{}", e),
            Error::GrinLibWallet { e } => write!(f, "{}: {}", "ERROR".bright_red(), e.kind()),
            Error::GrinWallet { e } => write!(f, "{}: {}", "ERROR".bright_red(), e.kind()),
            Error::Secp { e } => write!(f, "{}: {}", "ERROR".bright_red(), e),
            Error::WebSocket { e } => write!(f, "{}: {}", "ERROR".bright_red(), e),
            Error::Json { e } => write!(f, "{}: {}", "ERROR".bright_red(), e),
            _ => write!(f, "{}: {:?}", "ERROR".bright_red(), self)
        }
    }
}

impl From<toml::de::Error> for Error {
    fn from(_: toml::de::Error) -> Self {
        Error::Toml
    }
}

impl From<toml::ser::Error> for Error {
    fn from(_: toml::ser::Error) -> Self {
        Error::Toml
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IO { e }
    }
}

impl From<grin_wallet::libwallet::Error> for Error {
    fn from(e: grin_wallet::libwallet::Error) -> Self {
        Error::GrinLibWallet { e }
    }
}

impl From<grin_wallet::Error> for Error {
    fn from(e: grin_wallet::Error) -> Self {
        Error::GrinWallet { e }
    }
}

impl From<clap::Error> for Error {
    fn from(e: clap::Error) -> Self {
        Error::Cli { e }
    }
}

impl From<secp256k1::Error> for Error {
    fn from(e: secp256k1::Error) -> Self {
        Error::Secp { e }
    }
}

impl From<ws::Error> for Error {
    fn from(e: ws::Error) -> Self {
        Error::WebSocket { e }
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(e: serde_json::error::Error) -> Self {
        Error::Json { e }
    }
}
