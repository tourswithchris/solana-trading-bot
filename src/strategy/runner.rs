use anyhow::Result;
use tokio::sync::mpsc::Receiver;
use std::sync::Arc;
use solana_sdk::signature::Keypair;
use solana_sdk::pubkey::Pubkey;
use std::time::{Duration, Instant};
use chrono::Utc;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_transaction_status::UiTransactionEncoding;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signer;

use crate::execution::engine::ExecutionEngine;
use crate::execution::jupiter::JupiterExecutor;
use crate::strategy::event::SwapEvent;
use crate::strategy::state::{StateMachine, BotState};
use crate::strategy::candidate::{TradeCandidate, RiskLimits};
use crate::strategy::pnl::{PnLTracker, TradeRecord};
use crate::strategy::tx_parse::{extract_fee_sol, extract_owner_token_deltas};
use crate::notifications::telegram::TelegramNotifier;
use crate::db::trades::{TradeDatabase, TradeRecord as DbTradeRecord};

// Token allowlist (only trade these)
const ALLOWED_TOKENS: &[&str] = &[
    "So11111111111111111111111111111111111111112", // Wrapped SOL
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
    "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263", // BONK
    "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So", // mSOL
    "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn", // JitoSOL
    "bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1", // bSOL
];

pub struct StrategyRunner {
    pub state_machine: StateMachine,
    pub risk_limits: RiskLimits,
    pub execution_engine: Arc<ExecutionEngine>,
    pub wallet: Arc<Keypair>,
    pub pnl_tracker: PnLTracker,
    pub telegram: Option<Arc<TelegramNotifier>>,
    pub db: Option<Arc<TradeDatabase>>,
    pub last_simulation_time: Option<Instant>,
}

impl StrategyRunner {
    pub fn new(
        execution_engine: Arc<ExecutionEngine>,
        wallet: Arc<Keypair>,
        telegram: Option<Arc<TelegramNotifier>>,
        db: Option<Arc<TradeDatabase>>,
    ) -> Self {
        Self {
            state_machine: StateMachine::new(),
            risk_limits: RiskLimits::new(),
            execution_engine,
            wallet,
            pnl_tracker: PnLTracker::new(),
            telegram,
            db,
            last_simulation_time: None,
        }
    }

    pub async fn run(&mut self, mut event_receiver: Receiver<SwapEvent>) {
        println!("🚀 Strategy runner started");

        while let Some(event) = event_receiver.recv().await {
            println!("\n🎯 Received swap event: {}", event.signature);
            self.process_event(event).await;
        }
    }

