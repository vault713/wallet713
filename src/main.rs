#[macro_use] extern crate serde_derive;
#[macro_use] extern crate prettytable;
#[macro_use] extern crate log;
#[macro_use] extern crate serde_json;
extern crate failure;
extern crate serde;
extern crate clap;
extern crate colored;
extern crate ws;
extern crate futures;
extern crate tokio;
extern crate rand;
extern crate sha2;
extern crate digest;
extern crate hmac;
extern crate ripemd160;
extern crate ring;
extern crate uuid;
extern crate regex;
extern crate rustyline;
extern crate chrono;
extern crate term;
extern crate blake2_rfc;

extern crate grin_core;
extern crate grin_keychain;
extern crate grin_util;
extern crate grin_wallet;
extern crate grin_store;

use std::sync::{Arc, Mutex};
use std::io::{Read, Write};
use std::fs::File;
use std::path::Path;

use clap::{App, Arg, ArgMatches};
use colored::*;
use rustyline::Editor;

use grin_core::core;
use grin_core::global::{ChainTypes, set_mining_mode};

#[macro_use]
mod common;
mod broker;
mod wallet;
mod contacts;
mod cli;

use common::{ErrorKind, Result, RuntimeMode};
use common::config::Wallet713Config;
use wallet::Wallet;
use cli::Parser;

use contacts::{Address, AddressType, GrinboxAddress, Contact, AddressBook, Backend};

const CLI_HISTORY_PATH: &str = ".history";

fn do_config(args: &ArgMatches, chain: &Option<ChainTypes>, silent: bool, new_address_index: Option<u32>, config_path: Option<&str>) -> Result<Wallet713Config> {
    let mut config;
    let mut any_matches = false;
    let exists = Wallet713Config::exists(config_path, &chain)?;
    if exists {
        config = Wallet713Config::from_file(config_path, &chain)?;
    } else {
        config = Wallet713Config::default(&chain)?;
    }

    if let Some(data_path) = args.value_of("data-path") {
        config.wallet713_data_path = data_path.to_string();
        any_matches = true;
    }

    if let Some(domain) = args.value_of("domain") {
        config.grinbox_domain = domain.to_string();
        any_matches = true;
    }

    if let Some(port) = args.value_of("port") {
        let port = u16::from_str_radix(port, 10).map_err(|_| {
            ErrorKind::NumberParsingError
        })?;
        config.grinbox_port = Some(port);
        any_matches = true;
    }

    if let Some(node_uri) = args.value_of("node-uri") {
        config.grin_node_uri = Some(node_uri.to_string());
        any_matches = true;
    }

    if let Some(node_secret) = args.value_of("node-secret") {
        config.grin_node_secret = Some(node_secret.to_string());
        any_matches = true;
    }

    if new_address_index.is_some() {
        config.grinbox_address_index = new_address_index;
        any_matches = true;
    }

    config.to_file(config_path)?;

    if !any_matches && !silent {
        cli_message!("{}", config);
    }

    Ok(config)
}

fn do_contacts(args: &ArgMatches, address_book: Arc<Mutex<AddressBook>>) -> Result<()> {
    let mut address_book = address_book.lock().unwrap();
    if let Some(add_args) = args.subcommand_matches("add") {
        let name = add_args.value_of("name").expect("missing argument: name");
        let address = add_args.value_of("address").expect("missing argument: address");

        // try parse as a general address and fallback to grinbox address
        let contact_address = Address::parse(address);
        let contact_address: Result<Box<Address>> = match contact_address {
            Ok(address) => Ok(address),
            Err(e) => {
                Ok(Box::new(GrinboxAddress::from_str(address).map_err(|_| e)?) as Box<Address>)
            }
        };

        let contact = Contact::new(name, contact_address?)?;
        address_book.add_contact(&contact)?;
    } else if let Some(add_args) = args.subcommand_matches("remove") {
        let name = add_args.value_of("name").unwrap();
        address_book.remove_contact(name)?;
    } else {
        let contacts: Vec<()> = address_book
            .contacts()
            .map(|contact| {
                cli_message!("@{} = {}", contact.get_name(), contact.get_address());
                ()
            })
            .collect();

        if contacts.len() == 0 {
            cli_message!("your contact list is empty. consider using `contacts add` to add a new contact.");
        }
    }
    Ok(())
}

