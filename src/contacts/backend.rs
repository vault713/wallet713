// Copyright 2019 The vault713 Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::types::{parse_address, AddressBookBackend, AddressBookBatch, Contact};
use crate::common::Error;
use epic_core::ser::Error as CoreError;
use epic_core::ser::{Readable, Reader, Writeable, Writer};
use epic_store::Store;
use epic_store::{self, to_key};
use serde_json::json;
use std::cell::RefCell;
use std::fs::create_dir_all;
use std::path::Path;

const DB_DIR: &'static str = "contacts";
const CONTACT_PREFIX: u8 = 'X' as u8;

pub struct Backend {
	db: Store,
}

impl Backend {
	pub fn new(data_path: &str) -> Result<Self, Error> {
		let db_path = Path::new(data_path).join(DB_DIR);
		create_dir_all(&db_path)?;

		let store = Store::new(db_path.to_str().unwrap(), None, Some(DB_DIR), None)?;

		let res = Backend { db: store };
		Ok(res)
	}
}

impl AddressBookBackend for Backend {
	fn get_contact(&self, name: &[u8]) -> Result<Option<Contact>, Error> {
		let contact_key = to_key(CONTACT_PREFIX, &mut name.to_vec());
		let contact = self.db.get_ser(&contact_key)?;
		Ok(contact)
	}

	fn contacts(&self) -> Box<dyn Iterator<Item = Contact>> {
		Box::new(self.db.iter(&[CONTACT_PREFIX]).unwrap().map(|x| x.1))
	}

	fn batch<'a>(&'a self) -> Result<Box<dyn AddressBookBatch + 'a>, Error> {
		let batch = self.db.batch()?;
		let batch = Batch {
			_store: self,
			db: RefCell::new(Some(batch)),
		};
		Ok(Box::new(batch))
	}
}

pub struct Batch<'a> {
	_store: &'a Backend,
	db: RefCell<Option<epic_store::Batch<'a>>>,
}

impl<'a> AddressBookBatch for Batch<'a> {
	fn save_contact(&mut self, contact: &Contact) -> Result<(), Error> {
		let mut key = contact.name.to_string().into_bytes();
		let contact_key = to_key(CONTACT_PREFIX, &mut key);
		self.db
			.borrow()
			.as_ref()
			.unwrap()
			.put_ser(&contact_key, contact)?;
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

	fn commit(&mut self) -> Result<(), Error> {
		let db = self.db.replace(None);
		db.unwrap().commit()?;
		Ok(())
	}
}

impl Writeable for Contact {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), CoreError> {
		let json = json!({
			"name": self.name,
			"address": self.address.to_string(),
		});
		writer.write_bytes(&json.to_string().as_bytes())
	}
}

impl Readable for Contact {
	fn read(reader: &mut dyn Reader) -> Result<Contact, CoreError> {
		let data = reader.read_bytes_len_prefix()?;
		let data = std::str::from_utf8(&data).map_err(|_| CoreError::CorruptedData)?;

		let json: serde_json::Value =
			serde_json::from_str(&data).map_err(|_| CoreError::CorruptedData)?;

		let address = parse_address(json["address"].as_str().unwrap())
			.map_err(|_| CoreError::CorruptedData)?;

		let contact = Contact::new(json["name"].as_str().unwrap(), address)
			.map_err(|_| CoreError::CorruptedData)?;

		Ok(contact)
	}
}
