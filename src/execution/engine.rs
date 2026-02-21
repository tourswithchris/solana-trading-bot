use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use anyhow::{anyhow, Result};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_client::nonblocking::rpc_client::RpcClient;

use crate::execution::jupiter::JupiterExecutor;

#[derive(Clone)]
pub struct ExecutionEngine {
    pub rpc_client: Arc<RpcClient>,
    pub wallet: Arc<Keypair>,
    pub is_executing: Arc<Mutex<bool>>,
    pub last_execution: Arc<Mutex<Instant>>,
    pub cooldown_seconds: u64,
    pub max_position_sol: f64,
    pub min_position_sol: f64,
}

impl ExecutionEngine {
    pub fn new(rpc_client: Arc<RpcClient>, wallet: Arc<Keypair>) -> Self {
        Self {
            rpc_client,
            wallet,
            is_executing: Arc::new(Mutex::new(false)),
            last_execution: Arc::new(Mutex::new(Instant::now())),
            cooldown_seconds: 10,  // 10 seconds between trades
            max_position_sol: 0.1,  // Max 0.1 SOL per trade
            min_position_sol: 0.01, // Min 0.01 SOL per trade
        }
    }

    pub async fn can_execute(&self) -> Result<()> {
        // Check if already executing
        if *self.is_executing.lock().await {
            return Err(anyhow!("Already executing a trade"));
        }

        // Check cooldown
        let last = *self.last_execution.lock().await;
        let elapsed = last.elapsed().as_secs();
        if elapsed < self.cooldown_seconds {
            return Err(anyhow!("Cooldown active: {}s remaining", self.cooldown_seconds - elapsed));
        }

        Ok(())
    }

    pub async fn execute_swap(
        &self,
        input_mint: Pubkey,
        output_mint: Pubkey,
        amount_sol: f64,
        slippage_bps: u16,
    ) -> Result<String> {
        // Check position size
        if amount_sol < self.min_position_sol {
            return Err(anyhow!("Position too small: {} SOL < {} SOL", amount_sol, self.min_position_sol));
        }
        if amount_sol > self.max_position_sol {
            return Err(anyhow!("Position too large: {} SOL > {} SOL", amount_sol, self.max_position_sol));
        }

        // Check if we can execute
        self.can_execute().await?;

        // Set executing flag
        {
            let mut executing = self.is_executing.lock().await;
            *executing = true;
        }

        // Convert SOL to lamports (1 SOL = 1,000,000,000 lamports)
        let amount_lamports = (amount_sol * 1_000_000_000.0) as u64;

        println!("\n🚀 EXECUTING SWAP");
        println!("   Input: {}", input_mint);
        println!("   Output: {}", output_mint);
        println!("   Amount: {} SOL ({} lamports)", amount_sol, amount_lamports);
        println!("   Slippage: {} bps", slippage_bps);

        // Create executor and simulate
        let executor = JupiterExecutor::new(self.rpc_client.clone(), self.wallet.clone());
        
        // Simulate first
        println!("🔄 Simulating transaction...");
        let instruction = executor.build_swap_instruction(
            input_mint,
            output_mint,
            amount_lamports,
            slippage_bps,
        ).await?;
        
        if let Err(e) = executor.simulate_transaction(instruction.clone()).await {
            // Reset executing flag
            {
                let mut executing = self.is_executing.lock().await;
                *executing = false;
            }
            return Err(anyhow!("Simulation failed: {}", e));
        }

        // Execute
        println!("📤 Executing transaction...");
        let signature = executor.execute_swap(
            input_mint,
            output_mint,
            amount_lamports,
            slippage_bps,
        ).await?;

        // Update last execution time and clear executing flag
        {
            let mut last = self.last_execution.lock().await;
            *last = Instant::now();
            
            let mut executing = self.is_executing.lock().await;
            *executing = false;
        }

        println!("✅ Swap completed: {}", signature);
        Ok(signature)
    }
}
