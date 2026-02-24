use trading_bot::strategy::whale::WhaleTracker;
use trading_bot::strategy::sniper::launch::{LaunchSniper, TokenLaunch};
use std::str::FromStr;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_rpc_client_api::config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_rpc_client_api::response::{Response, RpcLogsResponse};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding};
use futures_util::stream::StreamExt;
use std::pin::Pin;
use std::collections::HashSet;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use solana_sdk::signature::Keypair;
use anyhow::Result;
use tokio::sync::mpsc::Sender;
use chrono::Utc;

use trading_bot::strategy::event::SwapEvent;
use trading_bot::notifications::telegram::TelegramNotifier;

// Known DEX program IDs
const JUPITER: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
const RAYDIUM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const PUMPFUN: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

// Tokens you want to track (add more as needed)
const TRACKED_TOKENS: &[&str] = &[
    "So11111111111111111111111111111111111111112", // Wrapped SOL
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
    "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263", // BONK
    "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So", // mSOL
    "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn", // JitoSOL
    "bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1", // bSOL
];

// Helper to extract mints from token balances
fn extract_mints(tx: &EncodedConfirmedTransactionWithStatusMeta) -> HashSet<String> {
    let mut mints = HashSet::new();

    if let Some(meta) = &tx.transaction.meta {
        // Handle pre_token_balances (which is OptionSerializer)
        use solana_transaction_status::option_serializer::OptionSerializer;

        match &meta.pre_token_balances {
            OptionSerializer::Some(balances) => {
                for b in balances {
                    mints.insert(b.mint.clone());
                }
            }
            _ => {}
        }

        match &meta.post_token_balances {
            OptionSerializer::Some(balances) => {
                for b in balances {
                    mints.insert(b.mint.clone());
                }
            }
            _ => {}
        }
    }

    mints
}

// Check which tracked tokens are in the mints set
fn tracked_hits(mints: &HashSet<String>) -> Vec<String> {
    TRACKED_TOKENS
        .iter()
        .filter(|t| mints.contains(&t.to_string()))
        .map(|t| t.to_string())
        .collect()
}

// Helper to extract amount from logs (simplified - you'll need to enhance this)
fn extract_amount_from_logs(_logs: &[String]) -> Option<f64> {
    // TODO: Implement actual amount extraction from transaction logs
    // This is a placeholder that returns a random amount for testing
    Some(5.0) // Return small amount to avoid triggering whale detection accidentally
}

// Helper to extract wallet address from logs
fn extract_wallet_from_logs(_logs: &[String]) -> Option<Pubkey> {
    // TODO: Implement actual wallet extraction
    None
}

