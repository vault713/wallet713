use std::fmt;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use grin_core::global::ChainTypes;
use grin_util::LoggingConfig;

use super::crypto::{public_key_from_secret_key, PublicKey, SecretKey};
use super::{ErrorKind, Result};
use super::is_cli;
use crate::contacts::{GrinboxAddress, DEFAULT_GRINBOX_PORT};

const WALLET713_HOME: &str = ".wallet713";
const WALLET713_DEFAULT_CONFIG_FILENAME: &str = "wallet713.toml";

const DEFAULT_CONFIG: &str = r#"
	wallet713_data_path = "wallet713_data"
	grinbox_domain = "grinbox.io"
	default_keybase_ttl = "24h"
"#;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Wallet713Config {
    pub chain: Option<ChainTypes>,
    pub wallet713_data_path: String,
    pub grinbox_domain: String,
    pub grinbox_port: Option<u16>,
    pub grinbox_protocol_unsecure: Option<bool>,
    pub grinbox_address_index: Option<u32>,
    pub grin_node_uri: Option<String>,
    pub grin_node_secret: Option<String>,
    pub grinbox_listener_auto_start: Option<bool>,
    pub keybase_listener_auto_start: Option<bool>,
    pub max_auto_accept_invoice: Option<u64>,
    pub default_keybase_ttl: Option<String>,
    pub owner_api: Option<bool>,
    pub owner_api_address: Option<String>,
    pub owner_api_secret: Option<String>,
    pub owner_api_include_foreign: Option<bool>,
    pub foreign_api: Option<bool>,
    pub foreign_api_address: Option<String>,
    pub foreign_api_secret: Option<String>,
    pub check_updates: Option<bool>,
    #[serde(skip)]
    pub config_home: Option<String>,
    #[serde(skip)]
    pub grinbox_address_key: Option<SecretKey>,
}

impl Wallet713Config {
    pub fn exists(config_path: Option<&str>, chain: &Option<ChainTypes>) -> Result<bool> {
        let default_path_buf = Wallet713Config::default_config_path(chain)?;
        let default_path = default_path_buf.to_str().unwrap();
        let config_path = config_path.unwrap_or(default_path);
        Ok(Path::new(config_path).exists())
    }

    pub fn from_file(
        config_path: Option<&str>,
        chain: &Option<ChainTypes>,
    ) -> Result<Wallet713Config> {
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
        config.chain = chain.clone();
        Ok(config)
    }

    pub fn to_file(&mut self, config_path: Option<&str>) -> Result<()> {
        let default_path_buf = Wallet713Config::default_config_path(&self.chain)?;
        let default_path = default_path_buf.to_str().unwrap();
        let config_path = config_path.unwrap_or(default_path);
        let toml_str = toml::to_string(&self)?;
        let mut f = File::create(config_path)?;
        f.write_all(toml_str.as_bytes())?;
        self.config_home = Some(config_path.to_string());
        Ok(())
    }

    pub fn as_wallet_config(&self) -> Result<WalletConfig> {
        let data_path_buf = self.get_data_path()?;
        let data_path = data_path_buf.to_str().unwrap();
        let mut wallet_config = WalletConfig::default();
        wallet_config.chain_type = self.chain.clone();
        wallet_config.data_file_dir = data_path.to_string();
        wallet_config.check_node_api_http_addr = self.grin_node_uri().clone();
        Ok(wallet_config)
    }

    pub fn grinbox_protocol_unsecure(&self) -> bool {
        self.grinbox_protocol_unsecure.unwrap_or(cfg!(windows))
    }

    pub fn grinbox_address_index(&self) -> u32 {
        self.grinbox_address_index.unwrap_or(0)
    }

    pub fn get_grinbox_address(&self) -> Result<GrinboxAddress> {
        let public_key = self.get_grinbox_public_key()?;
        Ok(GrinboxAddress::new(
            public_key,
            Some(self.grinbox_domain.clone()),
            self.grinbox_port,
        ))
    }

    pub fn get_grinbox_public_key(&self) -> Result<PublicKey> {
        public_key_from_secret_key(&self.get_grinbox_secret_key()?)
    }

    pub fn get_grinbox_secret_key(&self) -> Result<SecretKey> {
        self.grinbox_address_key.clone()
            .ok_or_else(|| ErrorKind::NoWallet.into())
    }

    pub fn get_data_path(&self) -> Result<PathBuf> {
        let mut data_path = PathBuf::new();
        data_path.push(self.wallet713_data_path.clone());
        if data_path.is_absolute() {
            return Ok(data_path);
        }

        let mut data_path = PathBuf::new();
        data_path.push(
            self.config_home
                .clone()
                .unwrap_or(WALLET713_DEFAULT_CONFIG_FILENAME.to_string()),
        );
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
        let chain_type = self.chain.as_ref().unwrap_or(&ChainTypes::Mainnet);
        match self.grin_node_uri {
            Some(_) => self.grin_node_secret.clone(),
            None => match chain_type {
                ChainTypes::Mainnet => Some(String::from("thanksvault713kizQ4ZVv")),
                _ => Some(String::from("thanksvault713EcRXKbYS")),
            },
        }
    }

    pub fn grinbox_listener_auto_start(&self) -> bool {
        self.grinbox_listener_auto_start.unwrap_or(is_cli())
    }

    pub fn keybase_listener_auto_start(&self) -> bool {
        self.keybase_listener_auto_start.unwrap_or(false)
    }

    pub fn owner_api_address(&self) -> String {
        let chain_type = self.chain.as_ref().unwrap_or(&ChainTypes::Mainnet);
        self.owner_api_address
            .as_ref()
            .map(|a| a.clone())
            .unwrap_or_else(|| match chain_type {
                ChainTypes::Mainnet => String::from("127.0.0.1:3420"),
                _ => String::from("127.0.0.1:13420"),
            })
    }

