use super::types::{
    AcctPathMapping, ChildNumber, ErrorKind, ExtKeychain, Identifier, Keychain, NodeClient, Result,
    WalletBackend,
};

/// Get next available key in the wallet for a given parent
pub fn next_available_key<T: ?Sized, C, K>(wallet: &mut T) -> Result<Identifier>
where
    T: WalletBackend<C, K>,
    C: NodeClient,
    K: Keychain,
{
    let child = wallet.derive_next()?;
    Ok(child)
}

/// Retrieve an existing key from a wallet
pub fn retrieve_existing_key<T: ?Sized, C, K>(
    wallet: &T,
    key_id: Identifier,
    mmr_index: Option<u64>,
) -> Result<(Identifier, u32)>
where
    T: WalletBackend<C, K>,
    C: NodeClient,
    K: Keychain,
{
    let existing = wallet.get_output(&key_id, &mmr_index)?;
    let key_id = existing.key_id.clone();
    let derivation = existing.n_child;
    Ok((key_id, derivation))
}

/// Returns a list of account to BIP32 path mappings
pub fn accounts<T: ?Sized, C, K>(wallet: &mut T) -> Result<Vec<AcctPathMapping>>
where
    T: WalletBackend<C, K>,
    C: NodeClient,
    K: Keychain,
{
    Ok(wallet.accounts().collect())
}

/// Adds an new parent account path with a given label
pub fn new_acct_path<T: ?Sized, C, K>(wallet: &mut T, label: &str) -> Result<Identifier>
where
    T: WalletBackend<C, K>,
    C: NodeClient,
    K: Keychain,
{
    let label = label.to_string();
    if let Some(_) = wallet.accounts().find(|l| l.label == label) {
        return Err(ErrorKind::AccountLabelAlreadyExists(label.clone()).into());
    }

    // We're always using paths at m/k/0 for parent keys for output derivations
    // so find the highest of those, then increment (to conform with external/internal
    // derivation chains in BIP32 spec)

    let highest_entry = wallet.accounts().max_by(|a, b| {
        <u32>::from(a.path.to_path().path[0]).cmp(&<u32>::from(b.path.to_path().path[0]))
    });

    let return_id = {
        if let Some(e) = highest_entry {
            let mut p = e.path.to_path();
            p.path[0] = ChildNumber::from(<u32>::from(p.path[0]) + 1);
            p.to_identifier()
        } else {
            ExtKeychain::derive_key_id(2, 0, 0, 0, 0)
        }
    };

    let save_path = AcctPathMapping {
        label: label.to_string(),
        path: return_id.clone(),
    };

    let mut batch = wallet.batch()?;
    batch.save_acct_path(&save_path)?;
    batch.commit()?;
    Ok(return_id)
}

/// Adds/sets a particular account path with a given label
pub fn set_acct_path<T: ?Sized, C, K>(wallet: &mut T, label: &str, path: &Identifier) -> Result<()>
where
    T: WalletBackend<C, K>,
    C: NodeClient,
    K: Keychain,
{
    let label = label.to_string();
    let save_path = AcctPathMapping {
        label,
        path: path.clone(),
    };

    let mut batch = wallet.batch()?;
    batch.save_acct_path(&save_path)?;
    batch.commit()?;
    Ok(())
}
