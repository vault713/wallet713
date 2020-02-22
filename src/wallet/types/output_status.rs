// Copyright 2018 The Grin Developers
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

use serde::{Deserialize, Serialize};
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
