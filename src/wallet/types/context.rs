use super::{Identifier, SecretKey};
use grin_core::libtx::aggsig;
use grin_core::ser;
use grin_util::secp;
use grin_util::secp::pedersen::Commitment;

#[derive(Serialize, Deserialize, Clone, Debug)]
/// Holds the context for a single aggsig transaction
pub struct Context {
    /// Parent key id
	pub parent_key_id: Identifier,
    /// Secret key (of which public is shared)
    pub sec_key: SecretKey,
    /// Secret nonce (of which public is shared)
    /// (basically a SecretKey)
    pub sec_nonce: SecretKey,
    /// store my outputs between invocations
    pub output_ids: Vec<(Identifier, Option<u64>, u64)>,
    /// store my inputs
    pub input_ids: Vec<(Identifier, Option<u64>, u64)>,
    /// keep track of the participant id
	pub participant_id: usize,
    /// store the transaction amount
    pub amount: u64,
    /// store the calculated fee
    pub fee: u64,
    /// Output commitments
    pub output_commits: Vec<Commitment>,
    /// Input commitments
    pub input_commits: Vec<Commitment>,
}

impl Context {
    /// Create a new context with defaults
    pub fn new(
        secp: &secp::Secp256k1,
        sec_key: SecretKey,
        parent_key_id: &Identifier,
        participant_id: usize,
    ) -> Context {
        Context {
            parent_key_id: parent_key_id.clone(),
            sec_key,
            sec_nonce: aggsig::create_secnonce(secp).unwrap(),
            output_ids: vec![],
            input_ids: vec![],
            participant_id,
            amount: 0,
            fee: 0,
            output_commits: vec![],
            input_commits: vec![],
        }
    }
}

impl Context {
    /// Tracks an output contributing to my excess value (if it needs to
    /// be kept between invocations
    pub fn add_output(&mut self, output_id: &Identifier, mmr_index: &Option<u64>, amount: u64) {
        self.output_ids.push((output_id.clone(), mmr_index.clone(), amount));
    }

    /// Returns all stored outputs
    pub fn get_outputs(&self) -> Vec<(Identifier, Option<u64>, u64)> {
        self.output_ids.clone()
    }

    /// Tracks IDs of my inputs into the transaction
    /// be kept between invocations
    pub fn add_input(&mut self, input_id: &Identifier, mmr_index: &Option<u64>, amount: u64) {
        self.input_ids.push((input_id.clone(), mmr_index.clone(), amount));
    }

    /// Returns all stored input identifiers
    pub fn get_inputs(&self) -> Vec<(Identifier, Option<u64>, u64)> {
        self.input_ids.clone()
    }
}

impl ser::Writeable for Context {
    fn write<W: ser::Writer>(&self, writer: &mut W) -> Result<(), ser::Error> {
        writer.write_bytes(&serde_json::to_vec(self).map_err(|_| ser::Error::CorruptedData)?)
    }
}

impl ser::Readable for Context {
    fn read(reader: &mut dyn ser::Reader) -> Result<Context, ser::Error> {
        let data = reader.read_bytes_len_prefix()?;
        serde_json::from_slice(&data[..]).map_err(|_| ser::Error::CorruptedData)
    }
}
