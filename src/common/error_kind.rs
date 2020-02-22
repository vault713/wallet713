// Copyright 2019 The vault713 Developers
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

use failure::Fail;

#[derive(Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
	#[fail(display = "Secp error")]
	Secp,
	#[fail(display = "Wallet already has a seed")]
	WalletHasSeed,
	#[fail(display = "Wallet doesnt have a seed")]
	WalletNoSeed,
	#[fail(display = "Wallet already connected")]
	WalletConnected,
	#[fail(display = "Unable to open wallet")]
	OpenWalletError,
	#[fail(display = "Unable derive keychain")]
	DeriveKeychainError,
	#[fail(display = "Wallet should be empty before attempting restore")]
	WalletShouldBeEmpty,
	#[fail(display = "Transaction doesn't have a proof")]
	TransactionHasNoProof,
	#[fail(display = "Unable to open wallet")]
	NoWallet,
	#[fail(display = "Listener for {} closed", 0)]
	ClosedListener(String),
	#[fail(display = "Contact '{}' already exists!", 0)]
	ContactAlreadyExists(String),
	#[fail(display = "Invalid base58 character!")]
	InvalidBase58Character(char, usize),
	#[fail(display = "Invalid base58 length")]
	InvalidBase58Length,
	#[fail(display = "Invalid base58 checksum")]
	InvalidBase58Checksum,
	#[fail(display = "Invalid base58 version bytes")]
	InvalidBase58Version,
	#[fail(display = "Invalid key")]
	InvalidBase58Key,
	#[fail(display = "Could not parse number from string")]
	NumberParsingError,
	#[fail(display = "Unknown address type '{}'", 0)]
	UnknownAddressType(String),
	#[fail(display = "Could not parse '{}' to a grinbox address", 0)]
	GrinboxAddressParsingError(String),
	#[fail(display = "Could not parse '{}' to a keybase address", 0)]
	KeybaseAddressParsingError(String),
	#[fail(display = "Could not parse `{}` to a http address", 0)]
	HttpAddressParsingError(String),
	#[fail(display = "Unable to parse address")]
	ParseAddress,
	#[fail(display = "Could not send keybase message")]
	KeybaseMessageSendError,
	#[fail(display = "Keybase not found! Consider installing it first")]
	KeybaseNotFound,
	#[fail(display = "Grinbox websocket terminated unexpectedly")]
	GrinboxWebsocketAbnormalTermination,
	#[fail(display = "Unable to encrypt message")]
	Encryption,
	#[fail(display = "Unable to decrypt message")]
	Decryption,
	#[fail(display = "Restore error")]
	Restore,
	#[fail(display = "Unknown account '{}'", 0)]
	UnknownAccountLabel(String),
	#[fail(display = "{}", 0)]
	GenericError(String),
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
}
