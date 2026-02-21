use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;

pub struct RiskConfig {
    pub max_position_size_sol: f64,
    pub min_position_size_sol: f64,
    pub max_slippage_bps: u16,
    pub max_price_impact_pct: f64,
    pub allowed_tokens: HashSet<Pubkey>,
    pub blocked_tokens: HashSet<Pubkey>,
    pub min_liquidity_sol: f64,
}

impl Default for RiskConfig {
    fn default() -> Self {
        let mut allowed = HashSet::new();
        // Add trusted tokens
        allowed.insert("So11111111111111111111111111111111111111112".parse().unwrap()); // WSOL
        allowed.insert("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".parse().unwrap()); // USDC
        allowed.insert("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".parse().unwrap()); // BONK

        Self {
            max_position_size_sol: 0.1,  // Max 0.1 SOL per trade
            min_position_size_sol: 0.01, // Min 0.01 SOL per trade
            max_slippage_bps: 300,       // 3% max slippage
            max_price_impact_pct: 2.0,   // 2% max price impact
            allowed_tokens: allowed,
            blocked_tokens: HashSet::new(),
            min_liquidity_sol: 1000.0,   // Min 1000 SOL liquidity
        }
    }
}

impl RiskConfig {
    pub fn can_trade(&self, input_mint: &Pubkey, output_mint: &Pubkey, amount_sol: f64) -> bool {
        // Check token allowlist
        if !self.allowed_tokens.contains(input_mint) || !self.allowed_tokens.contains(output_mint) {
            println!("⚠️ Token not in allowlist");
            return false;
        }

        // Check position size
        if amount_sol < self.min_position_size_sol {
            println!("⚠️ Position too small: {} SOL < {} SOL", amount_sol, self.min_position_size_sol);
            return false;
        }
        if amount_sol > self.max_position_size_sol {
            println!("⚠️ Position too large: {} SOL > {} SOL", amount_sol, self.max_position_size_sol);
            return false;
        }

        true
    }
}
