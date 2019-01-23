use std::sync::Arc;
use grin_wallet::{HTTPNodeClient, NodeClient, WalletConfig};

use common::{ErrorKind, Result};
use common::config::Wallet713Config;

use super::types::{SecretKey, Slate, ExtKeychain, Mutex, WalletBackend, WalletInst, WalletSeed};
use super::backend::Backend;
use super::api::{controller, display};

use crate::common::hasher::derive_address_key;
use crate::common::crypto::Hex;
use crate::wallet::types::TxProof;
use crate::wallet::api::Wallet713OwnerAPI;

pub struct Wallet {
    active_account: String,
    backend: Option<Arc<Mutex<Backend<HTTPNodeClient, ExtKeychain>>>>,
    max_auto_accept_invoice: Option<u64>,
}

impl Wallet {
    pub fn new(max_auto_accept_invoice: Option<u64>) -> Self {
        Self {
            active_account: "default".to_string(),
            backend: None,
            max_auto_accept_invoice,
        }
    }

    pub fn unlock(&mut self, config: &Wallet713Config, account: &str, passphrase: &str) -> Result<()> {
        self.lock();
        self.create_wallet_instance(config, account, passphrase).map_err(|_| {
            ErrorKind::WalletUnlockFailed
        })?;
        self.active_account = account.to_string();
        Ok(())
    }

    pub fn lock(&mut self) {
        self.backend = None;
    }

    pub fn is_locked(&self) -> bool {
        self.backend.is_none()
    }

    pub fn init(&mut self, config: &Wallet713Config, account: &str, passphrase: &str) -> Result<()> {
        let wallet_config = config.as_wallet_config()?;
        self.init_seed(&wallet_config, passphrase)?;
        self.init_backend(&wallet_config, &config, passphrase)?;
        self.unlock(config, account, passphrase)?;
        Ok(())
    }

    pub fn restore_seed(&self, config: &Wallet713Config, words: &Vec<&str>, passphrase: &str) -> Result<()> {
        let wallet_config = config.as_wallet_config()?;
        WalletSeed::recover_from_phrase(&wallet_config, &words.join(" "), passphrase)?;
        Ok(())
    }

    pub fn list_accounts(&self) -> Result<()> {
        let wallet = self.get_wallet_instance()?;
        controller::owner_single_use(wallet.clone(), |api| {
            let acct_mappings = api.accounts()?;
            display::accounts(acct_mappings);
            Ok(())
        })?;
        Ok(())
    }

