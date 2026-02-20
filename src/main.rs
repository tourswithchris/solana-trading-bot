use solana_sdk::signature::Signer;
use std::env;
use std::time::Instant;

use anyhow::{anyhow, Result};
use dotenv::dotenv;
use solana_client::rpc_client::RpcClient;

mod core;
use core::wallet::{load_keypair_from_env, test_signing};

mod ws_logs;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    println!("🚀 Solana Trading Bot - Phase 3");
    println!("============================================");

    // --- RPC Connection ---
    let rpc_https_url = env::var("RPC_HTTPS_URL")
        .map_err(|_| anyhow!("RPC_HTTPS_URL missing. Put it in .env"))?;

    let rpc = RpcClient::new(rpc_https_url.clone());

    println!("✅ RPC Connected: {}", rpc_https_url);

    let version = rpc.get_version()?;
    println!("✅ RPC version: {:?}", version);

    let t0 = Instant::now();
    let slot = rpc.get_slot()?;
    let dt = t0.elapsed();
    println!("✅ Latest slot: {} (latency: {:?})", slot, dt);

    // --- Wallet ---
    println!("\n🔐 Loading wallet...");
    
    match load_keypair_from_env() {
        Ok(kp) => {
            println!("✅ Wallet: {}", kp.pubkey());
            let sig = test_signing(&kp);
            println!("✅ Sign test: {}", &sig[..16]); // Show first 16 chars
        }
        Err(e) => {
            println!("⚠️ No wallet: {}", e);
        }
    }

    // --- WebSocket ---
    println!("\n📡 Phase 3: WebSocket Listener");
    
    let ws_url = env::var("RPC_WSS_URL")
        .unwrap_or_else(|_| "wss://api.mainnet-beta.solana.com".to_string());

    match ws_logs::listen_logs(&ws_url).await {
        Ok(_) => println!("✅ WebSocket completed"),
        Err(e) => println!("❌ WebSocket error: {}", e),
    }

    println!("============================================");
    Ok(())
}
