#[macro_use] pub mod macros;
pub mod config;
pub mod base58;
pub mod crypto;
pub mod hasher;
mod error_kind;

pub use self::error_kind::ErrorKind;
pub use self::macros::*;
pub use failure::Error;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, PartialEq)]
pub enum RuntimeMode {
    Cli,
    Daemon
}

static mut RUNTIME_MODE: RuntimeMode = RuntimeMode::Cli;

pub unsafe fn set_runtime_mode(runtime_mode: &RuntimeMode) {
    RUNTIME_MODE = runtime_mode.clone();
}

pub unsafe fn is_cli() -> bool {
    RUNTIME_MODE == RuntimeMode::Cli
}

pub const COLORED_PROMPT: &'static str = "\x1b[36mwallet713>\x1b[0m ";
pub const PROMPT: &'static str = "wallet713> ";
