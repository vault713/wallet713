use grin_core::libtx::slate::Slate;

use common::Error;
use contacts::Address;

pub enum CloseReason {
    Normal,
    Abnormal(Error)
}

pub trait Publisher {
    fn post_slate(&self, slate: &Slate, to: &Address) -> Result<(), Error>;
}

pub trait Subscriber {
    fn subscribe(&self, handler: Box<SubscriptionHandler + Send>) -> Result<(), Error>;
    fn unsubscribe(&self) -> Result<(), Error>;
}

pub trait SubscriptionHandler: Send {
    fn on_open(&self);
    fn on_slate(&self, from: &Address, slate: &mut Slate);
    fn on_close(&self, result: CloseReason);
}
