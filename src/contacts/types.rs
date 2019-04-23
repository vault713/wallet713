use regex::Regex;
use std::fmt::{self, Debug, Display};
use url::Url;

use grin_core::global::is_mainnet;

use common::crypto::{
    Base58, PublicKey, GRINBOX_ADDRESS_VERSION_MAINNET, GRINBOX_ADDRESS_VERSION_TESTNET,
};
use common::{ErrorKind, Result};

const ADDRESS_REGEX: &str = r"^((?P<address_type>keybase|grinbox|https)://).+$";
const GRINBOX_ADDRESS_REGEX: &str = r"^(grinbox://)?(?P<public_key>[123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz]{52})(@(?P<domain>[a-zA-Z0-9\.]+)(:(?P<port>[0-9]*))?)?$";
const KEYBASE_ADDRESS_REGEX: &str = r"^(keybase://)?(?P<username>[0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_]{1,16})(:(?P<topic>[a-zA-Z0-9_-]+))?$";
const DEFAULT_GRINBOX_DOMAIN: &str = "grinbox.io";
#[cfg(not(windows))]
pub const DEFAULT_GRINBOX_PORT: u16 = 443;
#[cfg(windows)]
pub const DEFAULT_GRINBOX_PORT: u16 = 80;

#[derive(PartialEq)]
pub enum AddressType {
    Grinbox,
    Keybase,
    Https,
}

pub trait Address: Debug + Display {
    fn from_str(s: &str) -> Result<Self>
    where
        Self: Sized;
    fn address_type(&self) -> AddressType;
    fn stripped(&self) -> String;
}

impl Address {
    pub fn parse(address: &str) -> Result<Box<Address>> {
        let re = Regex::new(ADDRESS_REGEX)?;
        let captures = re.captures(address);
        if captures.is_none() {
            return Ok(Box::new(GrinboxAddress::from_str(address)?));
        }

        let captures = captures.unwrap();
        let address_type = captures.name("address_type").unwrap().as_str().to_string();
        let address: Box<Address> = match address_type.as_ref() {
            "keybase" => Box::new(KeybaseAddress::from_str(address)?),
            "grinbox" => Box::new(GrinboxAddress::from_str(address)?),
            "https" => Box::new(HttpsAddress::from_str(address)?),
            x => Err(ErrorKind::UnknownAddressType(x.to_string()))?,
        };
        Ok(address)
    }
}

pub trait AddressBookBackend {
    fn get_contact(&mut self, name: &[u8]) -> Result<Contact>;
    fn contacts(&self) -> Box<Iterator<Item = Contact>>;
    fn batch<'a>(&'a self) -> Result<Box<AddressBookBatch + 'a>>;
}

pub trait AddressBookBatch {
    fn save_contact(&mut self, contact: &Contact) -> Result<()>;
    fn delete_contact(&mut self, public_key: &[u8]) -> Result<()>;
    fn commit(&mut self) -> Result<()>;
}

pub struct AddressBook {
    backend: Box<AddressBookBackend + Send>,
}

impl AddressBook {
    pub fn new(backend: Box<AddressBookBackend + Send>) -> Result<Self> {
        let address_book = Self { backend };
        Ok(address_book)
    }

    pub fn add_contact(&mut self, contact: &Contact) -> Result<()> {
        let result = self.get_contact(&contact.name);
        if result.is_ok() {
            return Err(ErrorKind::ContactAlreadyExists(contact.name.clone()))?;
        }
        let mut batch = self.backend.batch()?;
        batch.save_contact(contact)?;
        batch.commit()?;
        Ok(())
    }

    pub fn remove_contact(&mut self, name: &str) -> Result<()> {
        let mut batch = self.backend.batch()?;
        batch.delete_contact(name.as_bytes())?;
        batch.commit()?;
        Ok(())
    }

    pub fn get_contact(&mut self, name: &str) -> Result<Contact> {
        let contact = self.backend.get_contact(name.as_bytes())?;
        Ok(contact)
    }

