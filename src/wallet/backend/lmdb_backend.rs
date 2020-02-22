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

use super::types::{
	AcctPathMapping, ChildNumber, Context, Identifier, NodeClient, OutputData, Result, Transaction,
	TxLogEntry, TxProof, WalletBackend, WalletBackendBatch, WalletSeed,
};
use crate::common::config::WalletConfig;
use crate::common::{ErrorKind, Keychain};
use crate::internal::restore;
use blake2_rfc::blake2b::Blake2b;
use chrono::Utc;
use failure::ResultExt;
use grin_core::{global, ser};
use grin_keychain::SwitchCommitmentType;
use grin_store::Store;
use grin_store::{self, option_to_not_found, to_key, to_key_u64};
use grin_util::secp::constants::SECRET_KEY_SIZE;
use grin_util::{from_hex, to_hex, ZeroingString};
use std::cell::RefCell;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::Path;

pub const DB_DIR: &'static str = "db";
pub const TX_SAVE_DIR: &'static str = "saved_txs";
pub const TX_PROOF_SAVE_DIR: &'static str = "saved_proofs";

const OUTPUT_PREFIX: u8 = 'o' as u8;
const DERIV_PREFIX: u8 = 'd' as u8;
const CONFIRMED_HEIGHT_PREFIX: u8 = 'c' as u8;
const PRIVATE_TX_CONTEXT_PREFIX: u8 = 'p' as u8;
const TX_LOG_ENTRY_PREFIX: u8 = 't' as u8;
const TX_LOG_ID_PREFIX: u8 = 'i' as u8;
const ACCOUNT_PATH_MAPPING_PREFIX: u8 = 'a' as u8;

fn private_ctx_xor_keys<K>(
	keychain: &K,
	slate_id: &[u8],
) -> Result<([u8; SECRET_KEY_SIZE], [u8; SECRET_KEY_SIZE])>
where
	K: Keychain,
{
	let root_key = keychain.derive_key(0, &K::root_key_id(), &SwitchCommitmentType::None)?;

	// derive XOR values for storing secret values in DB
	// h(root_key|slate_id|"blind")
	let mut hasher = Blake2b::new(SECRET_KEY_SIZE);
	hasher.update(&root_key.0[..]);
	hasher.update(&slate_id[..]);
	hasher.update(&"blind".as_bytes()[..]);
	let blind_xor_key = hasher.finalize();
	let mut ret_blind = [0; SECRET_KEY_SIZE];
	ret_blind.copy_from_slice(&blind_xor_key.as_bytes()[0..SECRET_KEY_SIZE]);

	// h(root_key|slate_id|"nonce")
	let mut hasher = Blake2b::new(SECRET_KEY_SIZE);
	hasher.update(&root_key.0[..]);
	hasher.update(&slate_id[..]);
	hasher.update(&"nonce".as_bytes()[..]);
	let nonce_xor_key = hasher.finalize();
	let mut ret_nonce = [0; SECRET_KEY_SIZE];
	ret_nonce.copy_from_slice(&nonce_xor_key.as_bytes()[0..SECRET_KEY_SIZE]);

	Ok((ret_blind, ret_nonce))
}

pub struct Backend<C, K>
where
	C: NodeClient,
	K: Keychain,
{
	db: Option<Store>,
	password: Option<ZeroingString>,
	pub keychain: Option<K>,
	parent_key_id: Identifier,
	config: WalletConfig,
	w2n_client: C,
}

impl<C, K> Backend<C, K>
where
	C: NodeClient,
	K: Keychain,
{
	fn db(&self) -> Result<&Store> {
		self.db.as_ref().ok_or(ErrorKind::NoWallet.into())
	}

	/// Create `Backend` instance
	pub fn new(config: &WalletConfig, client: C) -> Result<Self> {
		Ok(Self {
			db: None,
			password: None,
			keychain: None,
			parent_key_id: K::derive_key_id(2, 0, 0, 0, 0),
			config: config.clone(),
			w2n_client: client,
		})
	}

	/*pub fn new(config: &WalletConfig, password: &str, n_client: C) -> Result<Self> {
		let res = Backend {
			db: None,
			password: Some(ZeroingString::from(password)),
			keychain: None,
			parent_key_id: K::derive_key_id(2, 0, 0, 0, 0),
			config: config.clone(),
			w2n_client: n_client,
		};
		Ok(res)
	}*/
}

