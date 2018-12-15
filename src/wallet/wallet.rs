use std::sync::Arc;

use grin_util::Mutex;
use grin_wallet::{display, controller, instantiate_wallet, WalletInst, WalletConfig, WalletSeed, HTTPNodeClient, NodeClient};
use grin_wallet::lmdb_wallet::LMDBBackend;
use grin_core::libtx::slate::Slate;
use grin_keychain::keychain::ExtKeychain;

use common::{Wallet713Error, Result};
use common::config::Wallet713Config;

pub struct Wallet {}

impl Wallet {
    pub fn new() -> Self {
        Self {}
    }

    pub fn init(&self, password: &str) -> Result<WalletSeed> {
        let config = Wallet713Config::from_file()?;
        let wallet_config = config.as_wallet_config()?;
        let seed = self.init_seed(&wallet_config, password)?;
        self.init_backend(&wallet_config, &config, password)?;
        Ok(seed)
    }

    pub fn info(&self, password: &str, account: &str) -> Result<()> {
        let wallet = self.get_wallet_instance(password)?;
        let result = controller::owner_single_use(wallet.clone(), |api| {
            let (validated, wallet_info) = api.retrieve_summary_info(true, 10)?;
            display::info(
                account,
                &wallet_info,
                validated,
                true,
            );
            Ok(())
        })?;
        Ok(result)
    }

    pub fn txs(&self, password: &str, account: &str) -> Result<()> {
        let wallet = self.get_wallet_instance(password)?;
        let result = controller::owner_single_use(wallet.clone(), |api| {
            let (height, _) = api.node_height()?;
            let (validated, txs) = api.retrieve_txs(true, None, None)?;
            display::txs(
                account,
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

    pub fn outputs(&self, password: &str, account: &str, show_spent: bool) -> Result<()> {
        let wallet = self.get_wallet_instance(password)?;
        let result = controller::owner_single_use(wallet.clone(), |api| {
            let (height, _) = api.node_height()?;
            let (validated, outputs) = api.retrieve_outputs(show_spent, true, None)?;
            display::outputs(
                account,
                height,
                validated,
                outputs,
                true,
            )?;
            Ok(())
        })?;
        Ok(result)
    }

    pub fn initiate_send_tx(&mut self, password: &str, account: &str, amount: u64, minimum_confirmations: u64, selection_strategy: &str, change_outputs: usize, max_outputs: usize) -> Result<Slate> {
        let wallet = self.get_wallet_instance(password)?;
        let mut s: Slate = Slate::blank(0);
        controller::owner_single_use(wallet.clone(), |api| {
            let (slate, lock_fn) = api.initiate_tx(
                Some(account),
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

    pub fn initiate_receive_tx(&mut self, password: &str, account: &str, amount: u64) -> Result<Slate> {
        let wallet = self.get_wallet_instance(password)?;
        let mut api = super::api::Wallet713ForeignAPI::new(wallet.clone());
        let (slate, add_fn) = api.initiate_receive_tx(
            Some(account),
            amount,
            None,
        )?;
        api.tx_add_outputs(&slate, add_fn)?;
        Ok(slate)
    }

    pub fn repost(&self, password: &str, id: u32, fluff: bool) -> Result<()> {
        let wallet = self.get_wallet_instance(password)?;
        controller::owner_single_use(wallet.clone(), |api| {
            let (_, txs) = api.retrieve_txs(true, Some(id), None)?;
            if txs.len() == 0 {
                return Err(grin_wallet::libwallet::ErrorKind::GenericError(
                    format!("could not find transaction with id {}!", id)
                ))?
            }

            let tx = api.get_stored_tx(&txs[0])?;
            if tx.is_none() {
                return Err(grin_wallet::libwallet::ErrorKind::GenericError(
                    format!("no transaction data stored for id {}, can not repost!", id)
                ))?
            }

            api.post_tx(&tx.unwrap(), fluff)?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn cancel(&self, password: &str, id: u32) -> Result<()> {
        let wallet = self.get_wallet_instance(password)?;
        controller::owner_single_use(wallet.clone(), |api| {
            api.cancel_tx(Some(id), None)
        })?;
        Ok(())
    }

    pub fn restore(&self, password: &str) -> Result<()> {
        let wallet = self.get_wallet_instance(password)?;
        controller::owner_single_use(wallet.clone(), |api| {
            api.restore()
        })?;
        Ok(())
    }

    pub fn process_slate(&self, account: &str, password: &str, slate: &mut Slate) -> Result<bool> {
        let wallet = self.get_wallet_instance(password)?;
        let is_finalized = if slate.num_participants > slate.participant_data.len() {
            if slate.fee != 0 {
                controller::foreign_single_use(wallet.clone(), |api| {
                    api.receive_tx(slate, Some(account), None)?;
                    Ok(())
                }).map_err(|_| {
                    Wallet713Error::GrinWalletReceiveError
                })?;
            } else {
                let mut api = super::api::Wallet713OwnerAPI::new(wallet.clone());
                let lock_fn = api.invoice_tx(
                    Some(account),
                    slate,
                    10,
                    500,
                    1,
                    true,
                    None,
                )?;
                controller::owner_single_use(wallet.clone(), |api| {
                    api.tx_lock_outputs(&slate, lock_fn)?;
                    Ok(())
                })?;
            };
            false
        } else {
            controller::owner_single_use(wallet.clone(), |api| {
                api.finalize_tx(slate)?;
                Ok(())
            }).map_err(|_| {
                Wallet713Error::GrinWalletFinalizeError
            })?;
            controller::owner_single_use(wallet.clone(), |api| {
                api.post_tx(&slate.tx, false)?;
                Ok(())
            }).map_err(|e| {
                println!("{:?}", e);
                Wallet713Error::GrinWalletPostError
            })?;
            true
        };
        Ok(is_finalized)
    }

    fn init_seed(&self, wallet_config: &WalletConfig, password: &str) -> Result<WalletSeed> {
        let result = WalletSeed::from_file(&wallet_config, password);
        match result {
            Err(_) => {
                // could not load from file, let's create a new one
                let seed = WalletSeed::init_file(&wallet_config, 32, password)?;
                if password.is_empty() {
                    cli_message!("{}: wallet with no password.", "WARNING".bright_yellow());
                };
                Ok(seed)
            }
            Ok(seed) => {
                cli_message!("{}: seed file already exists.", "WARNING".bright_yellow());
                Ok(seed)
            }
        }
    }

    fn init_backend(&self, wallet_config: &WalletConfig, wallet713_config: &Wallet713Config, password: &str) -> Result<LMDBBackend<HTTPNodeClient, ExtKeychain>> {
        let node_api_client = HTTPNodeClient::new(&wallet_config.check_node_api_http_addr, wallet713_config.grin_node_secret.clone());

        let backend = LMDBBackend::new(wallet_config.clone(), &password, node_api_client)?;
        Ok(backend)
    }

    fn get_wallet_instance(&self, password: &str) -> Result<Arc<Mutex<WalletInst<impl NodeClient + 'static, ExtKeychain>>>> {
        let config = Wallet713Config::from_file()?;
        let wallet_config = config.as_wallet_config()?;
        let node_client = HTTPNodeClient::new(&wallet_config.check_node_api_http_addr, config.grin_node_secret.clone());
        let wallet = instantiate_wallet(
            wallet_config,
            node_client,
            password,
            "default",
        ).map_err(|_| {
            Wallet713Error::NoWallet
        })?;
        Ok(wallet)
    }
}
