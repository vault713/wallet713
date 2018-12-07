use std::sync::Arc;

use grin_util::Mutex;

use common::config::Wallet713Config;
use grinbox::protocol::ProtocolResponse;
use grinbox::client::{GrinboxClient, GrinboxClientHandler, GrinboxClientOut};
use common::{Wallet713Error, Result};
use common::crypto::{PublicKey, Base58};

use grin_wallet::{display, controller, instantiate_wallet, WalletInst, WalletConfig, WalletSeed, HTTPNodeClient, LMDBBackend};
use grin_core::libtx::slate::Slate;
use grin_keychain::keychain::ExtKeychain;
use grin_core::core;
use contacts::AddressBook;

pub struct Wallet {
    pub client: GrinboxClient,
    pub address_book: Arc<std::sync::Mutex<AddressBook>>,
}

impl Wallet {
    pub fn new(address_book: Arc<std::sync::Mutex<AddressBook>>) -> Self {
        Wallet {
            client: GrinboxClient::new(),
            address_book,
        }
    }

    pub fn init(&self, password: &str) -> Result<WalletSeed> {
        let config = Wallet713Config::from_file()?;
        let wallet_config = config.as_wallet_config()?;
        let seed = self.init_seed(&wallet_config, password)?;
        self.init_backend(&wallet_config, &config, password)?;
        Ok(seed)
    }

    pub fn start_client(&mut self, password: &str, grinbox_uri: &str, grinbox_private_key: &str) -> Result<()> {
        if !self.client.is_started() {
            let wallet = self.get_wallet_instance(password)?;
            let address_book = self.address_book.clone();
            let handler = Box::new(MessageHandler {
                wallet,
                address_book,
            });
            self.client.start(grinbox_uri, grinbox_private_key, handler)?;
            Ok(())
        } else {
            let public_key = self.client.get_listening_address().unwrap_or("...".to_owned());
            Err(Wallet713Error::AlreadyListening(public_key))?
        }
    }

    pub fn stop_client(&self) -> Result<()> {
        self.client.stop()
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

    pub fn subscribe(&self) -> Result<()> {
        if !self.client.is_started() {
            Err(Wallet713Error::ClosedListener)?
        } else {
            self.client.subscribe()
        }
    }

    pub fn unsubscribe(&self) -> Result<()> {
        if !self.client.is_started() {
            Err(Wallet713Error::ClosedListener)?
        } else {
            self.client.unsubscribe()
        }
    }

    pub fn send(&mut self, password: &str, account: &str, to: &str, amount: u64, minimum_confirmations: u64, selection_strategy: &str, change_outputs: usize, max_outputs: usize) -> Result<Slate> {
        if !self.client.is_started() {
            Err(Wallet713Error::ClosedListener)?
        } else {
            let mut to = to.to_string();
            if to.starts_with("@") {
                let mut guard = self.address_book.lock().unwrap();
                let contact = guard.get_contact_by_name(&to[1..])?;
                to = contact.public_key.clone();
            }

            PublicKey::from_base58_check(&to, 2).map_err(|_| {
                Wallet713Error::InvalidPublicKey(to.clone())
            })?;

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

            self.client.post_slate(&to, &s)?;
            Ok(s)
        }
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

            let tx = txs[0].get_stored_tx();
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

    fn get_wallet_instance(&self, password: &str) -> Result<Arc<Mutex<WalletInst<HTTPNodeClient, ExtKeychain>>>> {
        let config = Wallet713Config::from_file()?;
        let wallet_config = config.as_wallet_config()?;
        let wallet = instantiate_wallet(
            wallet_config,
            password,
            "default",
            config.grin_node_secret.clone(),
        ).map_err(|_| {
            Wallet713Error::NoWallet
        })?;
        Ok(wallet)
    }
}

#[derive(Clone)]
struct MessageHandler {
    wallet: Arc<Mutex<WalletInst<HTTPNodeClient, ExtKeychain>>>,
    address_book: Arc<std::sync::Mutex<AddressBook>>,
}

impl MessageHandler {
    pub fn process_slate(&self, account: &str, slate: &mut Slate) -> Result<bool> {
        let is_finalized = if slate.num_participants > slate.participant_data.len() {
            controller::foreign_single_use(self.wallet.clone(), |api| {
                api.receive_tx(slate, Some(account), None)?;
                Ok(())
            })?;
            false
        } else {
            controller::owner_single_use(self.wallet.clone(), |api| {
                api.finalize_tx(slate)?;
                api.post_tx(&slate.tx, false)?;
                Ok(())
            })?;
            true
        };
        Ok(is_finalized)
    }
}

impl GrinboxClientHandler for MessageHandler {
    fn on_response(&self, response: &ProtocolResponse, out: &GrinboxClientOut) {
        match response {
            ProtocolResponse::Slate { from, str, challenge: _, signature: _ } => {
                let mut slate: Slate = serde_json::from_str(&str).unwrap();
                let mut guard = self.address_book.lock().unwrap();
                let mut display_from = from.clone();
                if let Ok(contact) = guard.get_contact(&from) {
                    display_from = format!("@{}", contact.name);
                }
                if slate.num_participants > slate.participant_data.len() {
                    cli_message!("slate [{}] received from [{}] for [{}] grins",
                             slate.id.to_string().bright_green(),
                             display_from.bright_green(),
                             core::amount_to_hr_string(slate.amount, false).bright_green()
                    );
                } else {
                    cli_message!("slate [{}] received back from [{}] for [{}] grins",
                             slate.id.to_string().bright_green(),
                             display_from.bright_green(),
                             core::amount_to_hr_string(slate.amount, false).bright_green()
                    );
                };
                let is_finalized = self.process_slate("", &mut slate).expect("failed processing slate!");

                if !is_finalized {
                    out.post_slate(&from, &slate).expect("failed posting slate!");
                    cli_message!("slate [{}] sent back to [{}] successfully",
                             slate.id.to_string().bright_green(),
                             display_from.bright_green()
                    );
                } else {
                    cli_message!("slate [{}] finalized successfully",
                             slate.id.to_string().bright_green()
                    );
                }
            },
            ProtocolResponse::Error { kind: _, description: _ } => {
                cli_message!("{}", response);
            },
            _ => {},
        };
    }

    fn on_close(&self, reason: &str) {
        cli_message!("{}: grinbox client closed with reason: {}", "WARNING".bright_yellow(), reason);
    }
}
