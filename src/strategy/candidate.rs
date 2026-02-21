use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone)]
pub struct TradeCandidate {
    pub in_mint: Pubkey,
    pub out_mint: Pubkey,
    pub in_amount_lamports: u64,
    pub max_slippage_bps: u16,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct RiskLimits {
    pub max_slippage_bps: u16,
    pub max_trade_lamports: u64,
    pub min_trade_lamports: u64,
    pub cooldown_secs: u64,
}

impl RiskLimits {
    pub fn new() -> Self {
        Self {
            max_slippage_bps: 300,           // 3%
            max_trade_lamports: 100_000_000, // 0.1 SOL
            min_trade_lamports: 10_000_000,  // 0.01 SOL
            cooldown_secs: 30,
        }
    }

    pub fn validate(&self, c: &TradeCandidate) -> Result<(), String> {
        if c.max_slippage_bps > self.max_slippage_bps {
            return Err(format!("slippage too high: {} bps", c.max_slippage_bps));
        }
        if c.in_amount_lamports > self.max_trade_lamports {
            return Err(format!("size too large: {}", c.in_amount_lamports));
        }
        if c.in_amount_lamports < self.min_trade_lamports {
            return Err(format!("size too small: {}", c.in_amount_lamports));
        }
        Ok(())
    }
}
