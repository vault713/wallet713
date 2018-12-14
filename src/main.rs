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

extern crate grin_wallet;
extern crate grin_keychain;
extern crate grin_util;
extern crate grin_core;
extern crate grin_store;

use std::sync::{Arc, Mutex};
use clap::ArgMatches;
use colored::*;

use grin_core::{core};

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

fn do_config(args: &ArgMatches, silent: bool) -> Result<Wallet713Config> {
	let mut config;
	let mut any_matches = false;
    let exists = Wallet713Config::exists();
	if exists {
		config = Wallet713Config::from_file()?;
	} else {
		config = Wallet713Config::default()?;
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
        config.grinbox_port = u16::from_str_radix(port, 10).map_err(|_| {
            Wallet713Error::NumberParsingError
        })?;
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
        any_matches = exists;
    }

	config.to_file()?;

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
        let address = Address::parse(address)?;
        let contact = Contact::new(name, address)?;
        address_book.add_contact(&contact)?;
    } else if let Some(add_args) = args.subcommand_matches("remove") {
        let name = add_args.value_of("name").unwrap();
        address_book.remove_contact(name)?;
    } else {
        let contacts: Vec<()> = address_book
            .contact_iter()
            .map(|contact| {
                println!("@{} = {}", contact.get_name(), contact.get_address());
                ()
            })
            .collect();

        if contacts.len() == 0 {
            println!("your contact list is empty. consider using `contacts add` to add a new contact.");
        }
    }
    Ok(())
}

const WELCOME_HEADER: &str = r#"
Welcome to wallet713

"#;

const WELCOME_FOOTER: &str = r#"Use `listen` to connect to grinbox or `help` to see available commands
"#;

fn welcome() -> Result<Wallet713Config> {
    let config = do_config(&ArgMatches::new(), true)?;

    let secret_key = SecretKey::from_hex(&config.grinbox_private_key)?;
    let public_key = common::crypto::public_key_from_secret_key(&secret_key);
    let public_key = public_key.to_base58_check(common::crypto::BASE58_CHECK_VERSION_GRIN_TX.to_vec());

	print!("{}", WELCOME_HEADER.bright_yellow().bold());
    println!("{}: {}", "Your 713.grinbox address".bright_yellow(), config.get_grinbox_address()?.stripped().bright_green());
	println!("{}", WELCOME_FOOTER.bright_blue().bold());

    Ok(config)
}

use broker::{GrinboxSubscriber, GrinboxPublisher, KeybasePublisher, KeybaseSubscriber, SubscriptionHandler, Subscriber, Publisher};
use grin_core::libtx::slate::Slate;
use std::borrow::Borrow;
use contacts::KeybaseAddress;
use common::crypto::{SecretKey, Hex};

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
}

impl SubscriptionHandler for Controller {
    fn on_open(&self) {
        cli_message!("listener established for [{}]", self.name.bright_green());
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
        let is_finalized = self.wallet.lock().unwrap().process_slate("default", "", slate).expect("failed processing slate!");
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
    }

    fn on_close(&self) {
        cli_message!("listener [{}] closed", self.name.bright_green());
    }
}

fn start_grinbox_listener(config: &Wallet713Config, wallet: Arc<Mutex<Wallet>>, address_book: Arc<Mutex<AddressBook>>) -> Result<GrinboxPublisher> {
    let grinbox_address = config.get_grinbox_address()?;
    let grinbox_secret_key = config.get_grinbox_secret_key()?;
    let grinbox_publisher = GrinboxPublisher::new(&grinbox_address, &grinbox_secret_key)?;
    std::thread::spawn(move || {
        let grinbox_subscriber = GrinboxSubscriber::new(&grinbox_address, &grinbox_secret_key).expect("could not start grinbox subscriber!");
        let grinbox_publisher = GrinboxPublisher::new(&grinbox_address, &grinbox_secret_key).expect("could not start grinbox listener!");
        let controller = Controller::new(&grinbox_address.stripped(), wallet.clone(), address_book.clone(),Box::new(grinbox_publisher)).expect("could not start grinbox controller!");
        grinbox_subscriber.subscribe(Box::new(controller)).expect("something went wrong!");
    });
    Ok(grinbox_publisher)
}

fn start_keybase_listener(config: &Wallet713Config, wallet: Arc<Mutex<Wallet>>, address_book: Arc<Mutex<AddressBook>>) -> Result<KeybasePublisher> {
    let keybase_publisher = KeybasePublisher::new()?;
    std::thread::spawn(move || {
        let keybase_subscriber = KeybaseSubscriber::new().expect("could not start keybase subscriber!");
        let keybase_publisher = KeybasePublisher::new().expect("could not start keybase publisher!");;
        let controller = Controller::new("keybase", wallet.clone(), address_book.clone(), Box::new(keybase_publisher)).expect("could not start keybase controller!");;
        keybase_subscriber.subscribe(Box::new(controller)).expect("something went wrong!");
    });
    Ok(keybase_publisher)
}

fn main() {
	let mut config = welcome().unwrap_or_else(|e| {
        panic!("{}: could not read or create config! {}", "ERROR".bright_red(), e);
    });

    let address_book_backend = LMDBBackend::new(&config.wallet713_data_path).expect("could not create address book backend!");
    let address_book = AddressBook::new(Box::new(address_book_backend)).expect("could not create an address book!");
    let address_book = Arc::new(Mutex::new(address_book));

    let wallet = Wallet::new();
    let wallet = Arc::new(Mutex::new(wallet));

    let mut grinbox_publisher = None;
    let mut keybase_publisher = None;
    loop {
        cli_message!();
        let mut command = String::new();
        std::io::stdin().read_line(&mut command).expect("oops!");
        let result = do_command(&command, &mut config, wallet.clone(), address_book.clone(), &mut keybase_publisher, &mut grinbox_publisher);
        if let Err(err) = result {
            cli_message!("{}: {}", "ERROR".bright_red(), err);
        }
    }
}

