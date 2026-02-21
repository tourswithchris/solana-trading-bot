use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub signature: String,
    pub timestamp: DateTime<Utc>,
    pub input_token: String,
    pub output_token: String,
    pub input_amount: f64,
    pub output_amount: f64,
    pub price: f64,
    pub fee_sol: f64,
    pub success: bool,
    pub strategy: String,
}

#[derive(Debug, Default)]
pub struct PnLTracker {
    pub trades: Vec<TradeRecord>,
    pub total_pnl_sol: f64,
    pub total_fees_sol: f64,
    pub wins: u32,
    pub losses: u32,
    pub total_volume_sol: f64,
}

impl PnLTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_trade(&mut self, trade: TradeRecord) {
        // Calculate PnL for this trade
        let pnl = if trade.input_token.contains("So111") { // WSOL
            trade.output_amount - trade.input_amount
        } else {
            trade.input_amount - trade.output_amount
        };

        if pnl > 0.0 {
            self.wins += 1;
            self.total_pnl_sol += pnl;
        } else {
            self.losses += 1;
            self.total_pnl_sol += pnl; // negative
        }

        self.total_fees_sol += trade.fee_sol;
        self.total_volume_sol += trade.input_amount;
        self.trades.push(trade);
    }

    pub fn win_rate(&self) -> f64 {
        if self.trades.is_empty() {
            return 0.0;
        }
        (self.wins as f64 / self.trades.len() as f64) * 100.0
    }

    pub fn avg_pnl_per_trade(&self) -> f64 {
        if self.trades.is_empty() {
            return 0.0;
        }
        self.total_pnl_sol / self.trades.len() as f64
    }

    pub fn sharpe_ratio(&self) -> f64 {
        // Simplified Sharpe ratio (assuming risk-free rate = 0)
        if self.trades.len() < 2 {
            return 0.0;
        }
        
        let returns: Vec<f64> = self.trades.iter().map(|t| {
            if t.input_token.contains("So111") {
                (t.output_amount - t.input_amount) / t.input_amount
            } else {
                (t.input_amount - t.output_amount) / t.input_amount
            }
        }).collect();
        
        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / returns.len() as f64;
        let std_dev = variance.sqrt();
        
        if std_dev == 0.0 {
            return 0.0;
        }
        
        mean / std_dev
    }

    pub fn print_summary(&self) {
        println!("\n📊 PnL Summary");
        println!("   Trades: {}", self.trades.len());
        println!("   Win Rate: {:.2}%", self.win_rate());
        println!("   Total PnL: {:.6} SOL", self.total_pnl_sol);
        println!("   Total Fees: {:.6} SOL", self.total_fees_sol);
        println!("   Net PnL: {:.6} SOL", self.total_pnl_sol - self.total_fees_sol);
        println!("   Volume: {:.2} SOL", self.total_volume_sol);
        println!("   Sharpe Ratio: {:.2}", self.sharpe_ratio());
    }
}
