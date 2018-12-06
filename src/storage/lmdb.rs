use std::sync::Arc;
use std::cell::RefCell;
use std::path::Path;
use std::fs::{create_dir_all};
use grin_store::{self, option_to_not_found, to_key};

use common::error::Error;
use common::types::Contact;
use common::config::Wallet713Config;

pub const DB_DIR: &'static str = "contacts";

const CONTACT_PREFIX: u8 = 'X' as u8;

pub trait Wallet713Backend {
    fn get_contact(&mut self, public_key: &[u8]) -> Result<Contact, Error>;
    fn contact_iter(&self) -> Box<Iterator<Item = Contact>>;
    fn batch<'a>(&'a mut self) -> Result<Box<Wallet713Batch + 'a>, Error>;
}

pub trait Wallet713Batch {
    fn save_contact(&mut self, contact: &Contact) -> Result<(), Error>;
    fn delete_contact(&mut self, public_key: &[u8]) -> Result<(), Error>;
    fn commit(&self) -> Result<(), Error>;
}

pub struct LMDBBackend {
    db: grin_store::Store,
}

impl LMDBBackend {
    pub fn new(config: &Wallet713Config) -> Result<Self, Error> {
        let db_path = Path::new(&config.wallet713_data_path).join(DB_DIR);
        create_dir_all(&db_path).expect("Couldn't create wallet backend directory!");

        let lmdb_env = Arc::new(grin_store::new_env(db_path.to_str().unwrap().to_string()));
        let store = grin_store::Store::open(lmdb_env, DB_DIR);

        let res = LMDBBackend {
            db: store,
        };
        Ok(res)
    }
}

impl Wallet713Backend for LMDBBackend {
    fn get_contact(&mut self, public_key: &[u8]) -> Result<Contact, Error> {
        let contact_key = to_key(CONTACT_PREFIX, &mut public_key.to_vec());
        option_to_not_found(
            self.db.get_ser(&contact_key),
            &format!("Contact id: {:x?}", public_key.to_vec()),
        ).map_err(|e| e.into())
    }

    fn contact_iter(&self) -> Box<Iterator<Item = Contact>> {
        Box::new(self.db.iter(&[CONTACT_PREFIX]).unwrap())
    }


    fn batch<'a>(&'a mut self) -> Result<Box<Wallet713Batch + 'a>, Error>
    {
        Ok(Box::new(Batch {
            _store: self,
            db: RefCell::new(Some(self.db.batch()?)),
        }))
    }
}


/// An atomic batch in which all changes can be committed all at once or
/// discarded on error.
pub struct Batch<'a> {
    _store: &'a LMDBBackend,
    db: RefCell<Option<grin_store::Batch<'a>>>,
}

impl<'a> Wallet713Batch for Batch<'a> {
    fn save_contact(&mut self, contact: &Contact) -> Result<(), Error> {
        let mut key = contact.public_key.clone().into_bytes();
        let contact_key = to_key(CONTACT_PREFIX, &mut key);
        self.db.borrow().as_ref().unwrap().put_ser(&contact_key, contact)?;
        Ok(())
    }

    fn delete_contact(&mut self, public_key: &[u8]) -> Result<(), Error> {
        let ctx_key = to_key(CONTACT_PREFIX, &mut public_key.to_vec());
        self.db
            .borrow()
            .as_ref()
            .unwrap()
            .delete(&ctx_key)
            .map_err(|e| e.into())
    }

    fn commit(&self) -> Result<(), Error> {
        let db = self.db.replace(None);
        db.unwrap().commit()?;
        Ok(())
    }
}
