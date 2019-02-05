use super::{Keychain, NodeClient, WalletBackend};

pub trait WalletInst<C, K>: WalletBackend<C, K> + Send + Sync + 'static
where
    C: NodeClient,
    K: Keychain,
{
}
impl<T, C, K> WalletInst<C, K> for T
where
    T: WalletBackend<C, K> + Send + Sync + 'static,
    C: NodeClient,
    K: Keychain,
{
}
