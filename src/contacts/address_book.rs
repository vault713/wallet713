use common::config::Wallet713Config;
use common::{Wallet713Error, Result};
use common::types::Contact;
use common::crypto::{PublicKey, Base58};
use storage::lmdb::{LMDBBackend, Wallet713Backend};

pub struct AddressBook {
    backend: LMDBBackend
}

impl AddressBook {
    pub fn new(config: &Wallet713Config) -> Result<Self> {
        let backend = LMDBBackend::new(&config)?;
        let address_book = Self {
            backend
        };
        Ok(address_book)
    }

    pub fn add_contact(&mut self, contact: &Contact) -> Result<()> {
        PublicKey::from_base58_check(&contact.public_key, 2).map_err(|_| {
            Wallet713Error::InvalidContactPublicKey(contact.public_key.clone())
        })?;

        let result = self.get_contact_by_name(&contact.name);
        if result.is_ok() {
            return Err(Wallet713Error::ContactAlreadyExists(contact.name.clone()))?;
        }

        let mut batch = self.backend.batch()?;
        batch.save_contact(contact)?;
        batch.commit()?;
        Ok(())
    }

    pub fn remove_contact(&mut self, public_key: &str) -> Result<()> {
        let mut batch = self.backend.batch()?;
        batch.delete_contact(public_key.as_bytes())?;
        batch.commit()?;
        Ok(())
    }

    pub fn remove_contact_by_name(&mut self, name: &str) -> Result<()> {
        let contact = self.get_contact_by_name(name)?;
        self.remove_contact(&contact.public_key)
    }

    pub fn get_contact(&mut self, public_key: &str) -> Result<Contact> {
        let contact = self.backend.get_contact(public_key.as_bytes())?;
        Ok(contact)
    }

    pub fn get_contact_by_name(&mut self, name: &str) -> Result<Contact> {
        for contact in self.contact_iter() {
            if contact.name == name {
                return Ok(contact);
            }
        }
        Err(Wallet713Error::ContactNotFound(name.to_string()))?
    }

    pub fn contact_iter(&self) -> Box<Iterator<Item = Contact>> {
        self.backend.contact_iter()
    }
}
