use grin_util::secp::Signature;

use crate::common::crypto::EncryptedMessage;
use crate::contacts::GrinboxAddress;

#[derive(Debug, Serialize, Deserialize)]
pub struct TxProof {
    pub message: String,
    pub challenge: String,
    pub signature: Signature,
    pub key: [u8; 32],
}

impl TxProof {
    pub fn verify(&self, address: GrinboxAddress) {
        // TODO
    }
}