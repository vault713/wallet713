pub use failure::Error;

#[derive(Debug, Fail)]
pub enum Wallet713Error {
    #[fail(display = "invalid transaction id given: `{}`", 0)]
    InvalidTxId(String),
    #[fail(display = "invalid amount given: `{}`", 0)]
    InvalidAmount(String),
    #[fail(display = "could not find a wallet! consider using `init`.")]
    NoWallet,
    #[fail(display = "{} listener is closed! consider using `listen` first.", 0)]
    ClosedListener(String),
    #[fail(display = "listener for {} already started!", 0)]
    AlreadyListening(String),
    #[fail(display = "`{}` is not a valid grinbox address. To send to your contacts add `@` before the name.", 0)]
    InvalidGrinboxAddress(String),
    #[fail(display = "`{}` is not a valid public key. To send to your contacts add `@` before the name.", 0)]
    InvalidPublicKey(String),
    #[fail(display = "`{}` is not a valid public key.", 0)]
    InvalidContactPublicKey(String),
    #[fail(display = "contact named `{}` already exists!", 0)]
    ContactAlreadyExists(String),
    #[fail(display = "could not find contact named `{}`!", 0)]
    ContactNotFound(String),
    #[fail(display = "wallet713 config not found! Use `config` command to set one up.")]
    ConfigNotFound,
    #[fail(display = "could not load config!")]
    LoadConfig,
    #[fail(display = "no keys found in configuration. consider running `config --generate-keys` to generate new keys.")]
    ConfigMissingKeys,
    #[fail(display = "missing configuration value for `{}`!", 0)]
    ConfigMissingValue(String),
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
    #[fail(display = "address `{}` is missing a type!", 0)]
    MissingAddressType(String),
    #[fail(display = "could not parse `{}` to an address!", 0)]
    AddressParsingError(String),
    #[fail(display = "could not parse `{}` to a grinbox address!", 0)]
    GrinboxAddressParsingError(String),
    #[fail(display = "could not parse `{}` to a keybase address!", 0)]
    KeybaseAddressParsingError(String),
    #[fail(display = "could not send keybase message!")]
    KeybaseMessageSendError,
    #[fail(display = "failed receiving slate!")]
    GrinWalletReceiveError,
    #[fail(display = "failed finalizing slate!")]
    GrinWalletFinalizeError,
    #[fail(display = "failed posting transaction!")]
    GrinWalletPostError,
    #[fail(display = "keybase not found! consider installing keybase locally first.")]
    KeybaseNotFound,
    #[fail(display = "address can not be normalized!")]
    AddressCannotBeNormalized
}