    pub fn create_account(&self, name: &str) -> Result<()> {
        let wallet = self.get_wallet_instance()?;
        controller::owner_single_use(wallet.clone(), |api| {
            api.create_account_path(name)?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn info(&self) -> Result<()> {
        let wallet = self.get_wallet_instance()?;
        controller::owner_single_use(wallet.clone(), |api| {
            let (validated, wallet_info) = api.retrieve_summary_info(true, 10)?;
            display::info(
                &self.active_account,
                &wallet_info,
                validated,
                true,
            );
            Ok(())
        })?;
        Ok(())
    }

    pub fn txs(&self) -> Result<()> {
        let wallet = self.get_wallet_instance()?;
        controller::owner_single_use(wallet.clone(), |api| {
            let (height, _) = api.node_height()?;
            let (validated, txs) = api.retrieve_txs(true, None, None)?;
            display::txs(
                &self.active_account,
                height,
                validated,
                txs,
                true,
                true,
            )?;
            Ok(())
        })?;

        Ok(())
    }

    pub fn outputs(&self, show_spent: bool) -> Result<()> {
        let wallet = self.get_wallet_instance()?;
        let result = controller::owner_single_use(wallet.clone(), |api| {
            let (height, _) = api.node_height()?;
            let (validated, outputs) = api.retrieve_outputs(show_spent, true, None)?;
            display::outputs(
                &self.active_account,
                height,
                validated,
                outputs,
                true,
            )?;
            Ok(())
        })?;
        Ok(result)
    }

    pub fn initiate_send_tx(&self, address: Option<String>, amount: u64, minimum_confirmations: u64, selection_strategy: &str, change_outputs: usize, max_outputs: usize, message: Option<String>) -> Result<Slate> {
        let wallet = self.get_wallet_instance()?;
        let mut s: Slate = Slate::blank(0);
        controller::owner_single_use(wallet.clone(), |api| {
            let (slate, lock_fn) = api.initiate_tx(
                address,
                amount,
                minimum_confirmations,
                max_outputs,
                change_outputs,
                selection_strategy == "all",
                message,
            )?;
            api.tx_lock_outputs(&slate.tx, lock_fn)?;
            s = slate;
            Ok(())
        })?;
        Ok(s)
    }

    pub fn initiate_receive_tx(&self, amount: u64, num_outputs: usize) -> Result<Slate> {
        let wallet = self.get_wallet_instance()?;
        let mut s: Slate = Slate::blank(0);
        controller::foreign_single_use(wallet.clone(), |api| {
            let (slate, add_fn) = api.initiate_receive_tx(
                amount,
                num_outputs,
                None,
            )?;
            api.tx_add_invoice_outputs(&slate, add_fn)?;
            s = slate;
            Ok(())
        })?;
        Ok(s)
    }

    pub fn repost(&self, id: u32, fluff: bool) -> Result<()> {
        let wallet = self.get_wallet_instance()?;
        controller::owner_single_use(wallet.clone(), |api| {
            let (_, txs) = api.retrieve_txs(true, Some(id), None)?;
            if txs.len() == 0 {
                return Err(grin_wallet::libwallet::ErrorKind::GenericError(
                    format!("could not find transaction with id {}!", id)
                ))?
            }
            let slate_id = txs[0].tx_slate_id;
            if let Some(slate_id) = slate_id {
                let stored_tx = api.get_stored_tx(&slate_id.to_string())?;
                api.post_tx(&stored_tx, fluff)?;
                Ok(())
            } else {
                Err(grin_wallet::libwallet::ErrorKind::GenericError(
                    format!("no transaction data stored for id {}, can not repost!", id)
                ))?
            }
        })?;
        Ok(())
    }

    pub fn cancel(&self, id: u32) -> Result<()> {
        let wallet = self.get_wallet_instance()?;
        controller::owner_single_use(wallet.clone(), |api| {
            api.cancel_tx(Some(id), None)
        })?;
        Ok(())
    }

    pub fn restore_state(&self) -> Result<()> {
        let wallet = self.get_wallet_instance()?;
        controller::owner_single_use(wallet.clone(), |api| {
            api.restore()
        })?;
        Ok(())
    }

    pub fn check_repair(&self) -> Result<()> {
        let wallet = self.get_wallet_instance()?;
        controller::owner_single_use(wallet.clone(), |api| {
            api.check_repair()
        })?;
        Ok(())
    }

    pub fn process_sender_initiated_slate(&self, address: Option<String>, slate: &mut Slate) -> Result<()> {
        let wallet = self.get_wallet_instance()?;
        controller::foreign_single_use(wallet.clone(), |api| {
            api.receive_tx(address, slate, None)?;
            Ok(())
        }).map_err(|_| {
            ErrorKind::GrinWalletReceiveError
        })?;
        Ok(())
    }

    pub fn process_receiver_initiated_slate(&self, slate: &mut Slate) -> Result<()> {
        // reject by default unless wallet is set to auto accept invoices under a certain threshold
        let max_auto_accept_invoice = self.max_auto_accept_invoice.ok_or(ErrorKind::DoesNotAcceptInvoices)?;

        if slate.amount > max_auto_accept_invoice {
            Err(ErrorKind::InvoiceAmountTooBig(slate.amount))?;
        }

        let wallet = self.get_wallet_instance()?;

        controller::owner_single_use(wallet.clone(), |api| {
            let lock_fn = api.invoice_tx(
                slate,
                10,
                500,
                1,
                false,
                None,
            )?;
            api.tx_lock_outputs(&slate.tx, lock_fn)?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn finalize_slate(&self, slate: &mut Slate, tx_proof: Option<&mut TxProof>) -> Result<()> {
        let wallet = self.get_wallet_instance()?;
        let mut should_post: bool = false;
        controller::owner_single_use(wallet.clone(), |api| {
            api.verify_slate_messages(&slate)?;
            Ok(())
        }).map_err(|_| {
            ErrorKind::GrinWalletVerifySlateMessagesError
        })?;
        controller::owner_single_use(wallet.clone(), |api| {
            should_post = api.finalize_tx(slate, tx_proof)?;
            Ok(())
        }).map_err(|_| {
            ErrorKind::GrinWalletFinalizeError
        })?;
        if should_post {
            controller::owner_single_use(wallet.clone(), |api| {
                api.post_tx(&slate.tx, false)?;
                Ok(())
            }).map_err(|_| {
                ErrorKind::GrinWalletPostError
            })?;
        }
        Ok(())
    }

    pub fn derive_address_key(&self, index: u32) -> Result<SecretKey> {
        let wallet = self.get_wallet_instance()?;
        let mut w = wallet.lock();
        w.open_with_credentials()?;
        derive_address_key(w.keychain(), index).map_err(|e| e.into())
    }

    pub fn verify_tx_proof(&self, tx_proof: &TxProof) -> Result<(String, u64, Vec<String>, String)> {
        let wallet = self.get_wallet_instance()?;
        let mut api = Wallet713OwnerAPI::new(wallet.clone());
        let (address, amount, outputs, excess_sum) = api.verify_tx_proof(tx_proof)?;

        let outputs = outputs
            .iter()
            .map(|o| grin_util::to_hex(o.0.to_vec()))
            .collect();

        Ok((address.public_key.clone(), amount, outputs, excess_sum.to_hex()))
    }

    fn init_seed(&self, wallet_config: &WalletConfig, passphrase: &str) -> Result<WalletSeed> {
        let result = WalletSeed::from_file(&wallet_config, passphrase);
        let seed = match result {
            Ok(seed) => seed,
            Err(_) => {
                // could not load from file, let's create a new one
                WalletSeed::init_file(&wallet_config, 32, None, passphrase)?
            }
        };
        Ok(seed)
    }

    fn get_wallet_instance(&self) -> Result<Arc<Mutex<WalletInst<impl NodeClient + 'static, ExtKeychain>>>> {
        if let Some(ref backend) = self.backend {
            Ok(backend.clone())
        } else {
            Err(ErrorKind::NoWallet)?
        }
    }

    fn create_wallet_instance(&mut self, config: &Wallet713Config, account: &str, passphrase: &str) -> Result<()> {
        let wallet_config = config.as_wallet_config()?;
        let node_client = HTTPNodeClient::new(&wallet_config.check_node_api_http_addr, config.grin_node_secret().clone());
        let _ = WalletSeed::from_file(&wallet_config, passphrase)?;
        let mut db_wallet = Backend::new(&wallet_config, passphrase, node_client)?;
        db_wallet.set_parent_key_id_by_name(account)?;
        self.backend = Some(Arc::new(Mutex::new(db_wallet)));
        Ok(())
    }

    fn init_backend(&self, wallet_config: &WalletConfig, wallet713_config: &Wallet713Config, passphrase: &str) -> Result<Backend<HTTPNodeClient, ExtKeychain>> {
        let node_api_client = HTTPNodeClient::new(&wallet_config.check_node_api_http_addr, wallet713_config.grin_node_secret().clone());
        let backend = Backend::new(wallet_config, passphrase, node_api_client)?;
        Ok(backend)
    }
}
