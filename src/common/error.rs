pub use failure::Error;

#[derive(Debug, Fail)]
pub enum Wallet713Error {
    #[fail(display = "invalid transaction id given: `{}`", 0)]
    InvalidTxId(String),
    #[fail(display = "invalid amount given: `{}`", 0)]
    InvalidAmount(String),
    #[fail(display = "could not find a wallet! consider using `init`.")]
    NoWallet,
    #[fail(display = "listener is closed! consider using `listen` first.")]
    ClosedListener,
    #[fail(display = "already listening on [{}]!", 0)]
    AlreadyListening(String),
    #[fail(display = "could not subscribe!")]
    Subscribe,
    #[fail(display = "could not unsubscribe!")]
    Unsubscribe,
    #[fail(display = "could not post slate!")]
    PostSlate,
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
}
