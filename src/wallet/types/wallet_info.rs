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

use crate::common::ser;
use serde::{Deserialize, Serialize};

/// a contained wallet info struct, so automated tests can parse wallet info
/// can add more fields here over time as needed
#[derive(Serialize, Eq, PartialEq, Deserialize, Debug, Clone)]
pub struct WalletInfo {
	/// height from which info was taken
	#[serde(with = "ser::string_or_u64")]
	pub last_confirmed_height: u64,
	/// Minimum number of confirmations for an output to be treated as "spendable".
	#[serde(with = "ser::string_or_u64")]
	pub minimum_confirmations: u64,
	/// total amount in the wallet
	#[serde(with = "ser::string_or_u64")]
	pub total: u64,
	/// amount awaiting finalization
	#[serde(with = "ser::string_or_u64")]
	pub amount_awaiting_finalization: u64,
	/// amount awaiting confirmation
	#[serde(with = "ser::string_or_u64")]
	pub amount_awaiting_confirmation: u64,
	/// coinbases waiting for lock height
	#[serde(with = "ser::string_or_u64")]
	pub amount_immature: u64,
	/// amount currently spendable
	#[serde(with = "ser::string_or_u64")]
	pub amount_currently_spendable: u64,
	/// amount locked via previous transactions
	#[serde(with = "ser::string_or_u64")]
	pub amount_locked: u64,
}
