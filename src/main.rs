use std::env;
use std::time::Instant;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use dotenv::dotenv;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Signer;
use solana_sdk::pubkey::Pubkey;

mod core;
use core::wallet::{load_keypair_from_env, test_signing};

mod ws_logs;
mod config;
use config::Config;

mod engine;
use trading_bot::engine::swap::{execute_swap};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    
    // Load and validate config
    let config = Config::from_env()?;
    config.validate()?;
    println!("✅ Dry run mode: {}", config.dry_run);

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
    
    let kp = load_keypair_from_env()?;
    let wallet = Arc::new(kp);
    println!("✅ Wallet: {}", wallet.pubkey());
    let sig = test_signing(&wallet);
    println!("✅ Sign test: {}", &sig[..16]);

    // --- WebSocket ---
    println!("\n📡 Phase 3: WebSocket Listener");
    
    let ws_url = env::var("RPC_WSS_URL")
        .unwrap_or_else(|_| "wss://api.mainnet-beta.solana.com".to_string());

    match ws_logs::listen_logs(&ws_url, wallet.clone()).await {
        Ok(_) => println!("✅ WebSocket completed"),
        Err(e) => println!("❌ WebSocket error: {}", e),
    }

    // --- Phase 4: Test Swap Simulation ---
    println!("\n🔄 Phase 4: Testing swap simulation");
    
    // Test the execute_swap function directly
    match execute_swap(
        "raydium",
        0.1,
        None,
        Some(Pubkey::default()),
    ).await {
        Ok(sigs) => println!("✅ Raydium swap simulation successful: {:?}", sigs),
        Err(e) => println!("❌ Raydium swap failed: {}", e),
    }
    
    match execute_swap(
        "pumpfun",
        0.05,
        Some(Pubkey::default()),
        None,
    ).await {
        Ok(sigs) => println!("✅ PumpFun swap simulation successful: {:?}", sigs),
        Err(e) => println!("❌ PumpFun swap failed: {}", e),
    }

    println!("============================================");
    Ok(())
}
