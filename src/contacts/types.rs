use std::fmt::{self, Display, Debug};
use regex::Regex;

use common::{Error, Wallet713Error};
use common::crypto::{PublicKey, Base58};

const ADDRESS_REGEX: &str = r"^((?P<address_type>keybase|grinbox)://).+$";
const GRINBOX_ADDRESS_REGEX: &str = r"^(grinbox://)?(?P<public_key>[123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz]{52})(@(?P<domain>[a-zA-Z0-9\.]+)(:(?P<port>[0-9]*))?)?$";
const KEYBASE_ADDRESS_REGEX: &str = r"^(keybase://)?(?P<username>[0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_]{1,16})$";
const DEFAULT_GRINBOX_DOMAIN: &str = "grinbox.io";
const DEFAULT_GRINBOX_PORT: u16 = 13420;

#[derive(PartialEq)]
pub enum AddressType {
    Grinbox,
    Keybase,
}
pub trait Address: Debug + Display {
    fn from_str(s: &str) -> Result<Self, Error> where Self: Sized;
    fn address_type(&self) -> AddressType;
    fn stripped(&self) -> String;
}

impl Address {
    pub fn parse(address: &str) -> Result<Box<Address>, Error> {
        let re = Regex::new(ADDRESS_REGEX)?;
        let captures = re.captures(address);
        if captures.is_none() {
            Err(Wallet713Error::MissingAddressType(address.to_string()))?;
        }
        let captures = captures.unwrap();
        let address_type = captures.name("address_type").unwrap().as_str().to_string();
        let address: Box<Address> = match address_type.as_ref() {
            "keybase" => Box::new(KeybaseAddress::from_str(address)?),
            "grinbox" => Box::new(GrinboxAddress::from_str(address)?),
            x => Err(Wallet713Error::UnknownAddressType(x.to_string()))?,
        };
        Ok(address)
    }
}

pub trait AddressBookBackend {
    fn get_contact(&mut self, name: &[u8]) -> Result<Contact, Error>;
    fn contact_iter(&self) -> Box<Iterator<Item=Contact>>;
    fn batch<'a>(&'a mut self) -> Result<Box<AddressBookBatch + 'a>, Error>;
}

pub trait AddressBookBatch {
    fn save_contact(&mut self, contact: &Contact) -> Result<(), Error>;
    fn delete_contact(&mut self, public_key: &[u8]) -> Result<(), Error>;
    fn commit(&self) -> Result<(), Error>;
}

pub struct AddressBook {
    backend: Box<AddressBookBackend + Send>
}

impl AddressBook {
    pub fn new(backend: Box<AddressBookBackend + Send>) -> Result<Self, Error> {
        let address_book = Self {
            backend
        };
        Ok(address_book)
    }

    pub fn add_contact(&mut self, contact: &Contact) -> Result<(), Error> {
        let result = self.get_contact(&contact.name);
        if result.is_ok() {
            return Err(Wallet713Error::ContactAlreadyExists(contact.name.clone()))?;
        }
        let mut batch = self.backend.batch()?;
        batch.save_contact(contact)?;
        batch.commit()?;
        Ok(())
    }

    pub fn remove_contact(&mut self, name: &str) -> Result<(), Error> {
        let mut batch = self.backend.batch()?;
        batch.delete_contact(name.as_bytes())?;
        batch.commit()?;
        Ok(())
    }

    pub fn get_contact(&mut self, name: &str) -> Result<Contact, Error> {
        let contact = self.backend.get_contact(name.as_bytes())?;
        Ok(contact)
    }

    pub fn get_contact_by_address(&mut self, address: &str) -> Result<Contact, Error> {
        for contact in self.contact_iter() {
            if contact.address.to_string() == address {
                return Ok(contact);
            }
        }
        Err(Wallet713Error::ContactNotFound(address.to_string()))?
    }

    pub fn contact_iter(&self) -> Box<Iterator<Item=Contact>> {
        self.backend.contact_iter()
    }
}

#[derive(Debug)]
pub struct Contact {
    name: String,
    address: Box<Address>
}

impl Contact {
    pub fn new(name: &str, address: Box<Address>) -> Result<Self, Error> {
        Ok(Self {
            name: name.to_string(),
            address,
        })
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_address(&self) -> &Box<Address> {
        &self.address
    }
}

impl Display for Contact {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.address.to_string())?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeybaseAddress {
    pub username: String,
}

impl Address for KeybaseAddress {
    fn from_str(s: &str) -> Result<Self, Error> {
        let re = Regex::new(KEYBASE_ADDRESS_REGEX).unwrap();
        let captures = re.captures(s);
        if captures.is_none() {
            Err(Wallet713Error::KeybaseAddressParsingError(s.to_string()))?;
        }

        let captures = captures.unwrap();
        let username = captures.name("username").unwrap().as_str().to_string();
        Ok(Self {
            username
        })
    }

    fn address_type(&self) -> AddressType {
        AddressType::Keybase
    }

    fn stripped(&self) -> String {
        format!("{}", self.username)
    }
}

impl Display for KeybaseAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "keybase://{}", self.username)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GrinboxAddress {
    pub public_key: String,
    pub domain: String,
    pub port: u16,
}

impl Address for GrinboxAddress {
    fn from_str(s: &str) -> Result<Self, Error> {
        let re = Regex::new(GRINBOX_ADDRESS_REGEX).unwrap();
        let captures = re.captures(s);
        if captures.is_none() {
            Err(Wallet713Error::GrinboxAddressParsingError(s.to_string()))?;
        }

        let captures = captures.unwrap();
        let public_key = captures.name("public_key").unwrap().as_str().to_string();
        let domain = captures.name("domain").map(|m| m.as_str().to_string()).unwrap_or(DEFAULT_GRINBOX_DOMAIN.to_string());
        let port = captures.name("port").map(|m| u16::from_str_radix(m.as_str(), 10).unwrap()).unwrap_or(DEFAULT_GRINBOX_PORT);

        PublicKey::from_base58_check(&public_key, 2).map_err(|_| {
            Wallet713Error::InvalidContactPublicKey(public_key.clone())
        })?;

        Ok(Self {
            public_key,
            domain,
            port
        })
    }

    fn address_type(&self) -> AddressType {
        AddressType::Grinbox
    }

    fn stripped(&self) -> String {
        format!("{}", self)[10..].to_string()
    }
}

impl Display for GrinboxAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "grinbox://{}", self.public_key)?;
        if self.domain != DEFAULT_GRINBOX_DOMAIN || self.port != DEFAULT_GRINBOX_PORT {
            write!(f, "@{}", self.domain)?;
            if self.port != DEFAULT_GRINBOX_PORT {
                write!(f, ":{}", self.port)?;
            }
        }
        Ok(())
    }
}

impl GrinboxAddress {
    pub fn local_to(&self, other: &GrinboxAddress) -> bool {
        self.domain == other.domain && self.port == other.port
    }
}