const WELCOME_HEADER: &str = r#"
Welcome to wallet713
"#;

const WELCOME_FOOTER: &str = r#"Use `listen` to connect to grinbox or `help` to see available commands
"#;

fn welcome(args: &ArgMatches, runtime_mode: &RuntimeMode) -> Result<Wallet713Config> {
    let chain: Option<ChainTypes> = match args.is_present("floonet") {
        true => Some(ChainTypes::Floonet),
        false => Some(ChainTypes::Mainnet)
    };

    unsafe {
        common::set_runtime_mode(runtime_mode);
    };

    let config = do_config(args, &chain, true, None, args.value_of("config-path"))?;
    set_mining_mode(config.chain.clone().unwrap_or(ChainTypes::Mainnet));

    Ok(config)
}

use std::borrow::Borrow;
use grin_core::libtx::slate::Slate;
use broker::{GrinboxSubscriber, GrinboxPublisher, KeybasePublisher, KeybaseSubscriber, SubscriptionHandler, Subscriber, Publisher, CloseReason};

struct Controller {
    name: String,
    wallet: Arc<Mutex<Wallet>>,
    address_book: Arc<Mutex<AddressBook>>,
    publisher: Box<Publisher + Send>,
}

impl Controller {
    pub fn new(name: &str, wallet: Arc<Mutex<Wallet>>, address_book: Arc<Mutex<AddressBook>>, publisher: Box<Publisher + Send>) -> Result<Self> {
        Ok(Self {
            name: name.to_string(),
            wallet,
            address_book,
            publisher,
        })
    }

    fn process_incoming_slate(&self, slate: &mut Slate) -> Result<bool> {
        if slate.num_participants > slate.participant_data.len() {
            //TODO: this needs to be changed to properly figure out if this slate is an invoice or a send
            if slate.tx.inputs().len() == 0 {
                self.wallet.lock().unwrap().process_receiver_initiated_slate(slate)?;
            } else {
                self.wallet.lock().unwrap().process_sender_initiated_slate(slate)?;
            }
            Ok(false)
        } else {
            self.wallet.lock().unwrap().finalize_slate(slate)?;
            Ok(true)
        }
    }
}

impl SubscriptionHandler for Controller {
    fn on_open(&self) {
        cli_message!("listener started for [{}]", self.name.bright_green());
    }

    fn on_slate(&self, from: &Address, slate: &mut Slate) {
        let mut display_from = from.stripped();
        if let Ok(contact) = self.address_book.lock().unwrap().get_contact_by_address(&from.to_string()) {
            display_from = contact.get_name().to_string();
        }

        if slate.num_participants > slate.participant_data.len() {
            cli_message!("slate [{}] received from [{}] for [{}] grins",
                slate.id.to_string().bright_green(),
                display_from.bright_green(),
                core::amount_to_hr_string(slate.amount, false).bright_green()
            );
        } else {
            cli_message!("slate [{}] received back from [{}] for [{}] grins",
                slate.id.to_string().bright_green(),
                display_from.bright_green(),
                core::amount_to_hr_string(slate.amount, false).bright_green()
            );
        };

        if from.address_type() == AddressType::Grinbox {
            GrinboxAddress::from_str(&from.to_string()).expect("invalid grinbox address");
        }

        let result = self.process_incoming_slate(slate).and_then(|is_finalized| {
            if !is_finalized {
                self.publisher.post_slate(slate, from).map_err(|e| {
                    cli_message!("{}: {}", "ERROR".bright_red(), e);
                    e
                }).expect("failed posting slate!");
                cli_message!("slate [{}] sent back to [{}] successfully",
                    slate.id.to_string().bright_green(),
                    display_from.bright_green()
                );
            } else {
                cli_message!("slate [{}] finalized successfully",
                    slate.id.to_string().bright_green()
                );
            }
            Ok(())
        });

        match result {
            Ok(()) => {}
            Err(e) => cli_message!("failed processing incoming slate: {}", e),
        }
    }

