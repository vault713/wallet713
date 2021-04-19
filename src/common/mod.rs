// Copyright 2019 The vault713 Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
pub trait Keychain: epic_keychain::Keychain + Clone + 'static {}
impl Keychain for epic_keychain::ExtKeychain {}

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
