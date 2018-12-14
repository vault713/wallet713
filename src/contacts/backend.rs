use std::sync::Arc;
use std::cell::RefCell;
use std::path::Path;
use std::fs::{create_dir_all};

use grin_store::{self, option_to_not_found, to_key};
use grin_core::ser::{Readable, Writeable, Reader, Writer};
use grin_core::ser::Error as CoreError;

use super::types::{Address, Contact, AddressBookBackend, AddressBookBatch};
use common::Error;

const DB_DIR: &'static str = "contacts";
const CONTACT_PREFIX: u8 = 'X' as u8;

pub struct LMDBBackend {
    db: grin_store::Store,
}

impl LMDBBackend {
    pub fn new(data_path: &str) -> Result<Self, Error> {
        let db_path = Path::new(data_path).join(DB_DIR);
        create_dir_all(&db_path)?;

        let lmdb_env = Arc::new(grin_store::new_env(db_path.to_str().unwrap().to_string()));
        let store = grin_store::Store::open(lmdb_env, DB_DIR);

        let res = LMDBBackend {
            db: store,
        };
        Ok(res)
    }
}

impl AddressBookBackend for LMDBBackend {
    fn get_contact(&mut self, name: &[u8]) -> Result<Contact, Error> {
        let contact_key = to_key(CONTACT_PREFIX, &mut name.to_vec());
        option_to_not_found(
            self.db.get_ser(&contact_key),
            &format!("Contact id: {:x?}", name.to_vec()),
        ).map_err(|e| e.into())
    }

    fn contact_iter(&self) -> Box<Iterator<Item = Contact>> {
        Box::new(self.db.iter(&[CONTACT_PREFIX]).unwrap())
    }


    fn batch<'a>(&'a mut self) -> Result<Box<AddressBookBatch + 'a>, Error>
    {
        let batch = self.db.batch()?;
        let batch = Batch {
            _store: self,
            db: RefCell::new(Some(batch))
        };
        Ok(Box::new(batch))
    }
}

pub struct Batch<'a> {
    _store: &'a LMDBBackend,
    db: RefCell<Option<grin_store::Batch<'a>>>,
}

impl<'a> AddressBookBatch for Batch<'a> {
    fn save_contact(&mut self, contact: &Contact) -> Result<(), Error> {
        let mut key = contact.get_name().to_string().into_bytes();
        let contact_key = to_key(CONTACT_PREFIX, &mut key);
        self.db.borrow().as_ref().unwrap().put_ser(&contact_key, contact)?;
        Ok(())
    }

    fn delete_contact(&mut self, name: &[u8]) -> Result<(), Error> {
        let ctx_key = to_key(CONTACT_PREFIX, &mut name.to_vec());
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

impl Writeable for Contact {
    fn write<W: Writer>(&self, writer: &mut W) -> Result<(), CoreError> {
        let json = json!({
            "name": self.get_name(),
            "address": self.get_address().to_string(),
        });
        writer.write_bytes(&json.to_string().as_bytes())
    }
}

impl Readable for Contact {
    fn read(reader: &mut Reader) -> Result<Contact, CoreError> {
        let data = reader.read_bytes_len_prefix()?;
        let data = std::str::from_utf8(&data)
            .map_err(|_| { CoreError::CorruptedData })?;

        let json: serde_json::Value = serde_json::from_str(&data)
            .map_err(|_| { CoreError::CorruptedData })?;

        let address = Address::parse(json["address"].as_str().unwrap())
            .map_err(|_| { CoreError::CorruptedData })?;

        let contact = Contact::new(json["name"].as_str().unwrap(), address)
            .map_err(|_| { CoreError::CorruptedData })?;

        Ok(contact)
    }
}