    fn on_close(&self, reason: CloseReason) {
        match reason {
            CloseReason::Normal => cli_message!("listener [{}] stopped", self.name.bright_green()),
            CloseReason::Abnormal(_) => cli_message!("{}: listener [{}] stopped unexpectedly", "ERROR".bright_red(), self.name.bright_green()),
        }
    }

    fn on_dropped(&self) {
        cli_message!("{}: listener [{}] lost connection. it will keep trying to restore connection in the background.", "WARNING".bright_yellow(), self.name.bright_green())
    }

    fn on_reestablished(&self) {
        cli_message!("{}: listener [{}] reestablished connection.", "INFO".bright_blue(), self.name.bright_green())
    }
}

fn start_grinbox_listener(config: &Wallet713Config, wallet: Arc<Mutex<Wallet>>, address_book: Arc<Mutex<AddressBook>>) -> Result<(GrinboxPublisher, GrinboxSubscriber)> {
    // make sure wallet is not locked, if it is try to unlock with no passphrase
    if let Ok(mut wallet) = wallet.lock() {
        if wallet.is_locked() {
            wallet.unlock(config, "default", "")?;
        }
    }

    cli_message!("starting grinbox listener...");
    let grinbox_address = config.get_grinbox_address()?;
    let grinbox_secret_key = config.get_grinbox_secret_key()?;
    let grinbox_publisher = GrinboxPublisher::new(&grinbox_address, &grinbox_secret_key, config.grinbox_protocol_unsecure(), config.grinbox_e2e_encryption())?;
    let grinbox_subscriber = GrinboxSubscriber::new(&grinbox_address, &grinbox_secret_key, config.grinbox_protocol_unsecure(), config.grinbox_e2e_encryption()).expect("could not start grinbox subscriber!");
    let cloned_publisher = grinbox_publisher.clone();
    let mut cloned_subscriber = grinbox_subscriber.clone();
    std::thread::spawn(move || {
        let controller = Controller::new(
            &grinbox_address.stripped(),
            wallet.clone(),
            address_book.clone(),
            Box::new(cloned_publisher),
        ).expect("could not start grinbox controller!");
        cloned_subscriber.start(Box::new(controller)).expect("something went wrong!");
    });
    Ok((grinbox_publisher, grinbox_subscriber))
}

fn start_keybase_listener(config: &Wallet713Config, wallet: Arc<Mutex<Wallet>>, address_book: Arc<Mutex<AddressBook>>) -> Result<(KeybasePublisher, KeybaseSubscriber)> {
    // make sure wallet is not locked, if it is try to unlock with no passphrase
    if let Ok(mut wallet) = wallet.lock() {
        if wallet.is_locked() {
            wallet.unlock(config, "default", "")?;
        }
    }

    cli_message!("starting keybase listener...");
    let keybase_subscriber = KeybaseSubscriber::new()?;
    let keybase_publisher = KeybasePublisher::new(config.default_keybase_ttl.clone())?;

    let mut cloned_subscriber = keybase_subscriber.clone();
    let cloned_publisher = keybase_publisher.clone();
    std::thread::spawn(move || {
        let controller = Controller::new(
            "keybase",
            wallet.clone(),
            address_book.clone(),
            Box::new(cloned_publisher),
        ).expect("could not start keybase controller!");
        cloned_subscriber.start(Box::new(controller)).expect("something went wrong!");
    });
    Ok((keybase_publisher, keybase_subscriber))
}