    async fn process_event(&mut self, event: SwapEvent) {
                // Step 1: Check if we're in cooldown
        if !self.state_machine.cooldown_done() {
            println!("   ⏳ In cooldown, skipping");
            
            // Optional: Send cooldown notification (uncomment if you want it)
            // if let Some(telegram) = &self.telegram {
            //     let _ = telegram.notify_text("⏳ Bot in cooldown - waiting for next trade").await;
            // }
            return;
        }

        // Step 2: Check state machine - only process in Idle or Watch state
        match self.state_machine.state {
            BotState::Idle | BotState::Watch => {
                self.state_machine.transition(BotState::Candidate);
            }
            _ => {
                println!("   ⏳ Bot busy in state {:?}, skipping", self.state_machine.state);
                return;
            }
        }

        // Step 3: Check allowlist
        let mut allowed_mints = Vec::new();
        for mint_str in &event.mints {
            if ALLOWED_TOKENS.contains(&mint_str.as_str()) {
                if let Ok(mint) = mint_str.parse::<Pubkey>() {
                    allowed_mints.push(mint);
                }
            }
        }

        if allowed_mints.is_empty() {
            println!("   ⚠️ No allowed tokens in event");
            self.state_machine.transition(BotState::Watch);
            return;
        }

        println!("   ✅ Allowed tokens: {:?}", allowed_mints);

        // Step 4: Create trade candidate (always trade WSOL→USDC for now)
        let in_mint = "So11111111111111111111111111111111111111112".parse().unwrap();
        let out_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".parse().unwrap();

        // Position sizes: 10_000_000 = 0.01 SOL, 100_000_000 = 0.1 SOL
        let amount = 10_000_000; // 0.01 SOL default

        let candidate = TradeCandidate {
            in_mint,
            out_mint,
            in_amount_lamports: amount,
            max_slippage_bps: 100, // 1%
            reason: format!("WSOL→USDC swap detected"),
        };

        // Step 5: Validate risk limits
        if let Err(e) = self.risk_limits.validate(&candidate) {
            println!("   ⚠️ Risk validation failed: {}", e);
            self.state_machine.transition(BotState::Watch);
            return;
        }

        println!("   ✅ Risk validation passed");

        // Step 6: Simulate
        self.state_machine.transition(BotState::Simulate);
        self.last_simulation_time = Some(Instant::now());

        println!("   🔄 Simulating trade...");

        // Build instruction for simulation
        let instruction = match self.execution_engine.build_swap_instruction(
            candidate.in_mint,
            candidate.out_mint,
            candidate.in_amount_lamports,
            candidate.max_slippage_bps,
        ).await {
            Ok(ix) => ix,
            Err(e) => {
                println!("   ❌ Failed to build instruction: {}", e);
                self.state_machine.transition(BotState::Watch);
                return;
            }
        };

        // Create executor for simulation
        let executor = JupiterExecutor::new(
            self.execution_engine.rpc_client.clone(),
            self.wallet.clone()
        );

        // Simulate
        if let Err(e) = executor.simulate_transaction(instruction).await {
            println!("   ❌ Simulation failed: {}", e);
            self.state_machine.set_cooldown(Duration::from_secs(10));
            return;
        }

        println!("   ✅ Simulation passed");

        // Step 7: Execute
        self.state_machine.transition(BotState::Execute);

        println!("   💰 Executing trade...");
        match self.execution_engine.execute_swap(
            candidate.in_mint,
            candidate.out_mint,
            candidate.in_amount_lamports as f64 / 1_000_000_000.0,
            candidate.max_slippage_bps,
        ).await {
            Ok(sig) => {
                println!("   ✅ Trade executed: {}", sig);
                self.state_machine.transition(BotState::Confirm);

                // Record trade in PnL
                let fee_estimate = 0.000005;
                self.pnl_tracker.record_trade(
                    sig.clone(),
                    candidate.in_mint.to_string(),
                    candidate.out_mint.to_string(),
                    candidate.in_amount_lamports as f64 / 1_000_000_000.0,
                    candidate.in_amount_lamports as f64 / 1_000_000_000.0 * 0.99,
                    fee_estimate,
                    true,
                );
                self.pnl_tracker.print_summary();

                // Send Telegram notification
                if let Some(telegram) = &self.telegram {
                    let amount = candidate.in_amount_lamports as f64 / 1_000_000_000.0;
                    let _ = telegram.notify_trade(
                        &sig,
                        &candidate.in_mint.to_string()[..8],
                        &candidate.out_mint.to_string()[..8],
                        amount
                    ).await;
                }

                // Save to database with real data
                if let Some(db) = &self.db {
                    // Fetch transaction to get real fees and amounts
                    let sig_parsed = sig.parse().ok();
                    if let Some(sig_parsed) = sig_parsed {
                        let tx_cfg = RpcTransactionConfig {
                            encoding: Some(UiTransactionEncoding::JsonParsed),
                            commitment: Some(CommitmentConfig::confirmed()),
                            max_supported_transaction_version: Some(0),
                        };

                        if let Ok(tx) = self.execution_engine.rpc_client.get_transaction_with_config(&sig_parsed, tx_cfg).await {
                            let fee_sol = extract_fee_sol(&tx);

                            let owner = self.wallet.pubkey();
                            let deltas = extract_owner_token_deltas(&tx, &owner);

                            // Find WSOL and USDC deltas
                            let wsol_mint = "So11111111111111111111111111111111111111112";
                            let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

                            let wsol_delta = deltas.iter().find(|d| d.mint == wsol_mint).map(|d| d.delta_ui).unwrap_or(0.0);
                            let usdc_delta = deltas.iter().find(|d| d.mint == usdc_mint).map(|d| d.delta_ui).unwrap_or(0.0);

                            // For a WSOL->USDC buy route:
                            let input_spent_sol = (-wsol_delta).max(0.0); // WSOL usually decreases => negative delta
                            let output_received_usdc = usdc_delta.max(0.0);
                            let price_usdc_per_sol = if input_spent_sol > 0.0 { output_received_usdc / input_spent_sol } else { 0.0 };

                            // Print REALIZED trade details
                            println!(
                                "✅ REALIZED: spent {:.6} SOL | received {:.4} USDC | fee {:.6} SOL | px {:.2} USDC/SOL",
                                input_spent_sol, output_received_usdc, fee_sol, price_usdc_per_sol
                            );

                            // Simple pnl in SOL terms (placeholder conversion)
                            let pnl_sol = -fee_sol;

                            let db_trade = DbTradeRecord {
                                id: 0,
                                signature: sig.clone(),
                                timestamp: chrono::Utc::now(),
                                input_token: wsol_mint.to_string(),
                                output_token: usdc_mint.to_string(),
                                input_amount: input_spent_sol,
                                output_amount: output_received_usdc,
                                price: price_usdc_per_sol,
                                fee_sol,
                                success: true,
                                strategy: "WSOL→USDC".to_string(),
                                pnl: pnl_sol,
                            };
                            
                            match db.insert_trade(&db_trade).await {
                                Ok(_) => println!("✅ DB INSERTED: {}", sig),
                                Err(e) => println!("❌ DB INSERT FAILED: {} | {}", sig, e),
                            }
                        }
                    }
                }

                // Cooldown after successful trade
                self.state_machine.set_cooldown(Duration::from_secs(30));
            }
            Err(e) => {
                println!("   ❌ Trade failed: {}", e);
                if let Some(telegram) = &self.telegram {
                    let _ = telegram.notify_error(&format!("Trade failed: {}", e)).await;
                }
                self.state_machine.set_cooldown(Duration::from_secs(10));
            }
        }
    }
}
