pub use failure::Error;

#[derive(Debug, Fail)]
pub enum Wallet713Error {
    #[fail(display = "invalid transaction id given: `{}`", 0)]
    InvalidTxId(String),
    #[fail(display = "invalid amount given: `{}`", 0)]
    InvalidAmount(String),
    #[fail(display = "invalid number of outputs given: `{}`", 0)]
    InvalidNumOutputs(String),
    #[fail(display = "could not find a wallet! consider using `init`.")]
    NoWallet,
    #[fail(display = "{} listener is closed! consider using `listen` first.", 0)]
    ClosedListener(String),
    #[fail(display = "listener for {} already started!", 0)]
    AlreadyListening(String),
    #[fail(display = "`{}` is not a valid public key.", 0)]
    InvalidContactPublicKey(String),
    #[fail(display = "contact named `{}` already exists!", 0)]
    ContactAlreadyExists(String),
    #[fail(display = "could not find contact named `{}`!", 0)]
    ContactNotFound(String),
    #[fail(display = "invalid character!")]
    InvalidBase58Character(char, usize),
    #[fail(display = "invalid length!")]
    InvalidBase58Length,
    #[fail(display = "invalid checksum!")]
    InvalidBase58Checksum,
    #[fail(display = "could not parse number from string!")]
    NumberParsingError,
    #[fail(display = "unknown address type `{}`!", 0)]
    UnknownAddressType(String),
    #[fail(display = "address `{}` is missing a type! psst.. to send to one of your contacts use '@' before the name.", 0)]
    MissingAddressType(String),
    #[fail(display = "could not parse `{}` to a grinbox address!", 0)]
    GrinboxAddressParsingError(String),
    #[fail(display = "could not parse `{}` to a keybase address!", 0)]
    KeybaseAddressParsingError(String),
    #[fail(display = "could not send keybase message!")]
    KeybaseMessageSendError,
    #[fail(display = "failed receiving slate!")]
    GrinWalletReceiveError,
    #[fail(display = "failed verifying slate messages!")]
    GrinWalletVerifySlateMessagesError,
    #[fail(display = "failed finalizing slate!")]
    GrinWalletFinalizeError,
    #[fail(display = "failed posting transaction!")]
    GrinWalletPostError,
    #[fail(display = "keybase not found! consider installing keybase locally first.")]
    KeybaseNotFound,
    #[fail(display = "grinbox websocket terminated unexpectedly!")]
    GrinboxWebsocketAbnormalTermination,
}
