use failure::Fail;

use crate::wallet::error::ErrorKind as WalletErrorKind;

#[derive(Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "\x1b[31;1merror:\x1b[0m secp error")]
    Secp,
    #[fail(display = "\x1b[31;1merror:\x1b[0m model not found!")]
    ModelNotFound,
    #[fail(display = "\x1b[31;1merror:\x1b[0m could not open wallet seed!")]
    WalletSeedCouldNotBeOpened,
    #[fail(display = "Wallet already has a seed")]
    WalletHasSeed,
    #[fail(display = "Wallet doesnt have a seed")]
    WalletNoSeed,
    #[fail(display = "Wallet already connected")]
    WalletConnected,
    #[fail(display = "\x1b[31;1merror:\x1b[0m error opening wallet!")]
    OpenWalletError,
    #[fail(display = "\x1b[31;1merror:\x1b[0m error deriving keychain!")]
    DeriveKeychainError,
    #[fail(display = "\x1b[31;1merror:\x1b[0m wallet should be empty before attempting restore!")]
    WalletShouldBeEmpty,
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m transaction with slate id {} already received!",
        0
    )]
    TransactionAlreadyReceived(String),
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m transaction with slate id {} does not exist!",
        0
    )]
    TransactionDoesntExist(String),
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m transaction with slate id {} can not be cancelled!",
        0
    )]
    TransactionNotCancellable(String),
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m transaction cancellation error: {}",
        _0
    )]
    TransactionCancellationError(&'static str),
    #[fail(display = "\x1b[31;1merror:\x1b[0m transaction doesn't have a proof!")]
    TransactionHasNoProof,
    #[fail(display = "\x1b[31;1merror:\x1b[0m internal transaction error!")]
    LibTX(WalletErrorKind),
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m Not enough funds. Required: {}, Available: {}",
        needed_disp, available_disp
    )]
    NotEnoughFunds {
        available: u64,
        available_disp: String,
        needed: u64,
        needed_disp: String,
    },
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m Account label {} already exists!",
        0
    )]
    AccountLabelAlreadyExists(String),
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m invalid transaction id given: `{}`",
        0
    )]
    InvalidTxId(String),
    #[fail(display = "\x1b[31;1merror:\x1b[0m invalid amount given: `{}`", 0)]
    InvalidAmount(String),
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m invalid selection strategy, use either 'smallest' or 'all'"
    )]
    InvalidStrategy,
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m invalid number of minimum confirmations given: `{}`",
        0
    )]
    InvalidMinConfirmations(String),
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m invalid number of outputs given: `{}`",
        0
    )]
    InvalidNumOutputs(String),
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m invalid slate version given: `{}`",
        0
    )]
    InvalidSlateVersion(String),
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m could not unlock wallet! are you using the correct passphrase?"
    )]
    WalletUnlockFailed,
    #[fail(display = "\x1b[31;1merror:\x1b[0m could not open wallet! use `unlock` or `init`.")]
    NoWallet,
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m {} listener is closed! consider using `listen` first.",
        0
    )]
    ClosedListener(String),
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m listener for {} already started!",
        0
    )]
    AlreadyListening(String),
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m contact named `{}` already exists!",
        0
    )]
    ContactAlreadyExists(String),
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m could not find contact named `{}`!",
        0
    )]
    ContactNotFound(String),
    #[fail(display = "\x1b[31;1merror:\x1b[0m invalid character!")]
    InvalidBase58Character(char, usize),
    #[fail(display = "\x1b[31;1merror:\x1b[0m invalid length!")]
    InvalidBase58Length,
    #[fail(display = "\x1b[31;1merror:\x1b[0m invalid checksum!")]
    InvalidBase58Checksum,
    #[fail(display = "\x1b[31;1merror:\x1b[0m invalid network!")]
    InvalidBase58Version,
    #[fail(display = "\x1b[31;1merror:\x1b[0m invalid key!")]
    InvalidBase58Key,
    #[fail(display = "\x1b[31;1merror:\x1b[0m could not parse number from string!")]
    NumberParsingError,
    #[fail(display = "\x1b[31;1merror:\x1b[0m unknown address type `{}`!", 0)]
    UnknownAddressType(String),
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m could not parse `{}` to a grinbox address!",
        0
    )]
    GrinboxAddressParsingError(String),
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m could not parse `{}` to a keybase address!",
        0
    )]
    KeybaseAddressParsingError(String),
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m could not parse `{}` to a http address!",
        0
    )]
    HttpAddressParsingError(String),
    #[fail(display = "Unable to parse address")]
    ParseAddress,
    #[fail(display = "\x1b[31;1merror:\x1b[0m could not send keybase message!")]
    KeybaseMessageSendError,
    #[fail(display = "\x1b[31;1merror:\x1b[0m failed receiving slate!")]
    GrinWalletReceiveError,
    #[fail(display = "\x1b[31;1merror:\x1b[0m failed verifying slate messages!")]
    GrinWalletVerifySlateMessagesError,
    #[fail(display = "\x1b[31;1merror:\x1b[0m failed finalizing slate!")]
    GrinWalletFinalizeError,
    #[fail(display = "\x1b[31;1merror:\x1b[0m failed posting transaction!")]
    GrinWalletPostError,
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m keybase not found! consider installing keybase locally first."
    )]
    KeybaseNotFound,
    #[fail(display = "\x1b[31;1merror:\x1b[0m grinbox websocket terminated unexpectedly!")]
    GrinboxWebsocketAbnormalTermination,
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m rejecting invoice as auto invoice acceptance is turned off!"
    )]
    DoesNotAcceptInvoices,
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m rejecting invoice as amount '{}' is too big!",
        0
    )]
    InvoiceAmountTooBig(u64),
    #[fail(
        display = "\x1b[31;1merror:\x1b[0m please stop the listeners before doing this operation"
    )]
    HasListener,
    #[fail(display = "\x1b[31;1merror:\x1b[0m wallet already unlocked")]
    WalletAlreadyUnlocked,
    #[fail(display = "\x1b[31;1merror:\x1b[0m unable to encrypt message")]
    Encryption,
    #[fail(display = "\x1b[31;1merror:\x1b[0m unable to decrypt message")]
    Decryption,
    #[fail(display = "\x1b[31;1merror:\x1b[0m restore error")]
    Restore,
    #[fail(display = "\x1b[31;1merror:\x1b[0m unknown account: {}", 0)]
    UnknownAccountLabel(String),
    #[fail(display = "\x1b[31;1merror:\x1b[0m http request error")]
    HttpRequest,
    #[fail(display = "Node API error")]
    Node,
    #[fail(display = "{}", 0)]
    GenericError(String),
    #[fail(display = "\x1b[31;1merror:\x1b[0m unable to verify proof")]
    VerifyProof,
    #[fail(display = "\x1b[31;1merror:\x1b[0m file '{}' not found", 0)]
    FileNotFound(String),
    #[fail(display = "{}", 0)]
    Usage(String),
    #[fail(display = "Argument '{}' required", 0)]
    Argument(String),
    #[fail(display = "Unable to parse number '{}'", 0)]
    ParseNumber(String),
    #[fail(display = "Unable to parse slate")]
    ParseSlate,
    #[fail(display = "Incorrect listener interface")]
    IncorrectListenerInterface,
    #[fail(display = "No seed")]
	NoSeed,
    #[fail(display = "No backend")]
	NoBackend,
}
