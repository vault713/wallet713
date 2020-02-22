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

use crate::common::Result;
use digest::generic_array::GenericArray;
use grin_core::global::is_floonet;
use grin_keychain::extkey_bip32::{BIP32Hasher, ChildNumber, ExtendedPrivKey};
use grin_keychain::{Keychain, SwitchCommitmentType};
use grin_util::secp::key::SecretKey;
use hmac::{Hmac, Mac};
use ripemd160::Ripemd160;
use sha2::{Digest, Sha256, Sha512};

type HmacSha512 = Hmac<Sha512>;

#[derive(Clone, Debug)]
pub struct BIP32GrinboxHasher {
	is_floo: bool,
	hmac_sha512: HmacSha512,
}

impl BIP32GrinboxHasher {
	/// New empty hasher
	pub fn new(is_floo: bool) -> Self {
		Self {
			is_floo,
			hmac_sha512: HmacSha512::new(GenericArray::from_slice(&[0u8; 128])),
		}
	}
}

impl BIP32Hasher for BIP32GrinboxHasher {
	fn network_priv(&self) -> [u8; 4] {
		match self.is_floo {
			true => [42, 0, 0, 42],
			false => [42, 1, 0, 42],
		}
	}
	fn network_pub(&self) -> [u8; 4] {
		match self.is_floo {
			true => [42, 0, 1, 42],
			false => [42, 1, 1, 42],
		}
	}
	fn master_seed() -> [u8; 12] {
		b"Grinbox_seed".to_owned()
	}
	fn init_sha512(&mut self, seed: &[u8]) {
		self.hmac_sha512 = HmacSha512::new_varkey(seed).expect("HMAC can take key of any size");
	}
	fn append_sha512(&mut self, value: &[u8]) {
		self.hmac_sha512.input(value);
	}
	fn result_sha512(&mut self) -> [u8; 64] {
		let mut result = [0; 64];
		result.copy_from_slice(self.hmac_sha512.result().code().as_slice());
		result
	}
	fn sha_256(&self, input: &[u8]) -> [u8; 32] {
		let mut sha2_res = [0; 32];
		let mut sha2 = Sha256::new();
		sha2.input(input);
		sha2_res.copy_from_slice(sha2.result().as_slice());
		sha2_res
	}
	fn ripemd_160(&self, input: &[u8]) -> [u8; 20] {
		let mut ripemd_res = [0; 20];
		let mut ripemd = Ripemd160::new();
		ripemd.input(input);
		ripemd_res.copy_from_slice(ripemd.result().as_slice());
		ripemd_res
	}
}

pub fn derive_address_key<K: Keychain>(keychain: &K, index: u32) -> Result<SecretKey> {
	let root = keychain.derive_key(713, &K::root_key_id(), &SwitchCommitmentType::Regular)?;
	let mut hasher = BIP32GrinboxHasher::new(is_floonet());
	let secp = keychain.secp();
	let master = ExtendedPrivKey::new_master(secp, &mut hasher, &root.0)?;
	Ok(master
		.ckd_priv(secp, &mut hasher, ChildNumber::from_normal_idx(index))?
		.secret_key)
}