fn main() {
    let matches = App::new("wallet713")
        .arg(Arg::from_usage("[config-path] -c, --config=<config-path> 'the path to the config file'"))
        .arg(Arg::from_usage("[log-config-path] -l, --log-config-path=<log-config-path> 'the path to the log config file'"))
        .arg(Arg::from_usage("[account] -a, --account=<account> 'the account to use'"))
        .arg(Arg::from_usage("[passphrase] -p, --passphrase=<passphrase> 'the passphrase to use'"))
        .arg(Arg::from_usage("[daemon] -d, --daemon 'run daemon'"))
        .arg(Arg::from_usage("[floonet] -f, --floonet 'use floonet'"))
        .get_matches();

    let runtime_mode = match matches.is_present("daemon") {
        true => RuntimeMode::Daemon,
        false => RuntimeMode::Cli,
    };

    let mut config: Wallet713Config = welcome(&matches, &runtime_mode).unwrap_or_else(|e| {
        panic!("{}: could not read or create config! {}", "ERROR".bright_red(), e);
    });

    let data_path_buf = config.get_data_path().unwrap();
    let data_path = data_path_buf.to_str().unwrap();

    let address_book_backend = Backend::new(data_path).expect("could not create address book backend!");
    let address_book = AddressBook::new(Box::new(address_book_backend)).expect("could not create an address book!");
    let address_book = Arc::new(Mutex::new(address_book));

    let wallet = Wallet::new(config.max_auto_accept_invoice);
    let wallet = Arc::new(Mutex::new(wallet));

    let mut grinbox_broker: Option<(GrinboxPublisher, GrinboxSubscriber)> = None;
    let mut keybase_broker: Option<(KeybasePublisher, KeybaseSubscriber)> = None;

    let account = matches.value_of("account");
    let passphrase = matches.value_of("passphrase");
    let result = wallet.lock().unwrap().unlock(&config, account.unwrap_or("default"), passphrase.unwrap_or(""));
    let has_wallet = result.is_ok();

    cli_message!("{}", WELCOME_HEADER.bright_yellow().bold());
    if account.is_some() || passphrase.is_some() {
        if let Err(err) = result {
            cli_message!("{}: {}", "ERROR".bright_red(), err);
        }
    }

    if has_wallet {
        let der = derive_address_key(&mut config, wallet.clone(), &mut grinbox_broker);
        if der.is_err() {
            cli_message!("{}: {}", "ERROR".bright_red(), der.unwrap_err());
        }
    } else {
        cli_message!("{}", "Unlock your existing wallet or type `init` to initiate a new one".bright_blue().bold());
    }
    cli_message!("{}", WELCOME_FOOTER.bright_blue());

    if let Some(auto_start) = config.grinbox_listener_auto_start {
        if auto_start {
            let result = do_command("listen -g", &mut config, wallet.clone(), address_book.clone(), &mut keybase_broker, &mut grinbox_broker);
            if let Err(err) = result {
                cli_message!("{}: {}", "ERROR".bright_red(), err);
            }
        }
    }

    if let Some(auto_start) = config.keybase_listener_auto_start {
        if auto_start {
            let result = do_command("listen -k", &mut config, wallet.clone(), address_book.clone(), &mut keybase_broker, &mut grinbox_broker);
            if let Err(err) = result {
                cli_message!("{}: {}", "ERROR".bright_red(), err);
            }
        }
    }

    let mut rl = Editor::<()>::new();

    let wallet713_home_path_buf = Wallet713Config::default_home_path(&config.chain).unwrap();
    let wallet713_home_path = wallet713_home_path_buf.to_str().unwrap();

    if let Some(path) = Path::new(wallet713_home_path).join(CLI_HISTORY_PATH).to_str() {
        rl.load_history(path).is_ok();
    }

    loop {
        let command = rl.readline("wallet713> ");
        match command {
            Ok(command) => {
                let command = command.trim();

                if command == "exit" {
                    break;
                }
                let result = do_command(&command, &mut config, wallet.clone(), address_book.clone(), &mut keybase_broker, &mut grinbox_broker);
                match result {
                    Err(err) => cli_message!("{}: {}", "ERROR".bright_red(), err),
                    Ok(safe) => if safe {
                        rl.add_history_entry(command);
                    },
                }
            }
            Err(_) => {
                break;
            }
        }
    }

    if let Some(path) = Path::new(wallet713_home_path).join(CLI_HISTORY_PATH).to_str() {
        rl.save_history(path).is_ok();
    }
}

