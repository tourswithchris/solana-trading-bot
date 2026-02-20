use anyhow::{anyhow, Result};
use solana_sdk::signature::{Keypair, Signer};
use std::env;

pub fn load_keypair_from_env() -> Result<Keypair> {
    let raw = env::var("PRIVATE_KEY")
        .map_err(|_| anyhow!("PRIVATE_KEY missing in .env"))?;

    // Expect JSON array like: [12,34,...]
    let bytes: Vec<u8> = serde_json::from_str(&raw)
        .map_err(|e| anyhow!("PRIVATE_KEY must be a JSON array of bytes: {e}"))?;

    let kp = Keypair::from_bytes(&bytes)
        .map_err(|e| anyhow!("Invalid PRIVATE_KEY bytes: {e}"))?;

    Ok(kp)
}

pub fn test_signing(kp: &Keypair) -> String {
    let msg = b"wallet-sign-test";
    let sig = kp.sign_message(msg);
    sig.to_string()
}
