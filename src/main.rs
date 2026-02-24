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
use trading_bot::notifications::telegram::TelegramNotifier;
use trading_bot::db::trades::TradeDatabase;
use trading_bot::web::server::start_web_server;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    // Load and validate config
    let config = Config::from_env()?;
    config.validate()?;
    println!("✅ Dry run mode: {}", config.dry_run);

    println!("🚀 Solana Trading Bot - Phase 6");
    println!("============================================");

    // --- Initialize Telegram (optional) ---
    let telegram = if let (Ok(token), Ok(chat_id)) = (
        env::var("TELEGRAM_BOT_TOKEN"),
        env::var("TELEGRAM_CHAT_ID")
    ) {
        let notifier = Arc::new(TelegramNotifier::new(token, chat_id));
        // Send startup notification
        let _ = notifier.send_message("🚀 Trading bot started").await;
        println!("✅ Telegram notifier initialized");
        Some(notifier)
    } else {
        println!("⚠️ Telegram not configured - continuing without notifications");
        None
    };

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

    // --- Initialize database ---
    let db_path = "./trading_bot.db";
    let db = Arc::new(TradeDatabase::new(db_path)?);
    println!("✅ Database initialized at {}", db_path);

    // --- Spawn daily summary task ---
    let tg_daily = telegram.clone();
    let db_daily = db.clone();
    
    tokio::spawn(async move {
        loop {
            // Run every 24 hours (86400 seconds)
            tokio::time::sleep(std::time::Duration::from_secs(24 * 60 * 60)).await;
            
            if let (Some(tg), Ok(stats)) = (tg_daily.as_ref(), db_daily.get_stats().await) {
                let msg = format!(
                    "📊 Daily Summary\n\
                     Trades: {}\n\
                     Win rate: {:.1}%\n\
                     Net PnL (SOL): {:.6}\n\
                     Fees (SOL): {:.6}\n\
                     Best: {:.6} | Worst: {:.6}",
                    stats["total_trades"].as_i64().unwrap_or(0),
                    stats["win_rate"].as_f64().unwrap_or(0.0),
                    stats["net_pnl_sol"].as_f64().unwrap_or(0.0),
                    stats["total_fees_sol"].as_f64().unwrap_or(0.0),
                    stats["best_trade_sol"].as_f64().unwrap_or(0.0),
                    stats["worst_trade_sol"].as_f64().unwrap_or(0.0),
                );
                let _ = tg.notify_text(&msg).await;
            }
        }
    });

    // --- Create channel for events ---
    let (event_sender, event_receiver) = mpsc::channel::<SwapEvent>(100);
    println!("✅ Event channel created");

    // --- Create execution engine ---
    let execution_engine = Arc::new(ExecutionEngine::new(rpc_nonblocking.clone(), wallet.clone()));
    println!("✅ Execution engine created");

    // --- Create strategy runner with Telegram and database ---
    let mut strategy_runner = StrategyRunner::new(
        execution_engine.clone(),
        wallet.clone(),
        telegram.clone(),
        Some(db.clone()),
    );
    println!("✅ Strategy runner created");

    // --- Spawn strategy runner task ---
    tokio::spawn(async move {
        strategy_runner.run(event_receiver).await;
    });

    // --- Spawn web server in background ---
    let web_db = db.clone();
    tokio::spawn(async move {
        if let Err(e) = start_web_server(web_db, 3000).await {
            eprintln!("❌ Web server error: {}", e);
        }
    });

    // --- WebSocket with auto-reconnect ---
    println!("\n📡 Phase 6: WebSocket Listener (auto-reconnect)");
    
    loop {
        let ws_url = env::var("RPC_WSS_URL")
            .unwrap_or_else(|_| "wss://api.mainnet-beta.solana.com".to_string());

        match ws_logs::listen_logs(&ws_url, wallet.clone(), event_sender.clone(), telegram.clone()).await {
            Ok(_) => println!("✅ WebSocket completed normally, reconnecting in 5s..."),
            Err(e) => {
                println!("❌ WebSocket error: {}, reconnecting in 5s...", e);
                if let Some(telegram) = &telegram {
                    let _ = telegram.notify_error(&format!("WebSocket error: {}, reconnecting...", e)).await;
                }
            }
        }
        
        // Wait before reconnecting
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

