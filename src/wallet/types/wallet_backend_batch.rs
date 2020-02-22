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
	AcctPathMapping, Context, Identifier, Keychain, OutputData, Result, Transaction, TxLogEntry,
};
use crate::wallet::types::TxProof;

pub trait WalletBackendBatch<K>
where
	K: Keychain,
{
	fn keychain(&mut self) -> &mut K;
	fn save_output(&mut self, out: &OutputData) -> Result<()>;
	fn delete_output(&mut self, id: &Identifier, mmr_index: &Option<u64>) -> Result<()>;
	fn lock_output(&mut self, out: &mut OutputData) -> Result<()>;
	fn save_child_index(&mut self, parent_key_id: &Identifier, index: u32) -> Result<()>;
	fn save_last_confirmed_height(&mut self, height: u64) -> Result<()>;
	fn next_tx_log_id(&mut self, parent_key_id: &Identifier) -> Result<u32>;
	fn save_tx_log_entry(&mut self, t: &TxLogEntry) -> Result<()>;
	fn save_acct_path(&mut self, mapping: &AcctPathMapping) -> Result<()>;
	fn save_private_context(
		&mut self,
		slate_id: &[u8],
		participant_id: usize,
		ctx: &Context,
	) -> Result<()>;
	fn delete_private_context(&mut self, slate_id: &[u8], participant_id: usize) -> Result<()>;
	fn store_tx(&self, uuid: &str, tx: &Transaction) -> Result<()>;
	fn store_tx_proof(&self, uuid: &str, tx_proof: &TxProof) -> Result<()>;
	fn commit(&mut self) -> Result<()>;
}