    pub fn get_contact_by_address(&mut self, address: &str) -> Result<Contact> {
        for contact in self.contacts() {
            if contact.address == address {
                return Ok(contact);
            }
        }
        Err(ErrorKind::ContactNotFound(address.to_string()))?
    }

    pub fn contacts(&self) -> Box<Iterator<Item = Contact>> {
        self.backend.contacts()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Contact {
    name: String,
    address: String,
}

impl Contact {
    pub fn new(name: &str, address: Box<Address>) -> Result<Self> {
        Ok(Self {
            name: name.to_string(),
            address: address.to_string(),
        })
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_address(&self) -> &String {
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
    pub topic: Option<String>,
}

impl Address for KeybaseAddress {
    fn from_str(s: &str) -> Result<Self> {
        let re = Regex::new(KEYBASE_ADDRESS_REGEX).unwrap();
        let captures = re.captures(s);
        if captures.is_none() {
            Err(ErrorKind::KeybaseAddressParsingError(s.to_string()))?;
        }

        let captures = captures.unwrap();
        let username = captures.name("username").unwrap().as_str().to_string();
        let topic = captures.name("topic").map(|m| m.as_str().to_string());
        Ok(Self { username, topic })
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
        if let Some(ref topic) = self.topic {
            write!(f, ":{}", topic)?;
        }
        Ok(())
    }
}

pub fn version_bytes() -> Vec<u8> {
    if is_mainnet() {
        GRINBOX_ADDRESS_VERSION_MAINNET.to_vec()
    } else {
        GRINBOX_ADDRESS_VERSION_TESTNET.to_vec()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GrinboxAddress {
    pub public_key: String,
    pub domain: String,
    pub port: Option<u16>,
}

impl GrinboxAddress {
    pub fn new(public_key: PublicKey, domain: Option<String>, port: Option<u16>) -> Self {
        Self {
            public_key: public_key.to_base58_check(version_bytes()),
            domain: domain.unwrap_or(DEFAULT_GRINBOX_DOMAIN.to_string()),
            port,
        }
    }

    pub fn public_key(&self) -> Result<PublicKey> {
        PublicKey::from_base58_check(&self.public_key, version_bytes())
    }
}

impl Address for GrinboxAddress {
    fn from_str(s: &str) -> Result<Self> {
        let re = Regex::new(GRINBOX_ADDRESS_REGEX).unwrap();
        let captures = re.captures(s);
        if captures.is_none() {
            Err(ErrorKind::GrinboxAddressParsingError(s.to_string()))?;
        }

        let captures = captures.unwrap();
        let public_key = captures.name("public_key").unwrap().as_str().to_string();
        let domain = captures.name("domain").map(|m| m.as_str().to_string());
        let port = captures
            .name("port")
            .map(|m| u16::from_str_radix(m.as_str(), 10).unwrap());

        let public_key = PublicKey::from_base58_check(&public_key, version_bytes())?;

        Ok(GrinboxAddress::new(public_key, domain, port))
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
        if self.domain != DEFAULT_GRINBOX_DOMAIN
            || (self.port.is_some() && self.port.unwrap() != DEFAULT_GRINBOX_PORT)
        {
            write!(f, "@{}", self.domain)?;
            if self.port.is_some() && self.port.unwrap() != DEFAULT_GRINBOX_PORT {
                write!(f, ":{}", self.port.unwrap())?;
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HttpsAddress {
    pub uri: String,
}

impl Address for HttpsAddress {
    fn from_str(s: &str) -> Result<Self> {
        Url::parse(s).map_err(|_| ErrorKind::HttpsAddressParsingError(s.to_string()))?;

        Ok(Self { uri: s.to_string() })
    }

    fn address_type(&self) -> AddressType {
        AddressType::Https
    }

    fn stripped(&self) -> String {
        self.uri.clone()
    }
}

impl Display for HttpsAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.uri)?;
        Ok(())
    }
}
