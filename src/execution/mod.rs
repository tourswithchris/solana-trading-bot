pub mod jupiter;
pub mod raydium;  // We'll add later
pub mod pumpfun;  // We'll add later

use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;

pub struct ExecutionConfig {
    pub simulate_before_send: bool,
    pub max_retries: u32,
    pub confirmation_timeout_secs: u64,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            simulate_before_send: true,
            max_retries: 3,
            confirmation_timeout_secs: 30,
        }
    }
}

pub enum SwapVenue {
    Jupiter,
    Raydium,
    PumpFun,
}
pub mod state;
pub mod risk;
pub mod engine;