    pub fn foreign_api_address(&self) -> String {
        let chain_type = self.chain.as_ref().unwrap_or(&ChainTypes::Mainnet);
        self.foreign_api_address
            .as_ref()
            .map(|a| a.clone())
            .unwrap_or_else(|| match chain_type {
                ChainTypes::Mainnet => String::from("127.0.0.1:3415"),
                _ => String::from("127.0.0.1:13415"),
            })
    }

    pub fn owner_api(&self) -> bool {
        self.owner_api.unwrap_or(false)
    }

    pub fn foreign_api(&self) -> bool {
        self.foreign_api.unwrap_or(false)
    }

    pub fn check_updates(&self) -> bool {
        self.check_updates.unwrap_or(is_cli())
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


/// Command-line wallet configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WalletConfig {
	/// Chain parameters (default to Mainnet if none at the moment)
	pub chain_type: Option<ChainTypes>,
	/// The api interface/ip_address that this api server (i.e. this wallet) will run
	/// by default this is 127.0.0.1 (and will not accept connections from external clients)
	pub api_listen_interface: String,
	/// The port this wallet will run on
	pub api_listen_port: u16,
	/// The port this wallet's owner API will run on
	pub owner_api_listen_port: Option<u16>,
	/// Location of the secret for basic auth on the Owner API
	pub api_secret_path: Option<String>,
	/// Location of the node api secret for basic auth on the Grin API
	pub node_api_secret_path: Option<String>,
	/// The api address of a running server node against which transaction inputs
	/// will be checked during send
	pub check_node_api_http_addr: String,
	/// Whether to include foreign API endpoints on the Owner API
	pub owner_api_include_foreign: Option<bool>,
	/// The directory in which wallet files are stored
	pub data_file_dir: String,
	/// If Some(true), don't cache commits alongside output data
	/// speed improvement, but your commits are in the database
	pub no_commit_cache: Option<bool>,
	/// TLS certificate file
	pub tls_certificate_file: Option<String>,
	/// TLS certificate private key file
	pub tls_certificate_key: Option<String>,
	/// Whether to use the black background color scheme for command line
	/// if enabled, wallet command output color will be suitable for black background terminal
	pub dark_background_color_scheme: Option<bool>,
	/// The exploding lifetime (minutes) for keybase notification on coins received
	pub keybase_notify_ttl: Option<u16>,
}

impl Default for WalletConfig {
	fn default() -> WalletConfig {
		WalletConfig {
			chain_type: Some(ChainTypes::Mainnet),
			api_listen_interface: "127.0.0.1".to_string(),
			api_listen_port: 3415,
			owner_api_listen_port: Some(WalletConfig::default_owner_api_listen_port()),
			api_secret_path: Some(".api_secret".to_string()),
			node_api_secret_path: Some(".api_secret".to_string()),
			check_node_api_http_addr: "http://127.0.0.1:3413".to_string(),
			owner_api_include_foreign: Some(false),
			data_file_dir: ".".to_string(),
			no_commit_cache: Some(false),
			tls_certificate_file: None,
			tls_certificate_key: None,
			dark_background_color_scheme: Some(true),
			keybase_notify_ttl: Some(1440),
		}
	}
}

impl WalletConfig {
	/// API Listen address
	pub fn api_listen_addr(&self) -> String {
		format!("{}:{}", self.api_listen_interface, self.api_listen_port)
	}

	/// Default listener port
	pub fn default_owner_api_listen_port() -> u16 {
		3420
	}

	/// Use value from config file, defaulting to sensible value if missing.
	pub fn owner_api_listen_port(&self) -> u16 {
		self.owner_api_listen_port
			.unwrap_or(WalletConfig::default_owner_api_listen_port())
	}

	/// Owner API listen address
	pub fn owner_api_listen_addr(&self) -> String {
		format!("127.0.0.1:{}", self.owner_api_listen_port())
	}
}
/// Error type wrapping config errors.
#[derive(Debug)]
pub enum ConfigError {
	/// Error with parsing of config file
	ParseError(String, String),

	/// Error with fileIO while reading config file
	FileIOError(String, String),

	/// No file found
	FileNotFoundError(String),

	/// Error serializing config values
	SerializationError(String),
}

impl fmt::Display for ConfigError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match *self {
			ConfigError::ParseError(ref file_name, ref message) => write!(
				f,
				"Error parsing configuration file at {} - {}",
				file_name, message
			),
			ConfigError::FileIOError(ref file_name, ref message) => {
				write!(f, "{} {}", message, file_name)
			}
			ConfigError::FileNotFoundError(ref file_name) => {
				write!(f, "Configuration file not found: {}", file_name)
			}
			ConfigError::SerializationError(ref message) => {
				write!(f, "Error serializing configuration: {}", message)
			}
		}
	}
}

impl From<io::Error> for ConfigError {
	fn from(error: io::Error) -> ConfigError {
		ConfigError::FileIOError(
			String::from(""),
			String::from(format!("Error loading config file: {}", error)),
		)
	}
}

/// Wallet should be split into a separate configuration file
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GlobalWalletConfig {
	/// Keep track of the file we've read
	pub config_file_path: Option<PathBuf>,
	/// Wallet members
	pub members: Option<GlobalWalletConfigMembers>,
}

/// Wallet internal members
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GlobalWalletConfigMembers {
	/// Wallet configuration
	#[serde(default)]
	pub wallet: WalletConfig,
	/// Logging config
	pub logging: Option<LoggingConfig>,
}