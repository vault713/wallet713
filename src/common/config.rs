use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::fs::File;
use std::fmt;

use colored::*;

use grin_wallet::{WalletConfig};
use grin_core::global::ChainTypes;

use super::Result;

use contacts::GrinboxAddress;
use super::crypto::{SecretKey, PublicKey, public_key_from_secret_key, Hex, Base58, GRINBOX_ADDRESS_VERSION_TESTNET, GRINBOX_ADDRESS_VERSION_MAINNET};

const WALLET713_HOME: &str = ".wallet713";
const WALLET713_DEFAULT_CONFIG_FILENAME: &str = "wallet713.toml";

const GRIN_HOME: &str = ".grin";
const GRIN_NODE_API_SECRET_FILE: &str = ".api_secret";

const DEFAULT_CONFIG: &str = r#"
	wallet713_data_path = "wallet713_data"
	grinbox_domain = "grinbox.io"
	grinbox_port = 13420
	grinbox_private_key = ""
	grin_node_uri = "http://127.0.0.1:13413"
	grin_node_secret = ""
	default_keybase_ttl = "24h"
"#;

#[derive(Debug, Serialize, Deserialize)]
pub struct Wallet713Config {
    pub chain: Option<ChainTypes>,
    pub wallet713_data_path: String,
    pub grinbox_domain: String,
    pub grinbox_port: u16,
    pub grinbox_private_key: String,
    pub grin_node_uri: String,
    pub grin_node_secret: Option<String>,
    pub grinbox_listener_auto_start: Option<bool>,
    pub keybase_listener_auto_start: Option<bool>,
    pub max_auto_accept_invoice: Option<u64>,
    pub default_keybase_ttl: Option<String>,
}

impl Wallet713Config {
    pub fn exists(config_path: Option<&str>, chain: &Option<ChainTypes>) -> Result<bool> {
        let default_path_buf = Wallet713Config::default_config_path(chain)?;
        let default_path = default_path_buf.to_str().unwrap();
        let config_path = config_path.unwrap_or(default_path);
        Ok(Path::new(config_path).exists())
    }

    pub fn from_file(config_path: Option<&str>, chain: &Option<ChainTypes>) -> Result<Wallet713Config> {
        let default_path_buf = Wallet713Config::default_config_path(chain)?;
        let default_path = default_path_buf.to_str().unwrap();
        let config_path = config_path.unwrap_or(default_path);
        let mut file = File::open(config_path)?;
        let mut toml_str = String::new();
        file.read_to_string(&mut toml_str)?;
        let config = toml::from_str(&toml_str[..])?;
        Ok(config)
    }

    pub fn default_config_path(chain: &Option<ChainTypes>) -> Result<PathBuf> {
        let mut path = Wallet713Config::default_home_path(chain)?;
        path.push(WALLET713_DEFAULT_CONFIG_FILENAME);
        Ok(path)
    }

    pub fn default_home_path(chain: &Option<ChainTypes>) -> Result<PathBuf> {
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

    pub fn default(chain: &Option<ChainTypes>) -> Result<Wallet713Config> {
        let mut config: Wallet713Config = toml::from_str(DEFAULT_CONFIG)?;
        config.grin_node_secret = None;
        config.chain = chain.clone();
        if let Some(mut home_path) = dirs::home_dir() {
            home_path.push(GRIN_HOME);
            match config.chain {
                Some(ref chain_type) => home_path.push(chain_type.shortname()),
                None => home_path.push(ChainTypes::Mainnet.shortname()),
            }
            home_path.push(GRIN_NODE_API_SECRET_FILE);
            let path_str = home_path.to_str().unwrap();
            if let Ok(mut file) = File::open(&path_str) {
                let mut contents: String = String::new();
                file.read_to_string(&mut contents)?;
                config.grin_node_secret = Some(contents);
            }
        };
        Ok(config)
    }

    pub fn to_file(&self, config_path: Option<&str>) -> Result<()> {
        let default_path_buf = Wallet713Config::default_config_path(&self.chain)?;
        let default_path = default_path_buf.to_str().unwrap();
        let config_path = config_path.unwrap_or(default_path);
        let toml_str = toml::to_string(&self)?;
        let mut f = File::create(config_path)?;
        f.write_all(toml_str.as_bytes())?;
        Ok(())
    }

    pub fn as_wallet_config(&self) -> Result<WalletConfig> {
        let data_path_buf = self.get_data_path()?;
        let data_path = data_path_buf.to_str().unwrap();
        let mut wallet_config = WalletConfig::default();
        wallet_config.chain_type = self.chain.clone();
        wallet_config.data_file_dir = data_path.to_string();
        wallet_config.check_node_api_http_addr = self.grin_node_uri.clone();
        Ok(wallet_config)
    }

    pub fn get_grinbox_address(&self) -> Result<GrinboxAddress> {
        let public_key = self.get_grinbox_public_key()?;
        let public_key = match self.chain {
            None | Some(ChainTypes::Mainnet) => public_key.to_base58_check(GRINBOX_ADDRESS_VERSION_MAINNET.to_vec()),
            Some(ChainTypes::AutomatedTesting) | Some(ChainTypes::UserTesting) | Some(ChainTypes::Floonet) => public_key.to_base58_check(GRINBOX_ADDRESS_VERSION_TESTNET.to_vec()),
        };
        let address = GrinboxAddress {
            public_key,
            domain: self.grinbox_domain.clone(),
            port: self.grinbox_port,
        };
        Ok(address)
    }

    pub fn get_grinbox_public_key(&self) -> Result<PublicKey> {
        let public_key = public_key_from_secret_key(&self.get_grinbox_secret_key()?);
        Ok(public_key)
    }

    pub fn get_grinbox_secret_key(&self) -> Result<SecretKey> {
        let secret_key = SecretKey::from_hex(&self.grinbox_private_key)?;
        Ok(secret_key)
    }

    pub fn get_data_path(&self) -> Result<PathBuf> {
        let mut default_path = Wallet713Config::default_home_path(&self.chain)?;
        default_path.push(self.wallet713_data_path.clone());
        Ok(default_path)
    }
}

impl fmt::Display for Wallet713Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "wallet713_data_path={}\ngrinbox_domain={}\ngrinbox_port={}\ngrinbox_private_key={}\ngrin_node_uri={}\ngrin_node_secret={}",
               self.wallet713_data_path,
               self.grinbox_domain,
               self.grinbox_port,
               "{...}",
               self.grin_node_uri,
               "{...}")?;
        if self.grinbox_private_key.is_empty() {
            write!(f, "\n{}: grinbox keypair not set! consider using `config --generate-keys`", "WARNING".bright_yellow())?;
        }
        Ok(())
    }
}