impl<C, K> WalletBackend<C, K> for Backend<C, K>
where
	C: NodeClient,
	K: Keychain,
{
	/// Check whether the backend has a seed or not
	fn has_seed(&self) -> Result<bool> {
		Ok(WalletSeed::seed_file_exists(&self.config).is_err())
	}

	/// Get the seed
	fn get_seed(&self) -> Result<ZeroingString> {
		match &self.password {
			Some(p) => {
				let seed = WalletSeed::from_file(&self.config, p)?;
				seed.to_mnemonic().map(|s| s.into())
			}
			None => Err(ErrorKind::NoWallet.into()),
		}
	}

	/// Set a new seed, encrypt with `password`
	/// Should fail if backend already has a seed,
	/// unless `overwrite` is set to `true
	fn set_seed(
		&mut self,
		mnemonic: Option<ZeroingString>,
		password: ZeroingString,
		overwrite: bool,
	) -> Result<()> {
		if self.has_seed()? && !overwrite {
			return Err(ErrorKind::WalletHasSeed.into());
		}
		self.password = Some(password.clone());
		let _ = WalletSeed::init_file(&self.config, 24, mnemonic, &password, overwrite)?;
		Ok(())
	}

	/// Check if the backend connection is established
	fn connected(&self) -> Result<bool> {
		Ok(self.db.is_some())
	}

	/// Connect to the backend
	fn connect(&mut self) -> Result<()> {
		if !self.has_seed()? {
			return Err(ErrorKind::WalletNoSeed.into());
		}
		if self.connected()? {
			return Err(ErrorKind::WalletConnected.into());
		}

		let root_path = Path::new(&self.config.data_file_dir);

		let db_path = root_path.join(DB_DIR);
		fs::create_dir_all(&db_path)?;

		let stored_tx_path = root_path.join(TX_SAVE_DIR);
		fs::create_dir_all(&stored_tx_path)?;

		let stored_tx_proof_path = root_path.join(TX_PROOF_SAVE_DIR);
		fs::create_dir_all(&stored_tx_proof_path)?;

		let store = Store::new(db_path.to_str().unwrap(), None, Some(DB_DIR), None)?;

		let default_account = AcctPathMapping {
			label: "default".to_string(),
			path: K::derive_key_id(2, 0, 0, 0, 0),
		};
		let acct_key = to_key(
			ACCOUNT_PATH_MAPPING_PREFIX,
			&mut default_account.label.as_bytes().to_vec(),
		);

		if !store.exists(&acct_key)? {
			let batch = store.batch()?;
			batch.put_ser(&acct_key, &default_account)?;
			batch.commit()?;
		}

		self.db = Some(store);
		Ok(())
	}

	/// Disconnect from backend
	fn disconnect(&mut self) -> Result<()> {
		self.db = None;
		Ok(())
	}

	/// Set password
	fn set_password(&mut self, password: ZeroingString) -> Result<()> {
		let _ = WalletSeed::from_file(&self.config, password.deref())?;
		self.password = Some(password);
		Ok(())
	}

	/// Clear out backend
	fn clear(&mut self) -> Result<()> {
		self.disconnect()?;

		let root_path = Path::new(&self.config.data_file_dir);
		if !root_path.exists() {
			return Ok(());
		}

		let backup_dir = Utc::now().format("%Y%m%d-%H%M%S").to_string();
		let backup_path = root_path.join("backups").join(backup_dir);
		fs::create_dir_all(&backup_path)?;
		let db_path = root_path.join(DB_DIR);
		if db_path.exists() {
			fs::rename(&db_path, &backup_path.join(DB_DIR))?;
		}
		let txs_path = root_path.join(TX_SAVE_DIR);
		if txs_path.exists() {
			fs::rename(&txs_path, &backup_path.join(TX_SAVE_DIR))?;
		}
		let proofs_path = root_path.join(TX_PROOF_SAVE_DIR);
		if proofs_path.exists() {
			fs::rename(&proofs_path, &backup_path.join(TX_PROOF_SAVE_DIR))?;
		}

		self.connect()?;

		Ok(())
	}

	/// Initialise with whatever stored credentials we have
	fn open_with_credentials(&mut self) -> Result<()> {
		let wallet_seed = WalletSeed::from_file(
			&self.config,
			&self.password.clone().ok_or(ErrorKind::OpenWalletError)?,
		)
		.map_err(|_| ErrorKind::OpenWalletError)?;
		self.keychain = Some(
			wallet_seed
				.derive_keychain(global::is_floonet())
				.map_err(|_| ErrorKind::DeriveKeychainError)?,
		);
		Ok(())
	}

	/// Close wallet and remove any stored credentials (TBD)
	fn close(&mut self) -> Result<()> {
		self.keychain = None;
		Ok(())
	}

	/// Return the keychain being used
	fn keychain(&mut self) -> &mut K {
		self.keychain.as_mut().unwrap()
	}

	/// Return the node client being used
	fn w2n_client(&mut self) -> &mut C {
		&mut self.w2n_client
	}

	/// Set parent path by account name
	fn set_parent_key_id_by_name(&mut self, label: &str) -> Result<()> {
		let label = label.to_owned();
		let res = self.accounts()?.find(|l| l.label == label);
		if let Some(a) = res {
			self.set_parent_key_id(&a.path);
			Ok(())
		} else {
			return Err(ErrorKind::UnknownAccountLabel(label.clone()).into());
		}
	}

	/// set parent path
	fn set_parent_key_id(&mut self, id: &Identifier) {
		self.parent_key_id = id.clone();
	}

	fn get_parent_key_id(&self) -> Identifier {
		self.parent_key_id.clone()
	}

	fn get_output(&self, id: &Identifier, mmr_index: &Option<u64>) -> Result<OutputData> {
		let key = match mmr_index {
			Some(i) => to_key_u64(OUTPUT_PREFIX, &mut id.to_bytes().to_vec(), *i),
			None => to_key(OUTPUT_PREFIX, &mut id.to_bytes().to_vec()),
		};
		option_to_not_found(self.db()?.get_ser(&key), &format!("Key Id: {}", id))
			.map_err(|e| e.into())
	}

	fn outputs<'a>(&'a self) -> Result<Box<dyn Iterator<Item = OutputData> + 'a>> {
		Ok(Box::new(
			self.db()?.iter(&[OUTPUT_PREFIX]).unwrap().map(|x| x.1),
		))
	}

	fn get_tx_log_by_slate_id(&self, slate_id: &str) -> Result<Option<TxLogEntry>> {
		let key = to_key(TX_LOG_ENTRY_PREFIX, &mut slate_id.as_bytes().to_vec());
		self.db()?.get_ser(&key).map_err(|e| e.into())
	}

	fn tx_logs<'a>(&'a self) -> Result<Box<dyn Iterator<Item = TxLogEntry> + 'a>> {
		Ok(Box::new(
			self.db()?
				.iter(&[TX_LOG_ENTRY_PREFIX])
				.unwrap()
				.map(|x| x.1),
		))
	}

	fn get_private_context(&mut self, slate_id: &[u8], participant_id: usize) -> Result<Context> {
		let ctx_key = to_key_u64(
			PRIVATE_TX_CONTEXT_PREFIX,
			&mut slate_id.to_vec(),
			participant_id as u64,
		);
		let (blind_xor_key, nonce_xor_key) = private_ctx_xor_keys(self.keychain(), slate_id)?;

		let mut ctx: Context = option_to_not_found(
			self.db()?.get_ser(&ctx_key),
			&format!("Slate id: {:x?}", slate_id.to_vec()),
		)?;

		for i in 0..SECRET_KEY_SIZE {
			ctx.sec_key.0[i] = ctx.sec_key.0[i] ^ blind_xor_key[i];
			ctx.sec_nonce.0[i] = ctx.sec_nonce.0[i] ^ nonce_xor_key[i];
		}

		Ok(ctx)
	}

	fn accounts<'a>(&'a self) -> Result<Box<dyn Iterator<Item = AcctPathMapping> + 'a>> {
		Ok(Box::new(
			self.db()?
				.iter(&[ACCOUNT_PATH_MAPPING_PREFIX])
				.unwrap()
				.map(|x| x.1),
		))
	}

	fn get_acct_path(&self, label: &str) -> Result<Option<AcctPathMapping>> {
		let acct_key = to_key(ACCOUNT_PATH_MAPPING_PREFIX, &mut label.as_bytes().to_vec());
		let ser = self.db()?.get_ser(&acct_key)?;
		Ok(ser)
	}

	fn get_stored_tx(&self, uuid: &str) -> Result<Option<Transaction>> {
		let filename = format!("{}.grintx", uuid);
		let path = Path::new(&self.config.data_file_dir)
			.join(TX_SAVE_DIR)
			.join(filename);
		if !path.exists() {
			return Ok(None);
		}
		let tx_file = Path::new(&path).to_path_buf();
		let mut tx_f = File::open(tx_file)?;
		let mut content = String::new();
		tx_f.read_to_string(&mut content)?;
		let tx_bin = from_hex(content).unwrap();
		Ok(Some(
			ser::deserialize::<Transaction>(&mut &tx_bin[..]).unwrap(),
		))
	}

	fn has_stored_tx_proof(&self, uuid: &str) -> Result<bool> {
		let filename = format!("{}.proof", uuid);
		let path = Path::new(&self.config.data_file_dir)
			.join(TX_PROOF_SAVE_DIR)
			.join(filename);
		let tx_proof_file = Path::new(&path).to_path_buf();
		Ok(tx_proof_file.exists())
	}

	fn get_stored_tx_proof(&self, uuid: &str) -> Result<Option<TxProof>> {
		let filename = format!("{}.proof", uuid);
		let path = Path::new(&self.config.data_file_dir)
			.join(TX_PROOF_SAVE_DIR)
			.join(filename);
		let tx_proof_file = Path::new(&path).to_path_buf();
		if !tx_proof_file.exists() {
			return Ok(None);
		}
		let mut tx_proof_f = File::open(tx_proof_file)?;
		let mut content = String::new();
		tx_proof_f.read_to_string(&mut content)?;
		Ok(Some(serde_json::from_str(&content)?))
	}

	fn batch<'a>(&'a self) -> Result<Box<dyn WalletBackendBatch<K> + 'a>> {
		Ok(Box::new(Batch {
			_store: self,
			db: RefCell::new(Some(self.db()?.batch()?)),
			keychain: self.keychain.clone(),
		}))
	}

	fn next_child<'a>(&mut self) -> Result<Identifier> {
		let mut deriv_idx = {
			let batch = self.db()?.batch()?;
			let deriv_key = to_key(DERIV_PREFIX, &mut self.parent_key_id.to_bytes().to_vec());
			match batch.get_ser(&deriv_key)? {
				Some(idx) => idx,
				None => 0,
			}
		};
		let mut return_path = self.parent_key_id.to_path();
		return_path.depth = return_path.depth + 1;
		return_path.path[return_path.depth as usize - 1] = ChildNumber::from(deriv_idx);
		deriv_idx = deriv_idx + 1;
		let mut batch = self.batch()?;
		batch.save_child_index(&self.parent_key_id, deriv_idx)?;
		batch.commit()?;
		Ok(Identifier::from_path(&return_path))
	}

	fn get_last_confirmed_height<'a>(&self) -> Result<u64> {
		let batch = self.db()?.batch()?;
		let height_key = to_key(
			CONFIRMED_HEIGHT_PREFIX,
			&mut self.parent_key_id.to_bytes().to_vec(),
		);
		let last_confirmed_height = match batch.get_ser(&height_key)? {
			Some(h) => h,
			None => 0,
		};
		Ok(last_confirmed_height)
	}

	fn restore(&mut self) -> Result<()> {
		restore::restore(self).context(ErrorKind::Restore)?;
		Ok(())
	}

	fn check_repair(&mut self, delete_unconfirmed: bool) -> Result<()> {
		restore::check_repair(self, delete_unconfirmed).context(ErrorKind::Restore)?;
		Ok(())
	}

	fn calc_commit_for_cache(&mut self, amount: u64, id: &Identifier) -> Result<Option<String>> {
		if self.config.no_commit_cache == Some(true) {
			Ok(None)
		} else {
			Ok(Some(grin_util::to_hex(
				self.keychain()
					.commit(amount, id, &SwitchCommitmentType::Regular)?
					.0
					.to_vec(),
			)))
		}
	}
}

