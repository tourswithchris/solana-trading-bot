use anyhow::Result;
use tokio::sync::mpsc::{Receiver};
use std::sync::Arc;
use solana_sdk::signature::Keypair;
use solana_sdk::pubkey::Pubkey;
use std::time::{Duration, Instant};

use crate::execution::engine::ExecutionEngine;
use crate::strategy::event::SwapEvent;
use crate::strategy::state::{StateMachine, BotState};
use crate::strategy::candidate::{TradeCandidate, RiskLimits};

// Token allowlist (only trade these)
const ALLOWED_TOKENS: &[&str] = &[
    "So11111111111111111111111111111111111111112", // Wrapped SOL
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
];

pub struct StrategyRunner {
    pub state_machine: StateMachine,
    pub risk_limits: RiskLimits,
    pub execution_engine: Arc<ExecutionEngine>,
    pub wallet: Arc<Keypair>,
    pub last_simulation_time: Option<Instant>,
}

impl StrategyRunner {
    pub fn new(execution_engine: Arc<ExecutionEngine>, wallet: Arc<Keypair>) -> Self {
        Self {
            state_machine: StateMachine::new(),
            risk_limits: RiskLimits::new(),
            execution_engine,
            wallet,
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

        // Step 4: Create trade candidate (simplified - always trade WSOL→USDC with 0.01 SOL)
        let in_mint = "So11111111111111111111111111111111111111112".parse().unwrap();
        let out_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".parse().unwrap();
        
        let candidate = TradeCandidate {
            in_mint,
            out_mint,
            in_amount_lamports: 10_000_000, // 0.01 SOL
            max_slippage_bps: 100, // 1%
            reason: "WSOL→USDC swap detected".to_string(),
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
        use crate::execution::jupiter::JupiterExecutor;
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
                
                // Cooldown after successful trade
                self.state_machine.set_cooldown(Duration::from_secs(30));
            }
            Err(e) => {
                println!("   ❌ Trade failed: {}", e);
                self.state_machine.set_cooldown(Duration::from_secs(10));
            }
        }
    }
}
