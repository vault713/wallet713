use failure::Error;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::fmt;

use grin_wallet::{WalletConfig};
use grin_core::global::{ChainTypes, is_mainnet};

use crate::common::error::Wallet713Error;
use contacts::{GrinboxAddress, DEFAULT_GRINBOX_PORT};
use super::crypto::{SecretKey, PublicKey, public_key_from_secret_key};

const WALLET713_HOME: &str = ".wallet713";
const WALLET713_DEFAULT_CONFIG_FILENAME: &str = "wallet713.toml";

const DEFAULT_CONFIG: &str = r#"
	wallet713_data_path = "wallet713_data"
	grinbox_domain = "grinbox.io"
	default_keybase_ttl = "24h"
"#;

#[derive(Debug, Serialize, Deserialize)]
pub struct Wallet713Config {
    pub chain: Option<ChainTypes>,
    pub wallet713_data_path: String,
    pub grinbox_domain: String,
    pub grinbox_port: Option<u16>,
    pub grinbox_e2e_encryption: Option<bool>,
    pub grinbox_address_index: Option<u32>,
    pub grin_node_uri: Option<String>,
    pub grin_node_secret: Option<String>,
    pub grinbox_listener_auto_start: Option<bool>,
    pub keybase_listener_auto_start: Option<bool>,
    pub max_auto_accept_invoice: Option<u64>,
    pub default_keybase_ttl: Option<String>,
    #[serde(skip)]
    config_home: Option<String>,
    #[serde(skip)]
    pub grinbox_address_key: Option<SecretKey>,
}

impl Wallet713Config {
    pub fn exists(config_path: Option<&str>, chain: &Option<ChainTypes>) -> Result<bool, Error> {
        let default_path_buf = Wallet713Config::default_config_path(chain)?;
        let default_path = default_path_buf.to_str().unwrap();
        let config_path = config_path.unwrap_or(default_path);
        Ok(Path::new(config_path).exists())
    }

    pub fn from_file(config_path: Option<&str>, chain: &Option<ChainTypes>) -> Result<Wallet713Config, Error> {
        let default_path_buf = Wallet713Config::default_config_path(chain)?;
        let default_path = default_path_buf.to_str().unwrap();
        let config_path = config_path.unwrap_or(default_path);
        let mut file = File::open(config_path)?;
        let mut toml_str = String::new();
        file.read_to_string(&mut toml_str)?;
        let mut config: Wallet713Config = toml::from_str(&toml_str[..])?;
        config.config_home = Some(config_path.to_string());
        Ok(config)
    }

    pub fn default_config_path(chain: &Option<ChainTypes>) -> Result<PathBuf, Error> {
        let mut path = Wallet713Config::default_home_path(chain)?;
        path.push(WALLET713_DEFAULT_CONFIG_FILENAME);
        Ok(path)
    }

    pub fn default_home_path(chain: &Option<ChainTypes>) -> Result<PathBuf, Error> {
        let mut path = match dirs::home_dir() {
            Some(home) => home,
            None => std::env::current_dir()?,
        };

        path.push(WALLET713_HOME);
        match chain {
            Some(ref chain_type) => path.push(chain_type.shortname()),
            None => path.push(ChainTypes::Mainnet.shortname()),
        };
        std::fs::create_dir_all(path.as_path())?;
        Ok(path)
    }

    pub fn default(chain: &Option<ChainTypes>) -> Result<Wallet713Config, Error> {
        let mut config: Wallet713Config = toml::from_str(DEFAULT_CONFIG)?;
        config.chain = chain.clone();
        Ok(config)
    }

    pub fn to_file(&mut self, config_path: Option<&str>) -> Result<(), Error> {
        let default_path_buf = Wallet713Config::default_config_path(&self.chain)?;
        let default_path = default_path_buf.to_str().unwrap();
        let config_path = config_path.unwrap_or(default_path);
        let toml_str = toml::to_string(&self)?;
        let mut f = File::create(config_path)?;
        f.write_all(toml_str.as_bytes())?;
        self.config_home = Some(config_path.to_string());
        Ok(())
    }

    pub fn as_wallet_config(&self) -> Result<WalletConfig, Error> {
        let data_path_buf = self.get_data_path()?;
        let data_path = data_path_buf.to_str().unwrap();
        let mut wallet_config = WalletConfig::default();
        wallet_config.chain_type = self.chain.clone();
        wallet_config.data_file_dir = data_path.to_string();
        wallet_config.check_node_api_http_addr = self.grin_node_uri().clone();
        Ok(wallet_config)
    }

    pub fn grinbox_e2e_encryption(&self) -> bool {
        self.grinbox_e2e_encryption.unwrap_or(is_mainnet())
    }

    pub fn grinbox_address_index(&self) -> u32 {
        self.grinbox_address_index.unwrap_or(0)
    }

    pub fn get_grinbox_address(&self) -> Result<GrinboxAddress, Error> {
        let public_key = self.get_grinbox_public_key()?;
        Ok(GrinboxAddress::new(public_key, self.grinbox_domain.clone(), self.grinbox_port))
    }

    pub fn get_grinbox_public_key(&self) -> Result<PublicKey, Error> {
        public_key_from_secret_key(&self.get_grinbox_secret_key()?)
    }

    pub fn get_grinbox_secret_key(&self) -> Result<SecretKey, Error> {
        self.grinbox_address_key.ok_or_else(|| Wallet713Error::NoWallet.into())
    }

    pub fn get_data_path(&self) -> Result<PathBuf, Error> {
        let mut data_path = PathBuf::new();
        data_path.push(self.wallet713_data_path.clone());
        if data_path.is_absolute() {
            return Ok(data_path);
        }

        let mut data_path = PathBuf::new();
        data_path.push(self.config_home.clone().unwrap_or(WALLET713_DEFAULT_CONFIG_FILENAME.to_string()));
        data_path.pop();
        data_path.push(self.wallet713_data_path.clone());
        Ok(data_path)
    }

    pub fn grin_node_uri(&self) -> String {
        let chain_type = self.chain.as_ref().unwrap_or(&ChainTypes::Floonet);
        self.grin_node_uri.clone().unwrap_or(match chain_type {
            ChainTypes::Mainnet => String::from("https://node.713.mw"),
            _ => String::from("https://floonet.node.713.mw"),
        })
    }

    pub fn grin_node_secret(&self) -> Option<String> {
        let chain_type = self.chain.as_ref().unwrap_or(&ChainTypes::Floonet);
        match self.grin_node_uri {
            Some(_) => self.grin_node_secret.clone(),
            None => match chain_type {
                ChainTypes::Mainnet => Some(String::from("thanksvault713kizQ4ZVv")),
                _ => Some(String::from("thanksvault713EcRXKbYS")),
            }
        }
    }
}

impl fmt::Display for Wallet713Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "wallet713_data_path={}\ngrinbox_domain={}\ngrinbox_port={}\ngrin_node_uri={}\ngrin_node_secret={}",
               self.wallet713_data_path,
               self.grinbox_domain,
               self.grinbox_port.unwrap_or(DEFAULT_GRINBOX_PORT),
               self.grin_node_uri.clone().unwrap_or(String::from("provided by vault713")),
               "{...}")?;
        Ok(())
    }
}