fn derive_address_key(config: &mut Wallet713Config, wallet: Arc<Mutex<Wallet>>, grinbox_broker: &mut Option<(GrinboxPublisher, GrinboxSubscriber)>) -> Result<()> {
    if grinbox_broker.is_some() {
        return Err(ErrorKind::HasListener.into());
    }
    let index = config.grinbox_address_index();
    let key = wallet.lock().unwrap().derive_address_key(index)?;
    config.grinbox_address_key = Some(key);
    show_address(config, false)?;
    Ok(())
}

fn show_address(config: &Wallet713Config, include_index: bool) -> Result<()> {
    cli_message!("{}: {}", "Your grinbox address".bright_yellow(), config.get_grinbox_address()?.stripped().bright_green());
    if include_index {
        cli_message!("Derived with index [{}]", config.grinbox_address_index().to_string().bright_blue());
    }
    Ok(())
}

fn do_command(command: &str, config: &mut Wallet713Config, wallet: Arc<Mutex<Wallet>>, address_book: Arc<Mutex<AddressBook>>, keybase_broker: &mut Option<(KeybasePublisher, KeybaseSubscriber)>, grinbox_broker: &mut Option<(GrinboxPublisher, GrinboxSubscriber)>) -> Result<bool> {
    let matches = Parser::parse(command)?;
    match matches.subcommand_name() {
        Some("config") => {
            let args = matches.subcommand_matches("config").unwrap();

            let new_address_index = match args.is_present("generate-address") {
                false => None,
                true => Some({
                    let index = match args.value_of("generate-address-index") {
                        Some(index) => u32::from_str_radix(index, 10).map_err(|_| ErrorKind::NumberParsingError)?,
                        None => config.grinbox_address_index() + 1,
                    };
                    config.grinbox_address_index = Some(index);
                    index
                }),
            };

            *config = do_config(args, &config.chain, false, new_address_index, config.config_home.as_ref().map(|x| &**x))?;

            if new_address_index.is_some() {
                derive_address_key(config, wallet, grinbox_broker)?;
                cli_message!("Derived with index [{}]", config.grinbox_address_index().to_string().bright_blue());
            }
        }
        Some("address") => {
            show_address(config, true)?;
        }
        Some("init") => {
            if keybase_broker.is_some() || grinbox_broker.is_some() {
                return Err(ErrorKind::HasListener.into());
            }

            let passphrase = matches.subcommand_matches("init").unwrap().value_of("passphrase").unwrap_or("");
            {
                wallet.lock().unwrap().init(config, "default", passphrase)?;
            }
            derive_address_key(config, wallet, grinbox_broker)?;
            if passphrase.is_empty() {
                cli_message!("{}: wallet with no passphrase.", "WARNING".bright_yellow());
            }
            return Ok(false);
        }
        Some("lock") => {
            if keybase_broker.is_some() || grinbox_broker.is_some() {
                return Err(ErrorKind::HasListener.into());
            }
            wallet.lock().unwrap().lock();
        }
        Some("unlock") => {
            let args = matches.subcommand_matches("unlock").unwrap();
            let account = args.value_of("account").unwrap_or("default");
            let passphrase = args.value_of("passphrase").unwrap_or("");
            {
                let mut w = wallet.lock().unwrap();
                if !w.is_locked() {
                    return Err(ErrorKind::WalletAlreadyUnlocked.into());
                }
                w.unlock(config, account, passphrase)?;
            }
            derive_address_key(config, wallet, grinbox_broker)?;
            return Ok(false);
        }
        Some("accounts") => {
            wallet.lock().unwrap().list_accounts()?;
        }
        Some("account") => {
            let args = matches.subcommand_matches("account").unwrap();
            let create_args = args.subcommand_matches("create");
            let switch_args = args.subcommand_matches("switch");
            if let Some(args) = create_args {
                wallet.lock().unwrap().create_account(args.value_of("name").unwrap())?;
            } else if let Some(args) = switch_args {
                let account = args.value_of("name").unwrap();
                let passphrase = args.value_of("passphrase").unwrap_or("");
                wallet.lock().unwrap().unlock(config, account, passphrase)?;
            }
        }
        Some("listen") => {
            let grinbox = matches.subcommand_matches("listen").unwrap().is_present("grinbox");
            let keybase = matches.subcommand_matches("listen").unwrap().is_present("keybase");
            if grinbox || !keybase {
                let is_running = match grinbox_broker {
                    Some((_, subscriber)) => subscriber.is_running(),
                    _ => false
                };
                if is_running {
                    Err(ErrorKind::AlreadyListening("grinbox".to_string()))?
                } else {
                    let (publisher, subscriber) = start_grinbox_listener(config, wallet.clone(), address_book.clone())?;
                    *grinbox_broker = Some((publisher, subscriber));
                }
            }
            if keybase {
                let is_running = match keybase_broker {
                    Some((_, subscriber)) => subscriber.is_running(),
                    _ => false
                };
                if is_running {
                    Err(ErrorKind::AlreadyListening("keybase".to_string()))?
                } else {
                    let (publisher, subscriber) = start_keybase_listener(config, wallet.clone(), address_book.clone())?;
                    *keybase_broker = Some((publisher, subscriber));
                }
            }
        }
        Some("stop") => {
            let grinbox = matches.subcommand_matches("stop").unwrap().is_present("grinbox");
            let keybase = matches.subcommand_matches("stop").unwrap().is_present("keybase");
            if grinbox || !keybase {
                let is_running = match grinbox_broker {
                    Some((_, subscriber)) => subscriber.is_running(),
                    _ => false
                };
                if is_running {
                    cli_message!("stopping grinbox listener...");
                    if let Some((_, subscriber)) = grinbox_broker {
                        subscriber.stop();
                    };
                    *grinbox_broker = None;
                } else {
                    Err(ErrorKind::ClosedListener("grinbox".to_string()))?
                }
            }
            if keybase {
                let is_running = match keybase_broker {
                    Some((_, subscriber)) => subscriber.is_running(),
                    _ => false
                };
                if is_running {
                    cli_message!("stopping keybase listener...");
                    if let Some((_, subscriber)) = keybase_broker {
                        subscriber.stop();
                    };
                    *keybase_broker = None;
                } else {
                    Err(ErrorKind::ClosedListener("keybase".to_string()))?
                }
            }
        }
        Some("info") => {
            wallet.lock().unwrap().info()?;
        }
        Some("txs") => {
            wallet.lock().unwrap().txs()?;
        }
        Some("contacts") => {
            let arg_matches = matches.subcommand_matches("contacts").unwrap();
            do_contacts(&arg_matches, address_book.clone())?;
        }
        Some("outputs") => {
            let args = matches.subcommand_matches("outputs").unwrap();
            let show_spent = args.is_present("show-spent");
            wallet.lock().unwrap().outputs(show_spent)?;
        }
        Some("repost") => {
            let args = matches.subcommand_matches("repost").unwrap();
            let id = args.value_of("id").unwrap();
            let id = id.parse::<u32>().map_err(|_| {
                ErrorKind::InvalidTxId(id.to_string())
            })?;
            wallet.lock().unwrap().repost(id, false)?;
        }
        Some("cancel") => {
            let args = matches.subcommand_matches("cancel").unwrap();
            let id = args.value_of("id").unwrap();
            let id = id.parse::<u32>().map_err(|_| {
                ErrorKind::InvalidTxId(id.to_string())
            })?;
            wallet.lock().unwrap().cancel(id)?;
        }
        Some("receive") => {
            let args = matches.subcommand_matches("receive").unwrap();
            let input = args.value_of("file").unwrap();
            let mut file = File::open(input)?;
            let mut slate = String::new();
            file.read_to_string(&mut slate)?;
            let mut slate: Slate = serde_json::from_str(&slate)?;
            let mut file = File::create(&format!("{}.response", input))?;
            wallet.lock().unwrap().process_sender_initiated_slate(&mut slate)?;
            cli_message!("{} received.", input);
            file.write_all(serde_json::to_string(&slate).unwrap().as_bytes())?;
            cli_message!("{}.response created successfully.", input);
        }
        Some("finalize") => {
            let args = matches.subcommand_matches("finalize").unwrap();
            let input = args.value_of("file").unwrap();
            let mut file = File::open(input)?;
            let mut slate = String::new();
            file.read_to_string(&mut slate)?;
            let mut slate: Slate = serde_json::from_str(&slate)?;
            wallet.lock().unwrap().finalize_slate(&mut slate)?;
            cli_message!("{} finalized.", input);
        }
        Some("send") => {
            let args = matches.subcommand_matches("send").unwrap();
            let to = args.value_of("to");
            let input = args.value_of("file");
            let message = args.value_of("message").map(|s| s.to_string());

            let change_outputs = args.value_of("change-outputs").unwrap_or("1");
            let change_outputs = usize::from_str_radix(change_outputs, 10)
                .map_err(|_| {
                    ErrorKind::InvalidNumOutputs(change_outputs.to_string())
                })?;

            let amount = args.value_of("amount").unwrap();
            let amount = core::amount_from_hr_string(amount).map_err(|_| {
                ErrorKind::InvalidAmount(amount.to_string())
            })?;

            if let Some(input) = input {
                let mut file = File::create(input)?;
                let slate = wallet.lock().unwrap().initiate_send_tx(amount, 10, "smallest", change_outputs, 500, message)?;
                file.write_all(serde_json::to_string(&slate).unwrap().as_bytes())?;
                cli_message!("{} created successfully.", input);
                return Ok(true);
            }

            let mut to = to.unwrap().to_string();
            if to.starts_with("@") {
                let contact = address_book.lock().unwrap().get_contact(&to[1..])?;
                to = contact.get_address().to_string();
            }

            // try parse as a general address and fallback to grinbox address
            let address = Address::parse(&to);
            let address: Result<Box<Address>> = match address {
                Ok(address) => Ok(address),
                Err(e) => {
                    Ok(Box::new(GrinboxAddress::from_str(&to).map_err(|_| e)?) as Box<Address>)
                }
            };

            let to = address?;
            let slate: Result<Slate> = match to.address_type() {
                AddressType::Keybase => {
                    if let Some((publisher, _)) = keybase_broker {
                        let slate = wallet.lock().unwrap().initiate_send_tx(amount, 10, "smallest", change_outputs, 500, message)?;
                        let mut keybase_address = contacts::KeybaseAddress::from_str(&to.to_string())?;
                        keybase_address.topic = Some(broker::TOPIC_SLATE_NEW.to_string());
                        publisher.post_slate(&slate, keybase_address.borrow())?;
                        Ok(slate)
                    } else {
                        Err(ErrorKind::ClosedListener("keybase".to_string()))?
                    }
                }
                AddressType::Grinbox => {
                    if let Some((publisher, _)) = grinbox_broker {
                        let slate = wallet.lock().unwrap().initiate_send_tx(amount, 10, "smallest", change_outputs, 500, message)?;
                        publisher.post_slate(&slate, to.borrow())?;
                        Ok(slate)
                    } else {
                        Err(ErrorKind::ClosedListener("grinbox".to_string()))?
                    }
                }
            };

            let slate = slate?;

            cli_message!("slate [{}] for [{}] grins sent successfully to [{}]",
                slate.id.to_string().bright_green(),
                core::amount_to_hr_string(slate.amount, false).bright_green(),
                to.stripped().bright_green()
            );
        }
        Some("invoice") => {
            let args = matches.subcommand_matches("invoice").unwrap();
            let to = args.value_of("to").unwrap();
            let outputs = args.value_of("outputs").unwrap_or("1");
            let outputs = usize::from_str_radix(outputs, 10)
                .map_err(|_| {
                    ErrorKind::InvalidNumOutputs(outputs.to_string())
                })?;
            let amount = args.value_of("amount").unwrap();
            let amount = core::amount_from_hr_string(amount).map_err(|_| {
                ErrorKind::InvalidAmount(amount.to_string())
            })?;

            let mut to = to.to_string();
            if to.starts_with("@") {
                let contact = address_book.lock().unwrap().get_contact(&to[1..])?;
                to = contact.get_address().to_string();
            }

            // try parse as a general address
            let address = Address::parse(&to);
            let address: Result<Box<Address>> = match address {
                Ok(address) => Ok(address),
                Err(e) => {
                    Ok(Box::new(GrinboxAddress::from_str(&to).map_err(|_| e)?) as Box<Address>)
                }
            };

            let to = address?;
            let slate: Result<Slate> = match to.address_type() {
                AddressType::Keybase => {
                    if let Some((publisher, _)) = keybase_broker {
                        let slate = wallet.lock().unwrap().initiate_receive_tx(amount, outputs)?;
                        publisher.post_slate(&slate, to.borrow())?;
                        Ok(slate)
                    } else {
                        Err(ErrorKind::ClosedListener("keybase".to_string()))?
                    }
                }
                AddressType::Grinbox => {
                    if let Some((publisher, _)) = grinbox_broker {
                        let slate = wallet.lock().unwrap().initiate_receive_tx(amount, outputs)?;
                        publisher.post_slate(&slate, to.borrow())?;
                        Ok(slate)
                    } else {
                        Err(ErrorKind::ClosedListener("grinbox".to_string()))?
                    }
                }
            };

            let slate = slate?;
            cli_message!("invoice slate [{}] for [{}] grins sent successfully to [{}]",
                slate.id.to_string().bright_green(),
                core::amount_to_hr_string(slate.amount, false).bright_green(),
                to.stripped().bright_green()
            );
        }
        Some("restore") => {
            if keybase_broker.is_some() || grinbox_broker.is_some() {
                return Err(ErrorKind::HasListener.into());
            }
            println!("restoring... please wait as this could take a few minutes to complete.");
            let args = matches.subcommand_matches("restore").unwrap();
            let passphrase = args.value_of("passphrase").unwrap_or("");
            {
                let mut w = wallet.lock().unwrap();
                if let Some(words) = args.values_of("words") {
                    let words: Vec<&str> = words.collect();
                    w.restore_seed(config, &words, passphrase)?;
                }
                w.init(config, "default", passphrase)?;
                w.restore_state()?;
            }

            derive_address_key(config, wallet, grinbox_broker)?;
            if passphrase.is_empty() {
                println!("{}: wallet with no passphrase.", "WARNING".bright_yellow());
            }

            cli_message!("wallet restoration done!");

            return Ok(false);
        }
        Some("check") => {
            if keybase_broker.is_some() || grinbox_broker.is_some() {
                return Err(ErrorKind::HasListener.into());
            }
            println!("checking and repairing... please wait as this could take a few minutes to complete.");
            if let Ok(mut wallet) = wallet.lock() {
                wallet.check_repair()?;
                cli_message!("check and repair done!");
            }
            return Ok(false);
        }
        Some(subcommand) => {
            cli_message!("{}: subcommand `{}` not implemented!", "ERROR".bright_red(), subcommand.bright_green());
        }
        None => {}
    };
    Ok(true)
}