use std::env;
use std::time::Instant;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use dotenv::dotenv;
use solana_sdk::signature::Signer;
use tokio::sync::mpsc;

mod core;
use core::wallet::{load_keypair_from_env, test_signing};

mod ws_logs;
mod config;
use config::Config;

use trading_bot::strategy::event::SwapEvent;
use trading_bot::strategy::runner::StrategyRunner;
use trading_bot::execution::engine::ExecutionEngine;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    
    // Load and validate config
    let config = Config::from_env()?;
    config.validate()?;
    println!("✅ Dry run mode: {}", config.dry_run);

    println!("🚀 Solana Trading Bot - Phase 6");
    println!("============================================");

    // --- RPC Connection ---
    let rpc_https_url = env::var("RPC_HTTPS_URL")
        .map_err(|_| anyhow!("RPC_HTTPS_URL missing. Put it in .env"))?;

    // Create both blocking and nonblocking clients
    let rpc_blocking = Arc::new(solana_client::rpc_client::RpcClient::new(rpc_https_url.clone()));
    let rpc_nonblocking = Arc::new(solana_client::nonblocking::rpc_client::RpcClient::new(rpc_https_url.clone()));

    println!("✅ RPC Connected: {}", rpc_https_url);

    let version = rpc_blocking.get_version()?;
    println!("✅ RPC version: {:?}", version);

    let t0 = Instant::now();
    let slot = rpc_blocking.get_slot()?;
    let dt = t0.elapsed();
    println!("✅ Latest slot: {} (latency: {:?})", slot, dt);

    // --- Wallet ---
    println!("\n🔐 Loading wallet...");
    
    let kp = load_keypair_from_env()?;
    let wallet = Arc::new(kp);
    println!("✅ Wallet: {}", wallet.pubkey());
    let sig = test_signing(&wallet);
    println!("✅ Sign test: {}", &sig[..16]);

    // --- Create channel for events ---
    let (event_sender, event_receiver) = mpsc::channel::<SwapEvent>(100);
    println!("✅ Event channel created");

    // --- Create execution engine ---
    let execution_engine = Arc::new(ExecutionEngine::new(rpc_nonblocking.clone(), wallet.clone()));
    println!("✅ Execution engine created");

    // --- Create strategy runner ---
    let mut strategy_runner = StrategyRunner::new(execution_engine.clone(), wallet.clone());
    println!("✅ Strategy runner created");

    // --- Spawn strategy runner task ---
    tokio::spawn(async move {
        strategy_runner.run(event_receiver).await;
    });

    // --- WebSocket ---
    println!("\n📡 Phase 6: WebSocket Listener");
    
    let ws_url = env::var("RPC_WSS_URL")
        .unwrap_or_else(|_| "wss://api.mainnet-beta.solana.com".to_string());

    match ws_logs::listen_logs(&ws_url, wallet.clone(), event_sender).await {
        Ok(_) => println!("✅ WebSocket completed"),
        Err(e) => println!("❌ WebSocket error: {}", e),
    }

    println!("============================================");
    Ok(())
}