pub async fn listen_logs(
    ws_url: &str, 
    _wallet: Arc<Keypair>, 
    event_sender: Sender<SwapEvent>,
    telegram: Option<Arc<TelegramNotifier>>  // Add telegram parameter
) -> Result<()> {
    println!("📡 Connecting PubSub: {}", ws_url);

    let client = PubsubClient::new(ws_url).await?;
    println!("✅ PubSub client created");

    // Create whale tracker and launch sniper
    let whale_tracker = Arc::new(WhaleTracker::new());
    let launch_sniper = Arc::new(LaunchSniper::new());
    println!("✅ Whale tracker and launch sniper initialized");

    // Create HTTP RPC client for fetching transaction details
    let http_rpc = Arc::new(RpcClient::new("https://rpc.helius.xyz/?api-key=e84d2325-40ef-4e91-8a0a-bd4721ea4b26".to_string()));
    println!("✅ HTTP RPC client created");

    let config = RpcTransactionLogsConfig {
        commitment: Some(CommitmentConfig::processed()),
    };

    // Subscribe to ALL logs (Helius only supports 1 address in Mentions)
    let (stream, _subscription) = client
        .logs_subscribe(RpcTransactionLogsFilter::All, config)
        .await?;

    println!("✅ Subscribed to ALL logs - watching for swaps...");

    let mut stream = Pin::from(stream);

    while let Some(response) = stream.next().await {
        let response: Response<RpcLogsResponse> = response;

        let signature = &response.value.signature;
        let logs = &response.value.logs;

        // Track invoked programs by parsing "Program <ID> invoke"
        let mut invoked: HashSet<String> = HashSet::new();
        for line in logs {
            if let Some(rest) = line.strip_prefix("Program ") {
                if let Some((program_id, _)) = rest.split_once(" invoke") {
                    invoked.insert(program_id.to_string());
                }
            }
        }

        // Map invoked program IDs to human labels
        let mut swap_programs: HashSet<&'static str> = HashSet::new();
        if invoked.contains(JUPITER) { swap_programs.insert("Jupiter"); }
        if invoked.contains(RAYDIUM) { swap_programs.insert("Raydium"); }
        if invoked.contains(PUMPFUN) { swap_programs.insert("PumpFun"); }

        // Look for strong instruction markers
        let mut has_strong_swap_marker = false;
        for line in logs {
            if line.contains("Instruction: Swap")
                || line.contains("Instruction: SwapBaseIn")
                || line.contains("Instruction: SwapBaseOut")
                || line.contains("Instruction: Route") // Jupiter route
                || (line.contains("pump") && line.contains("swap"))
                || line.contains("Program log: Swap")
            {
                has_strong_swap_marker = true;
                break;
            }
        }

        // Only classify as swap if a DEX program was invoked AND we have strong markers
        let is_swap = !swap_programs.is_empty() && has_strong_swap_marker;

        // Only process successful swaps to reduce HTTP calls
        if is_swap && response.value.err.is_none() {
            println!("\n🔄 SWAP DETECTED! 🔄");
            println!("   Programs: {:?}", swap_programs);
            println!("   Signature: {}", signature);
            println!("   Logs: {} entries", logs.len());

            // Show first few logs for context
            for (i, line) in logs.iter().take(8).enumerate() {
                println!("   {}. {}", i + 1, line);
            }

            // Fetch full transaction to see token mints involved
            if let Ok(sig) = signature.parse::<Signature>() {
                let tx_cfg = RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::JsonParsed),
                    commitment: Some(CommitmentConfig::confirmed()),
                    max_supported_transaction_version: Some(0),
                };

                match http_rpc.get_transaction_with_config(&sig, tx_cfg).await {
                    Ok(tx) => {
                        let mints = extract_mints(&tx);
                        let hits = tracked_hits(&mints);

                        if !hits.is_empty() {
                            println!("   🪙 Tracked tokens: {:?}", hits);

                            // WHALE DETECTION
                            let amount_sol = extract_amount_from_logs(logs).unwrap_or(0.0);
                            
                            // Try to extract wallet address
                            if let Some(wallet_addr) = extract_wallet_from_logs(logs) {
                                if amount_sol > 10.0 {
                                    println!("   🐋 POTENTIAL WHALE TRANSACTION DETECTED ({} SOL)", amount_sol);

                                    if let Some(whale) = whale_tracker.process_transaction(
                                        &wallet_addr,
                                        amount_sol,
                                        &hits[0]
                                    ).await {
                                        println!("   🏆 SUCCESSFUL WHALE IDENTIFIED! Win rate >70%");

                                        // Send Telegram alert for whale detection
                                        if let Some(telegram) = &telegram {
                                            let msg = format!("🐋 Whale detected! Address: {}...", &whale.to_string()[..8]);
                                            let _ = telegram.notify_text(&msg).await;
                                        }
                                    }
                                }
                            }

                            // LAUNCH SNIPER DETECTION
                            // Check if this is a new token launch on PumpFun
                            if swap_programs.contains("PumpFun") && logs.iter().any(|l| l.contains("Initialize")) {
                                println!("   🚀 POTENTIAL NEW TOKEN LAUNCH DETECTED");

                                // Create token launch object (simplified)
                                let launch = TokenLaunch {
                                    mint: Pubkey::default(), // TODO: Extract actual mint
                                    dev_wallet: Pubkey::default(), // TODO: Extract dev wallet
                                    launch_time: Utc::now(),
                                    initial_liquidity_sol: 5.0, // TODO: Extract actual liquidity
                                    social_mentions: 0,
                                    verified: true,
                                    blacklisted: false,
                                };

                                if let Some(score) = launch_sniper.evaluate_launch(&launch).await {
                                    println!("   🎯 HIGH-POTENTIAL LAUNCH! Score: {:.1}", score);

                                    // Send Telegram alert for high-potential launch
                                    if let Some(telegram) = &telegram {
                                        let msg = format!("🚀 High-potential token launch! Score: {:.1}", score);
                                        let _ = telegram.notify_text(&msg).await;
                                    }
                                }
                            }

                            // Create and send swap event
                            let event = SwapEvent {
                                signature: sig,
                                programs: swap_programs.iter().map(|&s| s.to_string()).collect(),
                                mints: hits.clone().into_iter().collect(),
                            };

                            if let Err(e) = event_sender.send(event).await {
                                println!("   ⚠️ Failed to send event: {}", e);
                            } else {
                                println!("   📤 Event sent to strategy engine");
                            }
                        }
                    }
                    Err(e) => {
                        println!("   ⚠️ get_transaction failed: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}
