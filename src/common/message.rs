use grin_util::secp::key::{PublicKey, SecretKey};
use grin_util::secp::Secp256k1;
use rand::thread_rng;
use rand::Rng;
use ring::aead;
use ring::{digest, pbkdf2};

use crate::common::crypto::{from_hex, to_hex};
use crate::common::{ErrorKind, Result};
use crate::contacts::GrinboxAddress;

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedMessage {
	pub destination: GrinboxAddress,
	encrypted_message: String,
	salt: String,
	nonce: String,
}

impl EncryptedMessage {
	pub fn new(
		message: String,
		destination: &GrinboxAddress,
		receiver_public_key: &PublicKey,
		secret_key: &SecretKey,
	) -> Result<EncryptedMessage> {
		let secp = Secp256k1::new();
		let mut common_secret = receiver_public_key.clone();
		common_secret
			.mul_assign(&secp, secret_key)
			.map_err(|_| ErrorKind::Encryption)?;
		let common_secret_ser = common_secret.serialize_vec(&secp, true);
		let common_secret_slice = &common_secret_ser[1..33];

		let salt: [u8; 8] = thread_rng().gen();
		let nonce: [u8; 12] = thread_rng().gen();
		let mut key = [0; 32];
		pbkdf2::derive(&digest::SHA512, 100, &salt, common_secret_slice, &mut key);
		let mut enc_bytes = message.as_bytes().to_vec();
		let suffix_len = aead::CHACHA20_POLY1305.tag_len();
		for _ in 0..suffix_len {
			enc_bytes.push(0);
		}
		let sealing_key = aead::SealingKey::new(&aead::CHACHA20_POLY1305, &key)
			.map_err(|_| ErrorKind::Encryption)?;
		aead::seal_in_place(&sealing_key, &nonce, &[], &mut enc_bytes, suffix_len)
			.map_err(|_| ErrorKind::Encryption)?;

		Ok(EncryptedMessage {
			destination: destination.clone(),
			encrypted_message: to_hex(enc_bytes),
			salt: to_hex(salt.to_vec()),
			nonce: to_hex(nonce.to_vec()),
		})
	}

	pub fn key(&self, sender_public_key: &PublicKey, secret_key: &SecretKey) -> Result<[u8; 32]> {
		let salt = from_hex(self.salt.clone()).map_err(|_| ErrorKind::Decryption)?;

		let secp = Secp256k1::new();
		let mut common_secret = sender_public_key.clone();
		common_secret
			.mul_assign(&secp, secret_key)
			.map_err(|_| ErrorKind::Decryption)?;
		let common_secret_ser = common_secret.serialize_vec(&secp, true);
		let common_secret_slice = &common_secret_ser[1..33];

		let mut key = [0; 32];
		pbkdf2::derive(&digest::SHA512, 100, &salt, common_secret_slice, &mut key);

		Ok(key)
	}

	pub fn decrypt_with_key(&self, key: &[u8; 32]) -> Result<String> {
		let mut encrypted_message =
			from_hex(self.encrypted_message.clone()).map_err(|_| ErrorKind::Decryption)?;
		let nonce = from_hex(self.nonce.clone()).map_err(|_| ErrorKind::Decryption)?;

		let opening_key = aead::OpeningKey::new(&aead::CHACHA20_POLY1305, key)
			.map_err(|_| ErrorKind::Decryption)?;
		let decrypted_data =
			aead::open_in_place(&opening_key, &nonce, &[], 0, &mut encrypted_message)
				.map_err(|_| ErrorKind::Decryption)?;

		String::from_utf8(decrypted_data.to_vec()).map_err(|_| ErrorKind::Decryption.into())
	}
}
