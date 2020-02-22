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

use super::{
	AcctPathMapping, Context, Identifier, Keychain, NodeClient, OutputData, Result, Transaction,
	TxLogEntry, TxProof, WalletBackendBatch,
};
use grin_util::ZeroingString;

pub trait WalletBackend<C, K>: Send + 'static
where
	C: NodeClient,
	K: Keychain,
{
	/// Check whether the backend has a seed or not
	fn has_seed(&self) -> Result<bool>;
	/// Get the seed
	fn get_seed(&self) -> Result<ZeroingString>;
	/// Set a new seed, encrypt with `password`
	/// Should fail if backend already has a seed,
	/// unless `overwrite` is set to `true
	fn set_seed(
		&mut self,
		mnemonic: Option<ZeroingString>,
		password: ZeroingString,
		overwrite: bool,
	) -> Result<()>;
	/// Check if the backend connection is established
	fn connected(&self) -> Result<bool>;
	/// Connect to the backend
	fn connect(&mut self) -> Result<()>;
	/// Disconnect from backend
	fn disconnect(&mut self) -> Result<()>;
	/// Set password
	fn set_password(&mut self, password: ZeroingString) -> Result<()>;
	/// Clear out backend
	fn clear(&mut self) -> Result<()>;

	fn open_with_credentials(&mut self) -> Result<()>;
	fn close(&mut self) -> Result<()>;
	fn restore(&mut self) -> Result<()>;
	fn check_repair(&mut self, delete_unconfirmed: bool) -> Result<()>;
	fn get_parent_key_id(&self) -> Identifier;
	fn set_parent_key_id(&mut self, id: &Identifier);
	fn set_parent_key_id_by_name(&mut self, label: &str) -> Result<()>;
	fn w2n_client(&mut self) -> &mut C;
	fn calc_commit_for_cache(&mut self, amount: u64, id: &Identifier) -> Result<Option<String>>;
	fn keychain(&mut self) -> &mut K;
	fn next_child(&mut self) -> Result<Identifier>;
	fn get_output(&self, id: &Identifier, mmr_index: &Option<u64>) -> Result<OutputData>;
	fn get_private_context(&mut self, slate_id: &[u8], participant_id: usize) -> Result<Context>;
	fn get_acct_path(&self, label: &str) -> Result<Option<AcctPathMapping>>;
	fn get_last_confirmed_height(&self) -> Result<u64>;
	fn get_stored_tx(&self, uuid: &str) -> Result<Option<Transaction>>;
	fn has_stored_tx_proof(&self, uuid: &str) -> Result<bool>;
	fn get_stored_tx_proof(&self, uuid: &str) -> Result<Option<TxProof>>;
	fn get_tx_log_by_slate_id(&self, slate_id: &str) -> Result<Option<TxLogEntry>>;
	fn outputs<'a>(&'a self) -> Result<Box<dyn Iterator<Item = OutputData> + 'a>>;
	fn tx_logs<'a>(&'a self) -> Result<Box<dyn Iterator<Item = TxLogEntry> + 'a>>;
	fn accounts<'a>(&'a self) -> Result<Box<dyn Iterator<Item = AcctPathMapping> + 'a>>;
	fn batch<'a>(&'a self) -> Result<Box<dyn WalletBackendBatch<K> + 'a>>;
}
