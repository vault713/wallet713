#[macro_use]
pub mod macros;
pub mod base58;
pub mod client;
pub mod config;
pub mod crypto;
mod error_kind;
pub mod hasher;
pub mod message;
pub mod motd;
pub mod ser;

pub use self::error_kind::ErrorKind;
pub use self::macros::*;
pub use failure::Error;
pub use parking_lot::{Mutex, MutexGuard};
use std::result::Result as StdResult;
pub use std::sync::Arc;

pub type Result<T> = StdResult<T, Error>;
pub trait Keychain: grin_keychain::Keychain + Clone + 'static {}
impl Keychain for grin_keychain::ExtKeychain {}

#[derive(Clone, PartialEq)]
pub enum RuntimeMode {
	Cli,
	Daemon,
}

static mut RUNTIME_MODE: RuntimeMode = RuntimeMode::Cli;

pub unsafe fn set_runtime_mode(runtime_mode: &RuntimeMode) {
	RUNTIME_MODE = runtime_mode.clone();
}

pub fn is_cli() -> bool {
	unsafe { RUNTIME_MODE == RuntimeMode::Cli }
}

pub const COLORED_PROMPT: &'static str = "\x1b[36mwallet713>\x1b[0m ";
