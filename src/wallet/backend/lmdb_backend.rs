use std::cell::RefCell;
use std::sync::Arc;
use std::{fs, path};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use failure::ResultExt;
use blake2_rfc::blake2b::Blake2b;

use grin_util::{to_hex, from_hex};
use grin_util::secp::constants::SECRET_KEY_SIZE;
use grin_util::ZeroingString;
use grin_core::{global, ser};
use grin_store::{self, option_to_not_found, to_key, to_key_u64};
use grin_wallet::WalletConfig;

use crate::wallet::types::TxProof;

use super::types::{ErrorKind, Result, WalletSeed, WalletBackend, WalletBackendBatch, ChildNumber, Transaction, OutputData, TxLogEntry, AcctPathMapping, Context, ExtKeychain, Identifier, Keychain, NodeClient};
use super::api::restore;

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
    let root_key = keychain.derive_key(0, &K::root_key_id())?;

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

pub struct Backend<C, K> {
    db: grin_store::Store,
    passphrase: ZeroingString,
    pub keychain: Option<K>,
    parent_key_id: Identifier,
    config: WalletConfig,
    w2n_client: C,
}

impl<C, K> Backend<C, K> {
    pub fn new(config: &WalletConfig, passphrase: &str, n_client: C) -> Result<Self> {
        let db_path = path::Path::new(&config.data_file_dir).join(DB_DIR);
        fs::create_dir_all(&db_path).expect("Couldn't create wallet backend directory!");

        let stored_tx_path = path::Path::new(&config.data_file_dir).join(TX_SAVE_DIR);
        fs::create_dir_all(&stored_tx_path)
            .expect("Couldn't create wallet backend tx storage directory!");

        let stored_tx_proof_path = path::Path::new(&config.data_file_dir).join(TX_PROOF_SAVE_DIR);
        fs::create_dir_all(&stored_tx_proof_path)
            .expect("Couldn't create wallet backend tx proof storage directory!");

        let lmdb_env = Arc::new(grin_store::new_env(db_path.to_str().unwrap().to_string()));
        let store = grin_store::Store::open(lmdb_env, DB_DIR);

        let default_account = AcctPathMapping {
            label: "default".to_string(),
            path: Backend::<C, K>::default_path(),
        };
        let acct_key = to_key(
            ACCOUNT_PATH_MAPPING_PREFIX,
            &mut default_account.label.as_bytes().to_vec(),
        );

        {
            let batch = store.batch()?;
            batch.put_ser(&acct_key, &default_account)?;
            batch.commit()?;
        }

        let res = Backend {
            db: store,
            passphrase: ZeroingString::from(passphrase),
            keychain: None,
            parent_key_id: Backend::<C, K>::default_path(),
            config: config.clone(),
            w2n_client: n_client,
        };
        Ok(res)
    }

    fn default_path() -> Identifier {
        ExtKeychain::derive_key_id(2, 0, 0, 0, 0)
    }
}

