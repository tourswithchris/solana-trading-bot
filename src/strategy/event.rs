use solana_sdk::signature::Signature;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct SwapEvent {
    pub signature: Signature,
    pub programs: HashSet<String>,
    pub mints: HashSet<String>,
}
