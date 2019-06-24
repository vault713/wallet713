use failure::Error;
use futures::sync::oneshot;
use grin_keychain::{ExtKeychain, Keychain};
use std::marker::PhantomData;
use std::thread::JoinHandle;
use crate::broker::{GrinboxPublisher, KeybasePublisher, GrinboxSubscriber, KeybaseSubscriber};
use crate::common::config::Wallet713Config;
use crate::common::{Arc, Mutex};
use crate::contacts::{AddressBook, GrinboxAddress, KeybaseAddress};
use crate::wallet::backend::Backend;
use crate::wallet::types::{HTTPNodeClient, NodeClient, WalletBackend};
use super::ErrorKind;

pub struct Container<W, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    pub config: Wallet713Config,
    backend: W,
    pub address_book: Option<AddressBook>,
    pub account: String,
    pub grinbox: Option<(GrinboxAddress, GrinboxPublisher, GrinboxSubscriber, JoinHandle<()>)>,
    pub keybase: Option<(KeybaseAddress, KeybasePublisher, KeybaseSubscriber)>,
    pub foreign_http: Option<(oneshot::Sender<()>, JoinHandle<()>)>,
    pub owner_http: Option<JoinHandle<()>>,
    phantom_c: PhantomData<C>,
    phantom_k: PhantomData<K>,
}

impl<W, C, K> Container<W, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    pub fn new(config: Wallet713Config, backend: W, address_book: Option<AddressBook>) -> Self {
        Self {
            config,
            backend,
            address_book,
            account: String::from("default"),
            grinbox: None,
            keybase: None,
            foreign_http: None,
            owner_http: None,
            phantom_c: PhantomData,
            phantom_k: PhantomData,
        }
    }

    pub fn raw_backend(&mut self) -> &mut W {
        &mut self.backend
    }

    pub fn backend(&mut self) -> Result<&mut W, Error> {
        if !self.backend.connected()? {
            return Err(ErrorKind::NoBackend.into());
        }
        Ok(&mut self.backend)
    }
}

pub fn create_container(config: Wallet713Config, address_book: Option<AddressBook>) -> Result<Arc<Mutex<Container<Backend<HTTPNodeClient, ExtKeychain>, HTTPNodeClient, ExtKeychain>>>, Error> {
    let wallet_config = config.as_wallet_config()?;
    let client = HTTPNodeClient::new(
        &wallet_config.check_node_api_http_addr,
        config.grin_node_secret().clone(),
    );
    let backend = Backend::new(&wallet_config, client)?;
    let container = Container::new(config, backend, address_book);
    Ok(Arc::new(Mutex::new(container)))
}