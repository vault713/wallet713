// Copyright 2018 The Grin Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Error types for libwallet

use grin_core::core::{committed, transaction};
use grin_core::libtx;
use grin_keychain;
use grin_util::secp;
use failure::Fail;

/// Wallet errors, mostly wrappers around underlying crypto or I/O errors.
#[derive(Clone, Eq, PartialEq, Debug, Fail, Serialize, Deserialize)]
pub enum ErrorKind {
	/// Not enough funds
	#[fail(
		display = "Not enough funds. Required: {}, Available: {}",
		needed_disp, available_disp
	)]
	NotEnoughFunds {
		/// available funds
		available: u64,
		/// Display friendly
		available_disp: String,
		/// Needed funds
		needed: u64,
		/// Display friendly
		needed_disp: String,
	},

	/// Fee error
	#[fail(display = "Fee Error: {}", _0)]
	Fee(String),

	/// LibTX Error
	#[fail(display = "LibTx Error")]
	LibTX(libtx::ErrorKind),

	/// Keychain error
	#[fail(display = "Keychain error")]
	Keychain(grin_keychain::Error),

	/// Transaction Error
	#[fail(display = "Transaction error")]
	Transaction(transaction::Error),

	/// API Error
	#[fail(display = "Client Callback Error: {}", _0)]
	ClientCallback(String),

	/// Secp Error
	#[fail(display = "Secp error")]
	Secp(secp::Error),

	/// Callback implementation error conversion
	#[fail(display = "Trait Implementation error")]
	CallbackImpl(&'static str),

	/// Wallet backend error
	#[fail(display = "Wallet store error: {}", _0)]
	Backend(String),

	/// BIP 39 word list
	#[fail(display = "BIP39 Mnemonic (word list) Error")]
	Mnemonic,

	/// Enc/Decryption Error
	#[fail(display = "Enc/Decryption error (check password?)")]
	Encryption,

	/// Callback implementation error conversion
	#[fail(display = "Restore Error")]
	Restore,

	/// An error in the format of the JSON structures exchanged by the wallet
	#[fail(display = "JSON format error")]
	Format,

	/// Other serialization errors
	#[fail(display = "Ser/Deserialization error")]
	Deser(grin_core::ser::Error),

	/// IO Error
	#[fail(display = "I/O error")]
	IO,

	/// Error when contacting a node through its API
	#[fail(display = "Node API error")]
	Node,

	/// Error contacting wallet API
	#[fail(display = "Wallet Communication Error: {}", _0)]
	WalletComms(String),

	/// Error originating from hyper.
	#[fail(display = "Hyper error")]
	Hyper,

	/// Error originating from hyper uri parsing.
	#[fail(display = "Uri parsing error")]
	Uri,

	/// Signature error
	#[fail(display = "Signature error: {}", _0)]
	Signature(String),

	/// Attempt to use duplicate transaction id in separate transactions
	#[fail(display = "Duplicate transaction ID error")]
	DuplicateTransactionId,

	/// Wallet seed already exists
	#[fail(display = "Wallet seed exists error")]
	WalletSeedExists,

	/// Wallet seed doesn't exist
	#[fail(display = "Wallet seed doesn't exist error")]
	WalletSeedDoesntExist,

	/// Wallet seed doesn't exist
	#[fail(display = "Wallet seed decryption error")]
	WalletSeedDecryption,

	/// Transaction doesn't exist
	#[fail(display = "Transaction {} doesn't exist", _0)]
	TransactionDoesntExist(String),

	/// Transaction already rolled back
	#[fail(display = "Transaction {} cannot be cancelled", _0)]
	TransactionNotCancellable(String),

	/// Cancellation error
	#[fail(display = "Cancellation Error: {}", _0)]
	TransactionCancellationError(&'static str),

	/// Cancellation error
	#[fail(display = "Tx dump Error: {}", _0)]
	TransactionDumpError(&'static str),

	/// Attempt to repost a transaction that's already confirmed
	#[fail(display = "Transaction already confirmed")]
	TransactionAlreadyConfirmed,

	/// Transaction has already been received
	#[fail(display = "Transaction {} has already been received", _0)]
	TransactionAlreadyReceived(String),

	/// Attempt to repost a transaction that's not completed and stored
	#[fail(display = "Transaction building not completed: {}", _0)]
	TransactionBuildingNotCompleted(u32),

	/// Invalid BIP-32 Depth
	#[fail(display = "Invalid BIP32 Depth (must be 1 or greater)")]
	InvalidBIP32Depth,

	/// Attempt to add an account that exists
	#[fail(display = "Account Label '{}' already exists", _0)]
	AccountLabelAlreadyExists(String),

	/// Reference unknown account label
	#[fail(display = "Unknown Account Label '{}'", _0)]
	UnknownAccountLabel(String),

	/// Error from summing commitments via committed trait.
	#[fail(display = "Committed Error")]
	Committed(committed::Error),

	/// Can't parse slate version
	#[fail(display = "Can't parse slate version")]
	SlateVersionParse,

	/// Can't deserialize slate
	#[fail(display = "Can't Deserialize slate")]
	SlateDeser,

	/// Unknown slate version
	#[fail(display = "Unknown Slate Version: {}", _0)]
	SlateVersion(u16),

	/// No seed
	#[fail(display = "No seed")]
	NoSeed,

	/// No backend opened
	#[fail(display = "No backend opened")]
	NoBackend,

	/// No address book found
	#[fail(display = "No address book found")]
	NoAddressBook,

	/// Contact not found
	#[fail(display = "Contact '{}' not found", 0)]
	ContactNotFound(String),

	#[fail(display = "Already listening on {}", 0)]
	AlreadyListening(String),

	#[fail(display = "No listener on {}", 0)]
	NoListener(String),

	#[fail(display = "Invalid listener interface")]
	InvalidListenerInterface,

	/// No transaction stored
	#[fail(display = "No transaction stored")]
	TransactionNotStored,

	/// No transaction proof stored
	#[fail(display = "No transaction proof stored")]
	TransactionProofNotStored,

	#[fail(display = "Incoming slate is not compatible with this wallet. Please upgrade the node or use a different one")]
	Compatibility,

	#[fail(display = "Unable to verify proof")]
	VerifyProof,

	/// Other
	#[fail(display = "Generic error: {}", _0)]
	GenericError(String),
}