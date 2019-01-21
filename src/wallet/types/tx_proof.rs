use grin_core::libtx::slate::Slate;
use grin_util::secp::Signature;
use grin_util::secp::key::SecretKey;

use crate::common::crypto::{EncryptedMessage, Hex};
use crate::common::crypto::verify_signature;
use crate::contacts::{Address, GrinboxAddress};

pub enum ErrorKind {
    ParseAddress,
    ParsePublicKey,
    ParseSignature,
    VerifySignature,
    ParseEncryptedMessage,
    DecryptionKey,
    DecryptMessage,
    ParseSlate,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TxProof {
    pub address: GrinboxAddress,
    pub message: String,
    pub challenge: String,
    pub signature: Signature,
    pub key: [u8; 32],
}

impl TxProof {
    pub fn verify_extract(&self) -> Result<Slate, ErrorKind> {
        let mut challenge = String::new();
        challenge.push_str(self.message.as_str());
        challenge.push_str(self.challenge.as_str());

        let public_key = self.address.public_key()
            .map_err(|_| ErrorKind::ParsePublicKey)?;

        verify_signature(&challenge, &self.signature, &public_key)
            .map_err(|_| ErrorKind::VerifySignature)?;

        let encrypted_message: EncryptedMessage = serde_json::from_str(&self.message)
            .map_err(|_| ErrorKind::ParseEncryptedMessage)?;

        let decrypted_message = encrypted_message.decrypt_with_key(&self.key)
            .map_err(|_| ErrorKind::DecryptMessage)?;

        serde_json::from_str(&decrypted_message)
            .map_err(|_| ErrorKind::ParseSlate)
    }

    pub fn from_response(from: String, message: String, signature: String, challenge: String, secret_key: &SecretKey) -> Result<(Slate, TxProof), ErrorKind> {
        let address = GrinboxAddress::from_str(from.as_str())
            .map_err(|_| ErrorKind::ParseAddress)?;
        let signature = Signature::from_hex(signature.as_str())
            .map_err(|_| ErrorKind::ParseSignature)?;
        let public_key = address.public_key()
            .map_err(|_| ErrorKind::ParsePublicKey)?;
        let encrypted_message: EncryptedMessage = serde_json::from_str(&message)
            .map_err(|_| ErrorKind::ParseEncryptedMessage)?;
        let key = encrypted_message.key(&public_key, secret_key)
                .map_err(|_| ErrorKind::DecryptionKey)?;

        let proof = TxProof {
            address,
            message,
            challenge,
            signature,
            key
        };

        let slate = proof.verify_extract()?;

        Ok((slate, proof))
    }
}