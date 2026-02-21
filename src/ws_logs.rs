use trading_bot::execution::engine::ExecutionEngine;
use anyhow::Result;
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

pub async fn listen_logs(ws_url: &str, _wallet: Arc<Keypair>) -> Result<()> {
    println!("📡 Connecting PubSub: {}", ws_url);

    let client = PubsubClient::new(ws_url).await?;
    println!("✅ PubSub client created");

    // Create HTTP RPC client for fetching transaction details
    let http_rpc = Arc::new(RpcClient::new("https://rpc.helius.xyz/?api-key=e84d2325-40ef-4e91-8a0a-bd4721ea4b26".to_string()));
    println!("✅ HTTP RPC client created");
    // Create execution engine
    let engine = Arc::new(ExecutionEngine::new(http_rpc.clone(), _wallet.clone()));
    println!("✅ Execution engine created");

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
                            
                            // Check if this is Wrapped SOL (our trigger token)
                            if hits.contains(&"So11111111111111111111111111111111111111112".to_string()) {
                                println!("   💰 Wrapped SOL detected - checking risk gates...");
                                
                                // Execute swap with risk gates
                                let input_mint = "So11111111111111111111111111111111111111112".parse().unwrap();
                                let output_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".parse().unwrap();
                                
                                match engine.execute_swap(
                                    input_mint,
                                    output_mint,
                                    0.01,  // 0.01 SOL
                                    100,   // 1% slippage
                                ).await {
                                    Ok(sig) => println!("   ✅ Swap executed: {}", sig),
                                    Err(e) => println!("   ⚠️ Swap rejected: {}", e),
                                }
                            } else {
                                println!("   🔍 Candidate identified - not in trigger list");
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
