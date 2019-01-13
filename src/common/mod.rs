#[macro_use] pub mod macros;
pub mod error;
pub mod config;
pub mod base58;
pub mod crypto;
pub mod hasher;

pub use self::error::Error;
pub use self::error::Wallet713Error;
pub use self::macros::*;
pub type Result<T> = std::result::Result<T, Error>;
