use std::sync::Arc;

use grin_wallet::HTTPNodeClient;
use grin_keychain::keychain::ExtKeychain;

use common::config::Wallet713Config;
use common::error::Error;
use common::types::Contact;
use common::crypto::{PublicKey, Base58};
use storage::lmdb::{LMDBBackend, Wallet713Backend};

pub struct AddressBook {
    backend: LMDBBackend<HTTPNodeClient, ExtKeychain>
}

impl AddressBook {
    pub fn new(password: &str) -> Result<Self, Error> {
        let config = Wallet713Config::from_file()?;
        let wallet_config = config.as_wallet_config()?;
        let node_api_client = HTTPNodeClient::new(&wallet_config.check_node_api_http_addr, config.grin_node_secret.clone());
        let backend = LMDBBackend::new(wallet_config.clone(), &password, node_api_client)?;
        let address_book = Self {
            backend
        };
        Ok(address_book)
    }

    pub fn add_contact(&mut self, contact: &Contact) -> Result<(), Error> {
        let key = PublicKey::from_base58_check(&contact.public_key, 2).map_err(|_| {
            Error::generic("invalid public key given!")
        })?;
        let mut batch = self.backend.wallet713_batch()?;
        batch.save_contact(contact)?;
        batch.commit()?;
        Ok(())
    }

    pub fn get_contact(&mut self, public_key: &str) -> Result<Contact, Error> {
        let contact = self.backend.get_contact(public_key.as_bytes())?;
        Ok(contact)
    }

    pub fn get_contact_by_name(&mut self, name: &str) -> Result<Contact, Error> {
        for contact in self.contact_iter() {
            if contact.name == name {
                return Ok(contact);
            }
        }
        Err(Error::generic(&format!("could not find contact named {}!", name)))
    }

    pub fn contact_iter(&self) -> Box<Iterator<Item = Contact>> {
        self.backend.contact_iter()
    }
}