impl<C, K> WalletBackend<C, K> for Backend<C, K>
    where
        C: NodeClient,
        K: Keychain,
{
    /// Initialise with whatever stored credentials we have
    fn open_with_credentials(&mut self) -> Result<()> {
        let wallet_seed = WalletSeed::from_file(&self.config, &self.passphrase)
            .context(ErrorKind::OpenWalletError)?;
        self.keychain = Some(
            wallet_seed
                .derive_keychain(global::is_floonet())
                .context(ErrorKind::DeriveKeychainError)?,
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
        let res = self.accounts().find(|l| l.label == label);
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
        option_to_not_found(self.db.get_ser(&key), &format!("Key Id: {}", id)).map_err(|e| e.into())
    }

    fn outputs<'a>(&'a self) -> Box<dyn Iterator<Item = OutputData> + 'a> {
        Box::new(self.db.iter(&[OUTPUT_PREFIX]).unwrap())
    }

    fn get_tx_log_by_slate_id(&self, slate_id: &str) -> Result<Option<TxLogEntry>> {
        let key = to_key(TX_LOG_ENTRY_PREFIX, &mut slate_id.as_bytes().to_vec());
        self.db.get_ser(&key).map_err(|e| e.into())
    }

    fn tx_logs<'a>(&'a self) -> Box<dyn Iterator<Item = TxLogEntry> + 'a> {
        Box::new(self.db.iter(&[TX_LOG_ENTRY_PREFIX]).unwrap())
    }

    fn get_private_context(&mut self, uuid: &str) -> Result<Context> {
        let ctx_key = to_key(PRIVATE_TX_CONTEXT_PREFIX, &mut uuid.as_bytes().to_vec());
        let (blind_xor_key, nonce_xor_key) = private_ctx_xor_keys(self.keychain(), uuid.as_bytes())?;

        let mut ctx: Context = option_to_not_found(
            self.db.get_ser(&ctx_key),
            &format!("Slate id: {}", uuid),
        )?;

        for i in 0..SECRET_KEY_SIZE {
            ctx.sec_key.0[i] = ctx.sec_key.0[i] ^ blind_xor_key[i];
            ctx.sec_nonce.0[i] = ctx.sec_nonce.0[i] ^ nonce_xor_key[i];
        }

        Ok(ctx)
    }

    fn accounts<'a>(&'a self) -> Box<dyn Iterator<Item = AcctPathMapping> + 'a> {
        Box::new(self.db.iter(&[ACCOUNT_PATH_MAPPING_PREFIX]).unwrap())
    }

    fn get_acct_path(&self, label: &str) -> Result<AcctPathMapping> {
        let acct_key = to_key(ACCOUNT_PATH_MAPPING_PREFIX, &mut label.as_bytes().to_vec());
        self.db.get_ser(&acct_key)?.ok_or(ErrorKind::ModelNotFound.into())
    }

    fn get_stored_tx(&self, uuid: &str) -> Result<Transaction> {
        let filename = format!("{}.grintx", uuid);
        let path = path::Path::new(&self.config.data_file_dir)
            .join(TX_SAVE_DIR)
            .join(filename);
        let tx_file = Path::new(&path).to_path_buf();
        let mut tx_f = File::open(tx_file)?;
        let mut content = String::new();
        tx_f.read_to_string(&mut content)?;
        let tx_bin = from_hex(content).unwrap();
        Ok(ser::deserialize::<Transaction>(&mut &tx_bin[..]).unwrap())
    }

    fn has_stored_tx_proof(&self, uuid: &str) -> Result<bool> {
        let filename = format!("{}.proof", uuid);
        let path = path::Path::new(&self.config.data_file_dir)
            .join(TX_PROOF_SAVE_DIR)
            .join(filename);
        let tx_proof_file = Path::new(&path).to_path_buf();
        Ok(tx_proof_file.exists())
    }

    fn get_stored_tx_proof(&self, uuid: &str) -> Result<TxProof> {
        let filename = format!("{}.proof", uuid);
        let path = path::Path::new(&self.config.data_file_dir)
            .join(TX_PROOF_SAVE_DIR)
            .join(filename);
        let tx_proof_file = Path::new(&path).to_path_buf();
        let mut tx_proof_f = File::open(tx_proof_file)?;
        let mut content = String::new();
        tx_proof_f.read_to_string(&mut content)?;
        Ok(serde_json::from_str(&content)?)
    }

    fn batch<'a>(&'a self) -> Result<Box<dyn WalletBackendBatch<K> + 'a>> {
        Ok(Box::new(Batch {
            _store: self,
            db: RefCell::new(Some(self.db.batch()?)),
            keychain: self.keychain.clone(),
        }))
    }

    fn derive_next<'a>(&mut self) -> Result<Identifier> {
        let mut deriv_idx = {
            let batch = self.db.batch()?;
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
        let batch = self.db.batch()?;
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

    fn check_repair(&mut self) -> Result<()> {
        restore::check_repair(self).context(ErrorKind::Restore)?;
        Ok(())
    }

    fn calc_commit_for_cache(&mut self, amount: u64, id: &Identifier) -> Result<Option<String>> {
        if self.config.no_commit_cache == Some(true) {
            Ok(None)
        } else {
            Ok(Some(grin_util::to_hex(
                self.keychain().commit(amount, &id)?.0.to_vec(),
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
        let path = path::Path::new(&self._store.config.data_file_dir)
            .join(TX_SAVE_DIR)
            .join(filename);
        let path_buf = Path::new(&path).to_path_buf();
        let mut stored_tx = File::create(path_buf)?;
        let tx_hex = to_hex(ser::ser_vec(tx).unwrap());;
        stored_tx.write_all(&tx_hex.as_bytes())?;
        stored_tx.sync_all()?;
        Ok(())
    }

    fn store_tx_proof(&self, uuid: &str, tx_proof: &TxProof) -> Result<()> {
        let filename = format!("{}.proof", uuid);
        let path = path::Path::new(&self._store.config.data_file_dir)
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

    fn save_last_confirmed_height(
        &mut self,
        height: u64,
    ) -> Result<()> {
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

    fn save_private_context(&mut self, uuid: &str, ctx: &Context) -> Result<()> {
        let ctx_key = to_key(PRIVATE_TX_CONTEXT_PREFIX, &mut uuid.as_bytes().to_vec());
        let (blind_xor_key, nonce_xor_key) = private_ctx_xor_keys(self.keychain(), uuid.as_bytes())?;

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

    fn delete_private_context(&mut self, uuid: &str) -> Result<()> {
        let ctx_key = to_key(PRIVATE_TX_CONTEXT_PREFIX, &mut uuid.as_bytes().to_vec());
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
