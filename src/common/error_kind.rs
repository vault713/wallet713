use failure::Fail;
use grin_core::libtx;

#[derive(Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "secp error")]
    Secp,
    #[fail(display = "model not found!")]
    ModelNotFound,
    #[fail(display = "error opening wallet!")]
    OpenWalletError,
    #[fail(display = "error deriving keychain!")]
    DeriveKeychainError,
    #[fail(display = "wallet should be empty before attempting restore!")]
    WalletShouldBeEmpty,
    #[fail(display = "transaction with slate id {} already received!", 0)]
    TransactionAlreadyReceived(String),
    #[fail(display = "transaction with slate id {} does not exist!", 0)]
    TransactionDoesntExist(String),
    #[fail(display = "transaction with slate id {} can not be cancelled!", 0)]
    TransactionNotCancellable(String),
    #[fail(display = "transaction cancellation error: {}", _0)]
    TransactionCancellationError(&'static str),
    #[fail(display = "internal transaction error!")]
    LibTX(libtx::ErrorKind),
    #[fail(display = "Not enough funds. Required: {}, Available: {}", needed_disp, available_disp)]
    NotEnoughFunds { available: u64, available_disp: String, needed: u64, needed_disp: String },
    #[fail(display = "Account label {} already exists!", 0)]
    AccountLabelAlreadyExists(String),
    #[fail(display = "invalid transaction id given: `{}`", 0)]
    InvalidTxId(String),
    #[fail(display = "invalid amount given: `{}`", 0)]
    InvalidAmount(String),
    #[fail(display = "invalid number of outputs given: `{}`", 0)]
    InvalidNumOutputs(String),
    #[fail(display = "could not unlock wallet! are you using the correct passphrase?")]
    WalletUnlockFailed,
    #[fail(display = "could not open wallet! use `unlock` or `init`.")]
    NoWallet,
    #[fail(display = "{} listener is closed! consider using `listen` first.", 0)]
    ClosedListener(String),
    #[fail(display = "listener for {} already started!", 0)]
    AlreadyListening(String),
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
    #[fail(display = "invalid network!")]
    InvalidBase58Version,
    #[fail(display = "invalid key!")]
    InvalidBase58Key,
    #[fail(display = "could not parse number from string!")]
    NumberParsingError,
    #[fail(display = "unknown address type `{}`!", 0)]
    UnknownAddressType(String),
    #[fail(display = "could not parse `{}` to a grinbox address!", 0)]
    GrinboxAddressParsingError(String),
    #[fail(display = "could not parse `{}` to a keybase address!", 0)]
    KeybaseAddressParsingError(String),
    #[fail(display = "could not parse `{}` to a https address!", 0)]
    HttpsAddressParsingError(String),
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
    #[fail(display = "rejecting invoice as auto invoice acceptance is turned off!")]
    DoesNotAcceptInvoices,
    #[fail(display = "rejecting invoice as amount '{}' is too big!", 0)]
    InvoiceAmountTooBig(u64),
    #[fail(display = "please stop the listeners before doing this operation")]
    HasListener,
    #[fail(display = "wallet already unlocked")]
    WalletAlreadyUnlocked,
    #[fail(display = "unable to encrypt message")]
    Encryption,
    #[fail(display = "unable to decrypt message")]
    Decryption,
    #[fail(display = "restore error")]
    Restore,
    #[fail(display = "unknown account: {}", 0)]
    UnknownAccountLabel(String),
    #[fail(display = "http request error")]
    HttpRequest
}
