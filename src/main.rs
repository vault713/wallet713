#[macro_use] extern crate failure;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;
extern crate serde;
extern crate clap;
extern crate colored;
extern crate ws;
extern crate futures;
extern crate tokio;
extern crate secp256k1;
extern crate rand;
extern crate sha2;
extern crate digest;
extern crate uuid;
extern crate regex;
extern crate rustyline;

extern crate grin_wallet;
extern crate grin_keychain;
extern crate grin_util;
extern crate grin_core;
extern crate grin_store;

use std::sync::{Arc, Mutex};
use std::io::{Read, Write};
use std::fs::File;
use std::path::Path;
use clap::{App, Arg, ArgMatches};
use colored::*;
use rustyline::Editor;

use grin_core::{core};
use grin_core::global::{ChainTypes, set_mining_mode};

#[macro_use] mod common;
mod broker;
mod wallet;
mod contacts;
mod cli;

use common::config::Wallet713Config;
use common::{Wallet713Error, Result};
use common::crypto::*;
use wallet::Wallet;
use cli::Parser;

use contacts::{Address, AddressType, GrinboxAddress, Contact, AddressBook, LMDBBackend};

const CLI_HISTORY_PATH: &str = ".history";

fn do_config(args: &ArgMatches, chain: &Option<ChainTypes>, silent: bool) -> Result<Wallet713Config> {
	let mut config;
	let mut any_matches = false;
    let config_path = args.value_of("config-path");
    let exists = Wallet713Config::exists(config_path, &chain)?;
	if exists {
		config = Wallet713Config::from_file(config_path, &chain)?;
	} else {
		config = Wallet713Config::default(&chain)?;
        if config.grin_node_secret.is_none() {
            println!("{}: initilized new configuration with no api secret!", "WARNING".bright_yellow())
        }
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
            Wallet713Error::NumberParsingError
        })?;
        config.grinbox_port = Some(port);
        any_matches = true;
    }

    if let Some(account) = args.value_of("private-key") {
        config.grinbox_private_key = account.to_string();
        any_matches = true;
    }

    if let Some(node_uri) = args.value_of("node-uri") {
        config.grin_node_uri = node_uri.to_string();
        any_matches = true;
    }

    if let Some(node_secret) = args.value_of("node-secret") {
        config.grin_node_secret = Some(node_secret.to_string());
        any_matches = true;
    }

    if !exists || args.is_present("generate-keys") {
        let (pr, _) = generate_keypair();
        config.grinbox_private_key = pr.to_string();
        println!("{}: {}", "Your new 713.grinbox address".bright_yellow(), config.get_grinbox_address()?.stripped().bright_green());        any_matches = exists;
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
            .contact_iter()
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

fn welcome(args: &ArgMatches) -> Result<Wallet713Config> {
    let chain: Option<ChainTypes> = if args.is_present("floonet") {
        Some(ChainTypes::Floonet)
    } else {
        println!("Mainnet not ready yet! In the meantime run `wallet713 --floonet`");
        std::process::exit(1);
    };
    let config = do_config(args, &chain, true)?;

	print!("{}", WELCOME_HEADER.bright_yellow().bold());
    println!("{}: {}", "Your 713.grinbox address".bright_yellow(), config.get_grinbox_address()?.stripped().bright_green());
    println!("{}", WELCOME_FOOTER.bright_blue().bold());

    Ok(config)
}

use std::borrow::Borrow;
use grin_core::libtx::slate::Slate;
use broker::{GrinboxSubscriber, GrinboxPublisher, KeybasePublisher, KeybaseSubscriber, SubscriptionHandler, Subscriber, Publisher, CloseReason};

struct Controller {
    chain: ChainTypes,
    name: String,
    wallet: Arc<Mutex<Wallet>>,
    address_book: Arc<Mutex<AddressBook>>,
    publisher: Box<Publisher + Send>,
}

impl Controller {
    pub fn new(name: &str, chain: ChainTypes, wallet: Arc<Mutex<Wallet>>, address_book: Arc<Mutex<AddressBook>>, publisher: Box<Publisher + Send>) -> Result<Self> {
        Ok(Self {
            name: name.to_string(),
            wallet,
            address_book,
            publisher,
            chain
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
        if let Ok(contact) = self.address_book.lock().unwrap().get_contact_by_address(&display_from) {
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
            let grinbox_address = GrinboxAddress::from_str(&from.to_string()).unwrap();
            let chain = grinbox_address.get_chain_type().expect("unknown chain type for address! ignoring slate.");
            if chain != self.chain {
                cli_message!("address [{}] is on the wrong chain!", grinbox_address.stripped().bright_green());
                return;
            }
        }

        let result = self.process_incoming_slate(slate).and_then(|is_finalized| {
            if !is_finalized {
                self.publisher.post_slate(slate, from).expect("failed posting slate!");
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
            Ok(()) => {},
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
    let grinbox_publisher = GrinboxPublisher::new(&grinbox_address, &grinbox_secret_key)?;
    let grinbox_subscriber = GrinboxSubscriber::new(&grinbox_address, &grinbox_secret_key).expect("could not start grinbox subscriber!");
    let cloned_publisher = grinbox_publisher.clone();
    let mut cloned_subscriber = grinbox_subscriber.clone();
    let chain = config.chain.clone().unwrap_or(ChainTypes::Mainnet);
    std::thread::spawn(move || {
        let controller = Controller::new(
            &grinbox_address.stripped(),
            chain,
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
    let keybase_subscriber = KeybaseSubscriber::new().expect("could not start keybase subscriber!");
    let keybase_publisher = KeybasePublisher::new(config.default_keybase_ttl.clone()).expect("could not start keybase publisher!");;

    let mut cloned_subscriber = keybase_subscriber.clone();
    let cloned_publisher = keybase_publisher.clone();
    let chain = config.chain.clone().unwrap_or(ChainTypes::Mainnet);
    std::thread::spawn(move || {
        let controller = Controller::new(
            "keybase",
            chain,
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
        .arg(Arg::from_usage("[account] -a, --account=<account> 'the account to use'"))
        .arg(Arg::from_usage("[passphrase] -p, --passphrase=<passphrase> 'the passphrase to use'"))
        .arg(Arg::from_usage("[floonet] -f, --floonet 'use floonet'"))
        .get_matches();

	let mut config = welcome(&matches).unwrap_or_else(|e| {
        panic!("{}: could not read or create config! {}", "ERROR".bright_red(), e);
    });

    set_mining_mode(config.chain.clone().unwrap_or(ChainTypes::Mainnet));

    let data_path_buf = config.get_data_path().unwrap();
    let data_path = data_path_buf.to_str().unwrap();

    let address_book_backend = LMDBBackend::new(data_path).expect("could not create address book backend!");
    let address_book = AddressBook::new(Box::new(address_book_backend)).expect("could not create an address book!");
    let address_book = Arc::new(Mutex::new(address_book));

    let wallet = Wallet::new(config.max_auto_accept_invoice);
    let wallet = Arc::new(Mutex::new(wallet));

    let mut grinbox_broker: Option<(GrinboxPublisher, GrinboxSubscriber)> = None;
    let mut keybase_broker: Option<(KeybasePublisher, KeybaseSubscriber)> = None;

    let account = matches.value_of("account");
    let passphrase = matches.value_of("passphrase");
    let result = wallet.lock().unwrap().unlock(&config, account.unwrap_or("default"), passphrase.unwrap_or(""));
    if account.is_some() || passphrase.is_some() {
        if let Err(err) = result {
            cli_message!("{}: {}", "ERROR".bright_red(), err);
        }
    }

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
            },
            Err(_) => {
                break;
            }
        }
    }

    if let Some(path) = Path::new(wallet713_home_path).join(CLI_HISTORY_PATH).to_str() {
        rl.save_history(path).is_ok();
    }
}

fn do_command(command: &str, config: &mut Wallet713Config, wallet: Arc<Mutex<Wallet>>, address_book: Arc<Mutex<AddressBook>>, keybase_broker: &mut Option<(KeybasePublisher, KeybaseSubscriber)>, grinbox_broker: &mut Option<(GrinboxPublisher, GrinboxSubscriber)>) -> Result<bool> {
    let matches = Parser::parse(command)?;
    match matches.subcommand_name() {
        Some("config") => {
            let args = matches.subcommand_matches("config").unwrap();
            *config = do_config(args, &config.chain, false)?;
        },
        Some("init") => {
            let passphrase = matches.subcommand_matches("init").unwrap().value_of("passphrase").unwrap_or("");
            wallet.lock().unwrap().init(config, "default", passphrase)?;
            if passphrase.is_empty() {
                cli_message!("{}: wallet with no passphrase.", "WARNING".bright_yellow());
            }
            return Ok(false);
        },
        Some("lock") => {
            wallet.lock().unwrap().lock();
        },
        Some("unlock") => {
            let args = matches.subcommand_matches("unlock").unwrap();
            let account = args.value_of("account").unwrap_or("default");
            let passphrase = args.value_of("passphrase").unwrap_or("");
            wallet.lock().unwrap().unlock(config, account, passphrase)?;
            return Ok(false);
        },
        Some("accounts") => {
            wallet.lock().unwrap().list_accounts()?;
        },
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
        },
        Some("listen") => {
            let grinbox = matches.subcommand_matches("listen").unwrap().is_present("grinbox");
            let keybase = matches.subcommand_matches("listen").unwrap().is_present("keybase");
            if grinbox || !keybase {
                let is_running = match grinbox_broker {
                    Some((_, subscriber)) => subscriber.is_running(),
                    _ => false
                };
                if is_running {
                    Err(Wallet713Error::AlreadyListening("grinbox".to_string()))?
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
                    Err(Wallet713Error::AlreadyListening("keybase".to_string()))?
                } else {
                    let (publisher, subscriber) = start_keybase_listener(config, wallet.clone(), address_book.clone())?;
                    *keybase_broker = Some((publisher, subscriber));
                }
            }
        },
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
                    Err(Wallet713Error::ClosedListener("grinbox".to_string()))?
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
                    Err(Wallet713Error::ClosedListener("keybase".to_string()))?
                }
            }
        },
        Some("info") => {
            wallet.lock().unwrap().info()?;
        },
        Some("txs") => {
            wallet.lock().unwrap().txs()?;
        },
        Some("contacts") => {
            let arg_matches = matches.subcommand_matches("contacts").unwrap();
            do_contacts(&arg_matches, address_book.clone())?;
        },
        Some("outputs") => {
            let args = matches.subcommand_matches("outputs").unwrap();
            let show_spent = args.is_present("show-spent");
            wallet.lock().unwrap().outputs(show_spent)?;
        },
        Some("repost") => {
            let args = matches.subcommand_matches("repost").unwrap();
            let id = args.value_of("id").unwrap();
            let id = id.parse::<u32>().map_err(|_| {
                Wallet713Error::InvalidTxId(id.to_string())
            })?;
            wallet.lock().unwrap().repost(id, false)?;
        },
        Some("cancel") => {
            let args = matches.subcommand_matches("cancel").unwrap();
            let id = args.value_of("id").unwrap();
            let id = id.parse::<u32>().map_err(|_| {
                Wallet713Error::InvalidTxId(id.to_string())
            })?;
            wallet.lock().unwrap().cancel(id)?;
        },
        Some("receive") => {
            let args = matches.subcommand_matches("receive").unwrap();
            let input = args.value_of("input").unwrap();
            let mut file = File::open(input)?;
            let mut slate = String::new();
            file.read_to_string(&mut slate)?;
            let mut slate: Slate = serde_json::from_str(&slate)?;
            let mut file = File::create(&format!("{}.{}", input, "response"))?;
            wallet.lock().unwrap().process_sender_initiated_slate(&mut slate)?;
            file.write_all(serde_json::to_string(&slate).unwrap().as_bytes())?;
        },
        Some("send") => {
            let args = matches.subcommand_matches("send").unwrap();
            let to = args.value_of("to").unwrap();
            let message = args.value_of("message").map(|s| s.to_string());

            let change_outputs = args.value_of("change-outputs").unwrap_or("1");
            let change_outputs = usize::from_str_radix(change_outputs, 10)
                .map_err(|_| {
                    Wallet713Error::InvalidNumOutputs(change_outputs.to_string())
                })?;

            let amount = args.value_of("amount").unwrap();
            let amount = core::amount_from_hr_string(amount).map_err(|_| {
                Wallet713Error::InvalidAmount(amount.to_string())
            })?;

            let mut to = to.to_string();
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
                        Err(Wallet713Error::ClosedListener("keybase".to_string()))?
                    }
                },
                AddressType::Grinbox => {
                    if let Some((publisher, _)) = grinbox_broker {
                        let slate = wallet.lock().unwrap().initiate_send_tx(amount, 10, "smallest", change_outputs, 500, message)?;
                        publisher.post_slate(&slate, to.borrow())?;
                        Ok(slate)
                    } else {
                        Err(Wallet713Error::ClosedListener("grinbox".to_string()))?
                    }
                },
            };

            let slate = slate?;

            cli_message!("slate [{}] for [{}] grins sent successfully to [{}]",
                slate.id.to_string().bright_green(),
                core::amount_to_hr_string(slate.amount, false).bright_green(),
                to.stripped().bright_green()
            );
        },
        Some("invoice") => {
            let args = matches.subcommand_matches("invoice").unwrap();
            let to = args.value_of("to").unwrap();
            let outputs = args.value_of("outputs").unwrap_or("1");
            let outputs = usize::from_str_radix(outputs, 10)
                .map_err(|_| {
                    Wallet713Error::InvalidNumOutputs(outputs.to_string())
                })?;
            let amount = args.value_of("amount").unwrap();
            let amount = core::amount_from_hr_string(amount).map_err(|_| {
                Wallet713Error::InvalidAmount(amount.to_string())
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
                        Err(Wallet713Error::ClosedListener("keybase".to_string()))?
                    }
                },
                AddressType::Grinbox => {
                    if let Some((publisher, _)) = grinbox_broker {
                        let slate = wallet.lock().unwrap().initiate_receive_tx(amount, outputs)?;
                        publisher.post_slate(&slate, to.borrow())?;
                        Ok(slate)
                    } else {
                        Err(Wallet713Error::ClosedListener("grinbox".to_string()))?
                    }
                },
            };

            let slate = slate?;
            cli_message!("invoice slate [{}] for [{}] grins sent successfully to [{}]",
                slate.id.to_string().bright_green(),
                core::amount_to_hr_string(slate.amount, false).bright_green(),
                to.stripped().bright_green()
            );
        },
        Some("restore") => {
            cli_message!("restoring... please wait as this could take a few minutes to complete.");
            if let Ok(mut wallet) = wallet.lock() {
                let args = matches.subcommand_matches("restore").unwrap();
                let passphrase = args.value_of("passphrase").unwrap_or("");
                if let Some(words) = args.values_of("words") {
                    let words: Vec<&str> = words.collect();
                    wallet.restore_seed(config, &words, passphrase)?;
                }
                wallet.init(config, "default", passphrase)?;
                wallet.restore_state()?;
                cli_message!("wallet restoration done!");
                if passphrase.is_empty() {
                    cli_message!("{}: wallet with no passphrase.", "WARNING".bright_yellow());
                }
            }
            return Ok(false);
        },
        Some("check") => {
            cli_message!("checking and repairing... please wait as this could take a few minutes to complete.");
            if let Ok(mut wallet) = wallet.lock() {
                wallet.check_repair()?;
                cli_message!("check and repair done!");
            }
            return Ok(false);
        },
        Some(subcommand) => {
            cli_message!("{}: subcommand `{}` not implemented!", "ERROR".bright_red(), subcommand.bright_green());
        },
        None => {},
    };
    Ok(true)
}