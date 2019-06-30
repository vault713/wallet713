use colored::Colorize;
use grin_core::core::amount_to_hr_string;
use std::marker::Send;
use crate::common::{Arc, Keychain, Mutex, Error};
use crate::contacts::{Address, AddressBook, AddressType, GrinboxAddress};
use crate::wallet::api::{Foreign, Owner};
use crate::wallet::types::{NodeClient, Slate, VersionedSlate, TxProof, WalletBackend};
use crate::wallet::Container;
use crate::cli_message;

pub enum CloseReason {
    Normal,
    Abnormal(Error),
}

pub trait Publisher: Send {
    fn post_slate(&self, slate: &VersionedSlate, to: &Address) -> Result<(), Error>;
}

pub trait Subscriber {
    fn start<W, C, K, P>(&mut self, handler: Controller<W, C, K, P>) -> Result<(), Error>
        where
            W: WalletBackend<C, K>,
            C: NodeClient,
            K: Keychain,
            P: Publisher,
    ;
    fn stop(&self);
    fn is_running(&self) -> bool;
}

pub trait SubscriptionHandler: Send {
    fn on_open(&self);
    fn on_slate(&self, from: &Address, slate: &VersionedSlate, proof: Option<&mut TxProof>);
    fn on_close(&self, result: CloseReason);
    fn on_dropped(&self);
    fn on_reestablished(&self);
}

pub struct Controller<W, C, K, P>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
        P: Publisher,
{
    name: String,
    owner: Owner<W, C, K>,
    foreign: Foreign<W, C, K>,
    publisher: P,
}

impl<W, C, K, P> Controller<W, C, K, P>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
        P: Publisher,
{
    pub fn new(
        name: &str,
        container: Arc<Mutex<Container<W, C, K>>>,
        publisher: P,
    ) -> Result<Self, Error> {
        Ok(Self {
            name: name.to_string(),
            owner: Owner::new(container.clone()),
            foreign: Foreign::new(container),
            publisher,
        })
    }

    fn process_incoming_slate(
        &self,
        address: Option<String>,
        slate: &mut Slate,
        tx_proof: Option<&mut TxProof>,
    ) -> Result<bool, Error> {
        if slate.num_participants > slate.participant_data.len() {
            if slate.tx.inputs().len() == 0 {
                // TODO: invoicing
            } else {
                *slate = self.foreign.receive_tx(slate, None, None)?;
            }
            Ok(false)
        } else {
            self.owner.finalize_tx(slate, tx_proof)?;
            Ok(true)
        }
    }
}

impl<W, C, K, P> SubscriptionHandler for Controller<W, C, K, P>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
        P: Publisher,
{
    fn on_open(&self) {
//        cli_message!("Listener for {} started", self.name.bright_green());
    }

    fn on_slate(&self, from: &Address, slate: &VersionedSlate, tx_proof: Option<&mut TxProof>) {
        let mut display_from = from.stripped();
        /*if let Ok(contact) = self
            .address_book
            .lock()
            .get_contact_by_address(&from.to_string())
        {
            display_from = contact.get_name().to_string();
        }*/

        let version = slate.version();
        let mut slate: Slate = slate.clone().into();

        if slate.num_participants > slate.participant_data.len() {
            cli_message!(
                "Slate [{}] received from [{}] for [{}] grins",
                slate.id.to_string().bright_green(),
                display_from.bright_green(),
                amount_to_hr_string(slate.amount, false).bright_green()
            );
        } else {
            cli_message!(
                "Slate [{}] received back from [{}] for [{}] grins",
                slate.id.to_string().bright_green(),
                display_from.bright_green(),
                amount_to_hr_string(slate.amount, false).bright_green()
            );
        };

        if from.address_type() == AddressType::Grinbox {
            GrinboxAddress::from_str(&from.to_string()).expect("invalid grinbox address");
        }

        let result = self
            .process_incoming_slate(Some(from.to_string()), &mut slate, tx_proof)
            .and_then(|is_finalized| {
                if !is_finalized {
                    let id = slate.id.clone();
                    let slate = VersionedSlate::into_version(slate, version);

                    self.publisher
                        .post_slate(&slate, from)
                        .map_err(|e| {
                            cli_message!("{}: {}", "ERROR".bright_red(), e);
                            e
                        })
                        .expect("failed posting slate!");
                    cli_message!(
                        "slate [{}] sent back to [{}] successfully",
                        id.to_string().bright_green(),
                        display_from.bright_green()
                    );
                } else {
                    cli_message!(
                        "slate [{}] finalized successfully",
                        slate.id.to_string().bright_green()
                    );
                }
                Ok(())
            });

        match result {
            Ok(()) => {}
            Err(e) => cli_message!("{}", e),
        }
    }

    fn on_close(&self, reason: CloseReason) {
        match reason {
            CloseReason::Normal => {
                //println!("Listener for {} stopped", self.name.bright_green())
            },
            CloseReason::Abnormal(_) => cli_message!(
                "{}: listener [{}] stopped unexpectedly",
                "ERROR".bright_red(),
                self.name.bright_green()
            ),
        }
    }

    fn on_dropped(&self) {
        cli_message!("{}: listener [{}] lost connection. it will keep trying to restore connection in the background.", "WARNING".bright_yellow(), self.name.bright_green())
    }

    fn on_reestablished(&self) {
        cli_message!(
            "{}: listener [{}] reestablished connection.",
            "INFO".bright_blue(),
            self.name.bright_green()
        )
    }
}