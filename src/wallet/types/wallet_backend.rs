use super::{
    AcctPathMapping, Context, Identifier, Keychain, NodeClient, OutputData, Result, Transaction,
    TxLogEntry, TxProof, WalletBackendBatch,
};

pub trait WalletBackend<C, K>
where
    C: NodeClient,
    K: Keychain,
{
    fn open_with_credentials(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
    fn restore(&mut self) -> Result<()>;
    fn check_repair(&mut self) -> Result<()>;
    fn get_parent_key_id(&self) -> Identifier;
    fn set_parent_key_id(&mut self, id: &Identifier);
    fn set_parent_key_id_by_name(&mut self, label: &str) -> Result<()>;
    fn w2n_client(&mut self) -> &mut C;
    fn calc_commit_for_cache(&mut self, amount: u64, id: &Identifier) -> Result<Option<String>>;
    fn keychain(&mut self) -> &mut K;
    fn derive_next(&mut self) -> Result<Identifier>;
    fn get_output(&self, id: &Identifier, mmr_index: &Option<u64>) -> Result<OutputData>;
    fn get_private_context(&mut self, uuid: &str) -> Result<Context>;
    fn get_acct_path(&self, label: &str) -> Result<AcctPathMapping>;
    fn get_last_confirmed_height(&self) -> Result<u64>;
    fn get_stored_tx(&self, uuid: &str) -> Result<Transaction>;
    fn has_stored_tx_proof(&self, uuid: &str) -> Result<bool>;
    fn get_stored_tx_proof(&self, uuid: &str) -> Result<TxProof>;
    fn get_tx_log_by_slate_id(&self, slate_id: &str) -> Result<Option<TxLogEntry>>;
    fn outputs<'a>(&'a self) -> Box<dyn Iterator<Item = OutputData> + 'a>;
    fn tx_logs<'a>(&'a self) -> Box<dyn Iterator<Item = TxLogEntry> + 'a>;
    fn accounts<'a>(&'a self) -> Box<dyn Iterator<Item = AcctPathMapping> + 'a>;
    fn batch<'a>(&'a self) -> Result<Box<dyn WalletBackendBatch<K> + 'a>>;
}
