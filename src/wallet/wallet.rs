use std::sync::Arc;

use grin_util::Mutex;
use grin_wallet::{display, controller, WalletBackend, WalletInst, WalletConfig, WalletSeed, HTTPNodeClient, NodeClient};
use grin_wallet::lmdb_wallet::LMDBBackend;
use grin_core::libtx::slate::Slate;
use grin_keychain::keychain::ExtKeychain;

use common::{Wallet713Error, Result};
use common::config::Wallet713Config;

pub struct Wallet {
    active_account: String,
    backend: Option<Arc<Mutex<LMDBBackend<HTTPNodeClient, ExtKeychain>>>>,
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
            Wallet713Error::WalletUnlockFailed
        })?;
        Ok(())
    }

    pub fn lock(&mut self) {
        self.backend = None;
    }

    pub fn is_locked(&self) -> bool {
        self.backend.is_none()
    }

    pub fn init(&mut self, config: &Wallet713Config, account: &str, passphrase: &str) -> Result<WalletSeed> {
        let wallet_config = config.as_wallet_config()?;
        let seed = self.init_seed(&wallet_config, passphrase)?;
        self.init_backend(&wallet_config, &config, passphrase)?;
        self.unlock(config, account, passphrase)?;
        Ok(seed)
    }

    pub fn info(&self) -> Result<()> {
        let wallet = self.get_wallet_instance()?;
        let result = controller::owner_single_use(wallet.clone(), |api| {
            let (validated, wallet_info) = api.retrieve_summary_info(true, 10)?;
            display::info(
                &self.active_account,
                &wallet_info,
                validated,
                true,
            );
            Ok(())
        })?;
        Ok(result)
    }

    pub fn txs(&self) -> Result<()> {
        let wallet = self.get_wallet_instance()?;
        let result = controller::owner_single_use(wallet.clone(), |api| {
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
        Ok(result)
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

    pub fn initiate_send_tx(&self, amount: u64, minimum_confirmations: u64, selection_strategy: &str, change_outputs: usize, max_outputs: usize) -> Result<Slate> {
        let wallet = self.get_wallet_instance()?;
        let mut s: Slate = Slate::blank(0);
        controller::owner_single_use(wallet.clone(), |api| {
            let (slate, lock_fn) = api.initiate_tx(
                Some(&self.active_account),
                amount,
                minimum_confirmations,
                max_outputs,
                change_outputs,
                selection_strategy == "all",
                None,
            )?;
            api.tx_lock_outputs(&slate, lock_fn)?;
            s = slate;
            Ok(())
        })?;
        Ok(s)
    }

    pub fn initiate_receive_tx(&self, amount: u64, num_outputs: usize) -> Result<Slate> {
        let wallet = self.get_wallet_instance()?;
        let mut api = super::api::Wallet713ForeignAPI::new(wallet.clone());
        let (slate, add_fn) = api.initiate_receive_tx(
            Some(&self.active_account),
            amount,
            num_outputs,
            None,
        )?;
        api.tx_add_outputs(&slate, add_fn)?;
        Ok(slate)
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

            let stored_tx = api.get_stored_tx(&txs[0])?;
            if let Some(tx) = stored_tx {
                api.post_tx(&tx, fluff)?;
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

    pub fn restore_seed(&self, config: &Wallet713Config, words: &Vec<&str>, passphrase: &str) -> Result<()> {
        let wallet_config = config.as_wallet_config()?;
        WalletSeed::recover_from_phrase(&wallet_config, &words.join(" "), passphrase)?;
        Ok(())
    }

    pub fn process_sender_initiated_slate(&self, slate: &mut Slate) -> Result<()> {
        let wallet = self.get_wallet_instance()?;
        controller::foreign_single_use(wallet.clone(), |api| {
            api.receive_tx(slate, Some(&self.active_account), None)?;
            Ok(())
        }).map_err(|_| {
            Wallet713Error::GrinWalletReceiveError
        })?;
        Ok(())
    }

    pub fn process_receiver_initiated_slate(&self, slate: &mut Slate) -> Result<()> {
        // reject by default unless wallet is set to auto accept invoices under a certain threshold
        let max_auto_accept_invoice = self.max_auto_accept_invoice.ok_or(Wallet713Error::DoesNotAcceptInvoices)?;

        if slate.amount > max_auto_accept_invoice {
            Err(Wallet713Error::InvoiceAmountTooBig(slate.amount))?;
        }

        let wallet = self.get_wallet_instance()?;
        let mut api = super::api::Wallet713OwnerAPI::new(wallet.clone());
        let lock_fn = api.invoice_tx(
            Some(&self.active_account),
            slate,
            10,
            500,
            1,
            false,
            None,
        )?;
        controller::owner_single_use(wallet.clone(), |api| {
            api.tx_lock_outputs(&slate, lock_fn)?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn finalize_slate(&self, slate: &mut Slate) -> Result<()> {
        let wallet = self.get_wallet_instance()?;
        controller::owner_single_use(wallet.clone(), |api| {
            api.verify_slate_messages(&slate)?;
            Ok(())
        }).map_err(|_| {
            Wallet713Error::GrinWalletVerifySlateMessagesError
        })?;
        controller::owner_single_use(wallet.clone(), |api| {
            api.finalize_tx(slate)?;
            Ok(())
        }).map_err(|_| {
            Wallet713Error::GrinWalletFinalizeError
        })?;
        controller::owner_single_use(wallet.clone(), |api| {
            api.post_tx(&slate.tx, false)?;
            Ok(())
        }).map_err(|_| {
            Wallet713Error::GrinWalletPostError
        })?;
        Ok(())
    }

    fn init_seed(&self, wallet_config: &WalletConfig, passphrase: &str) -> Result<WalletSeed> {
        let result = WalletSeed::from_file(&wallet_config, passphrase);
        let seed = match result {
            Ok(seed) => seed,
            Err(_) => {
                // could not load from file, let's create a new one
                WalletSeed::init_file(&wallet_config, 32, passphrase)?
            }
        };
        Ok(seed)
    }

    fn init_backend(&self, wallet_config: &WalletConfig, wallet713_config: &Wallet713Config, passphrase: &str) -> Result<LMDBBackend<HTTPNodeClient, ExtKeychain>> {
        let node_api_client = HTTPNodeClient::new(&wallet_config.check_node_api_http_addr, wallet713_config.grin_node_secret.clone());
        let backend = LMDBBackend::new(wallet_config.clone(), passphrase, node_api_client)?;
        Ok(backend)
    }

    fn get_wallet_instance(&self) -> Result<Arc<Mutex<WalletInst<impl NodeClient + 'static, ExtKeychain>>>> {
        if let Some(ref backend) = self.backend {
            Ok(backend.clone())
        } else {
            Err(Wallet713Error::NoWallet)?
        }
    }

    fn create_wallet_instance(&mut self, config: &Wallet713Config, account: &str, passphrase: &str) -> Result<()> {
        let wallet_config = config.as_wallet_config()?;
        let node_client = HTTPNodeClient::new(&wallet_config.check_node_api_http_addr, config.grin_node_secret.clone());
        let _ = WalletSeed::from_file(&wallet_config, passphrase)?;
        let mut db_wallet = LMDBBackend::new(wallet_config.clone(), passphrase, node_client)?;
        db_wallet.set_parent_key_id_by_name(account)?;
        self.backend = Some(Arc::new(Mutex::new(db_wallet)));
        Ok(())
    }
}