/// An atomic batch in which all changes can be committed all at once or
/// discarded on error.
pub struct Batch<'a, C, K>
where
	C: NodeClient,
	K: Keychain,
{
	_store: &'a Backend<C, K>,
	db: RefCell<Option<grin_store::Batch<'a>>>,
	/// Keychain
	keychain: Option<K>,
}

#[allow(missing_docs)]
impl<'a, C, K> WalletBackendBatch<K> for Batch<'a, C, K>
where
	C: NodeClient,
	K: Keychain,
{
	fn keychain(&mut self) -> &mut K {
		self.keychain.as_mut().unwrap()
	}

	fn save_output(&mut self, out: &OutputData) -> Result<()> {
		// Save the output data to the db.
		{
			let key = match out.mmr_index {
				Some(i) => to_key_u64(OUTPUT_PREFIX, &mut out.key_id.to_bytes().to_vec(), i),
				None => to_key(OUTPUT_PREFIX, &mut out.key_id.to_bytes().to_vec()),
			};
			self.db.borrow().as_ref().unwrap().put_ser(&key, &out)?;
		}

		Ok(())
	}

	fn delete_output(&mut self, id: &Identifier, mmr_index: &Option<u64>) -> Result<()> {
		// Delete the output data.
		{
			let key = match mmr_index {
				Some(i) => to_key_u64(OUTPUT_PREFIX, &mut id.to_bytes().to_vec(), *i),
				None => to_key(OUTPUT_PREFIX, &mut id.to_bytes().to_vec()),
			};
			let _ = self.db.borrow().as_ref().unwrap().delete(&key);
		}

		Ok(())
	}

	fn store_tx(&self, uuid: &str, tx: &Transaction) -> Result<()> {
		let filename = format!("{}.grintx", uuid);
		let path = Path::new(&self._store.config.data_file_dir)
			.join(TX_SAVE_DIR)
			.join(filename);
		let path_buf = Path::new(&path).to_path_buf();
		let mut stored_tx = File::create(path_buf)?;
		let tx_hex = to_hex(ser::ser_vec(tx).unwrap());
		stored_tx.write_all(&tx_hex.as_bytes())?;
		stored_tx.sync_all()?;
		Ok(())
	}

	fn store_tx_proof(&self, uuid: &str, tx_proof: &TxProof) -> Result<()> {
		let filename = format!("{}.proof", uuid);
		let path = Path::new(&self._store.config.data_file_dir)
			.join(TX_PROOF_SAVE_DIR)
			.join(filename);
		let path_buf = Path::new(&path).to_path_buf();
		let mut stored_tx = File::create(path_buf)?;
		let proof_ser = serde_json::to_string(tx_proof)?;
		stored_tx.write_all(&proof_ser.as_bytes())?;
		stored_tx.sync_all()?;
		Ok(())
	}

	fn next_tx_log_id(&mut self, parent_key_id: &Identifier) -> Result<u32> {
		let tx_id_key = to_key(TX_LOG_ID_PREFIX, &mut parent_key_id.to_bytes().to_vec());
		let last_tx_log_id = match self.db.borrow().as_ref().unwrap().get_ser(&tx_id_key)? {
			Some(t) => t,
			None => 0,
		};
		self.db
			.borrow()
			.as_ref()
			.unwrap()
			.put_ser(&tx_id_key, &(last_tx_log_id + 1))?;
		Ok(last_tx_log_id)
	}

	fn save_last_confirmed_height(&mut self, height: u64) -> Result<()> {
		let height_key = to_key(
			CONFIRMED_HEIGHT_PREFIX,
			&mut self._store.get_parent_key_id().to_bytes().to_vec(),
		);
		self.db
			.borrow()
			.as_ref()
			.unwrap()
			.put_ser(&height_key, &height)?;
		Ok(())
	}

	fn save_child_index(&mut self, parent_key_id: &Identifier, index: u32) -> Result<()> {
		let deriv_key = to_key(DERIV_PREFIX, &mut parent_key_id.to_bytes().to_vec());
		self.db
			.borrow()
			.as_ref()
			.unwrap()
			.put_ser(&deriv_key, &index)?;
		Ok(())
	}

	fn save_tx_log_entry(&mut self, t: &TxLogEntry) -> Result<()> {
		let tx_log_key = to_key_u64(
			TX_LOG_ENTRY_PREFIX,
			&mut t.parent_key_id.to_bytes().to_vec(),
			t.id as u64,
		);
		self.db
			.borrow()
			.as_ref()
			.unwrap()
			.put_ser(&tx_log_key, &t)?;
		Ok(())
	}

	fn save_acct_path(&mut self, mapping: &AcctPathMapping) -> Result<()> {
		let acct_key = to_key(
			ACCOUNT_PATH_MAPPING_PREFIX,
			&mut mapping.label.as_bytes().to_vec(),
		);
		self.db
			.borrow()
			.as_ref()
			.unwrap()
			.put_ser(&acct_key, &mapping)?;
		Ok(())
	}

	fn lock_output(&mut self, out: &mut OutputData) -> Result<()> {
		out.lock();
		self.save_output(out)
	}

	fn save_private_context(
		&mut self,
		slate_id: &[u8],
		participant_id: usize,
		ctx: &Context,
	) -> Result<()> {
		let ctx_key = to_key_u64(
			PRIVATE_TX_CONTEXT_PREFIX,
			&mut slate_id.to_vec(),
			participant_id as u64,
		);
		let (blind_xor_key, nonce_xor_key) = private_ctx_xor_keys(self.keychain(), slate_id)?;

		let mut s_ctx = ctx.clone();
		for i in 0..SECRET_KEY_SIZE {
			s_ctx.sec_key.0[i] = s_ctx.sec_key.0[i] ^ blind_xor_key[i];
			s_ctx.sec_nonce.0[i] = s_ctx.sec_nonce.0[i] ^ nonce_xor_key[i];
		}

		self.db
			.borrow()
			.as_ref()
			.unwrap()
			.put_ser(&ctx_key, &s_ctx)?;
		Ok(())
	}

	fn delete_private_context(&mut self, slate_id: &[u8], participant_id: usize) -> Result<()> {
		let ctx_key = to_key_u64(
			PRIVATE_TX_CONTEXT_PREFIX,
			&mut slate_id.to_vec(),
			participant_id as u64,
		);
		self.db
			.borrow()
			.as_ref()
			.unwrap()
			.delete(&ctx_key)
			.map_err(|e| e.into())
	}

	fn commit(&mut self) -> Result<()> {
		let db = self.db.replace(None);
		db.unwrap().commit()?;
		Ok(())
	}
}