fn do_command(command: &str, config: &mut Wallet713Config, wallet: Arc<Mutex<Wallet>>, address_book: Arc<Mutex<AddressBook>>, keybase_publisher: &mut Option<KeybasePublisher>, grinbox_publisher: &mut Option<GrinboxPublisher>) -> Result<()> {
    let account = "default".to_owned();
    let matches = Parser::parse(command)?;
    match matches.subcommand_name() {
        Some("exit") => {
            std::process::exit(0);
        },
        Some("config") => {
            *config = do_config(matches.subcommand_matches("config").unwrap(), false)?;
        },
        Some("init") => {
            let password = matches.subcommand_matches("init").unwrap().value_of("password").unwrap_or("");
            wallet.lock().unwrap().init(password)?;
        },
        Some("listen") => {
            let grinbox = matches.subcommand_matches("listen").unwrap().is_present("grinbox");
            let keybase = matches.subcommand_matches("listen").unwrap().is_present("keybase");
            if grinbox || !keybase {
                if grinbox_publisher.is_some() {
                    Err(Wallet713Error::AlreadyListening("grinbox".to_string()))?
                } else {
                    let publisher = start_grinbox_listener(config, wallet.clone(), address_book.clone())?;
                    *grinbox_publisher = Some(publisher);
                }
            }
            if keybase {
                if keybase_publisher.is_some() {
                    Err(Wallet713Error::AlreadyListening("keybase".to_string()))?
                } else {
                    let publisher = start_keybase_listener(config, wallet.clone(), address_book.clone())?;
                    *keybase_publisher = Some(publisher);
                }
            }
        },
        Some("stop") => {
            //TODO: stop listeners
        },
        Some("info") => {
            let password = matches.subcommand_matches("info").unwrap().value_of("password").unwrap_or("");
            wallet.lock().unwrap().info(password, &account[..])?;
        },
        Some("txs") => {
            let password = matches.subcommand_matches("txs").unwrap().value_of("password").unwrap_or("");
            wallet.lock().unwrap().txs(password, &account[..])?;
        },
        Some("contacts") => {
            let arg_matches = matches.subcommand_matches("contacts").unwrap();
            do_contacts(&arg_matches, address_book.clone())?;
        },
        Some("outputs") => {
            let password = matches.subcommand_matches("outputs").unwrap().value_of("password").unwrap_or("");
            let show_spent = matches.subcommand_matches("outputs").unwrap().is_present("show-spent");
            wallet.lock().unwrap().outputs(password, &account[..], show_spent)?;
        },
        Some("repost") => {
            let password = matches.subcommand_matches("repost").unwrap().value_of("password").unwrap_or("");
            let id = matches.subcommand_matches("repost").unwrap().value_of("id").unwrap();
            let id = id.parse::<u32>().map_err(|_| {
                Wallet713Error::InvalidTxId(id.to_string())
            })?;
            wallet.lock().unwrap().repost(password, id, false)?;
        },
        Some("cancel") => {
            let password = matches.subcommand_matches("cancel").unwrap().value_of("password").unwrap_or("");
            let id = matches.subcommand_matches("cancel").unwrap().value_of("id").unwrap();
            let id = id.parse::<u32>().map_err(|_| {
                Wallet713Error::InvalidTxId(id.to_string())
            })?;
            wallet.lock().unwrap().cancel(password, id)?;
        },
        Some("send") => {
            let args = matches.subcommand_matches("send").unwrap();
            let password = args.value_of("password").unwrap_or("");
            let to = args.value_of("to").unwrap();
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
                    if let Some(keybase_publisher) = keybase_publisher {
                        let slate = wallet.lock().unwrap().initiate_send_tx(password, &account[..], amount, 10, "all", 1, 500)?;
                        keybase_publisher.post_slate(&slate, to.borrow());
                        Ok(slate)
                    } else {
                        Err(Wallet713Error::ClosedListener("keybase".to_string()))?
                    }
                },
                AddressType::Grinbox => {
                    if let Some(grinbox_publisher) = grinbox_publisher {
                        let slate = wallet.lock().unwrap().initiate_send_tx(password, &account[..], amount, 10, "all", 1, 500)?;
                        grinbox_publisher.post_slate(&slate, to.borrow());
                        Ok(slate)
                    } else {
                        Err(Wallet713Error::ClosedListener("grinbox".to_string()))?
                    }
                },
            };

            if let Ok(slate) = slate {
                cli_message!("slate [{}] for [{}] grins sent successfully to [{}]",
                    slate.id.to_string().bright_green(),
                    core::amount_to_hr_string(slate.amount, false).bright_green(),
                    to.stripped().bright_green()
                );
            }
        },
        Some("restore") => {
            let password = matches.subcommand_matches("restore").unwrap().value_of("password").unwrap_or("");
            wallet.lock().unwrap().restore(password)?;
        },
        Some(subcommand) => {
            cli_message!("{}: subcommand `{}` not implemented!", "ERROR".bright_red(), subcommand.bright_green());
        },
        None => {},
    };
    Ok(())
}