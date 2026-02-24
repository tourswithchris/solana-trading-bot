use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};

#[derive(Debug, Clone)]
pub struct WhaleProfile {
    pub address: Pubkey,
    pub total_trades: u32,
    pub wins: u32,
    pub losses: u32,
    pub avg_position_sol: f64,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub favorite_tokens: HashSet<String>,
    pub win_rate: f64,
}

impl WhaleProfile {
    pub fn new(address: Pubkey) -> Self {
        Self {
            address,
            total_trades: 0,
            wins: 0,
            losses: 0,
            avg_position_sol: 0.0,
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            favorite_tokens: HashSet::new(),
            win_rate: 0.0,
        }
    }

    pub fn update(&mut self, trade: &WhaleTrade) {
        self.total_trades += 1;
        if trade.profitable {
            self.wins += 1;
        } else {
            self.losses += 1;
        }
        self.win_rate = self.wins as f64 / self.total_trades as f64 * 100.0;
        self.last_seen = Utc::now();
        self.favorite_tokens.insert(trade.token_mint.clone());
    }
}

#[derive(Debug)]
pub struct WhaleTrade {
    pub whale_address: Pubkey,
    pub token_mint: String,
    pub amount_sol: f64,
    pub entry_price: f64,
    pub exit_price: Option<f64>,
    pub profitable: bool,
    pub timestamp: DateTime<Utc>,
}

pub struct WhaleTracker {
    whales: Arc<RwLock<HashMap<String, WhaleProfile>>>,
    min_whale_tx_sol: f64,
    min_whale_win_rate: f64,
}

impl WhaleTracker {
    pub fn new() -> Self {
        Self {
            whales: Arc::new(RwLock::new(HashMap::new())),
            min_whale_tx_sol: 10.0,  // 10 SOL minimum for whale status
            min_whale_win_rate: 70.0,  // 70% win rate to follow
        }
    }
    
    pub async fn process_transaction(&self, from: &Pubkey, amount_sol: f64, token: &str) -> Option<Pubkey> {
        if amount_sol < self.min_whale_tx_sol {
            return None;
        }
        
        let addr_str = from.to_string();
        let mut whales = self.whales.write().await;
        
        let profile = whales.entry(addr_str.clone()).or_insert_with(|| {
            WhaleProfile::new(*from)
        });
        
        profile.total_trades += 1;
        profile.avg_position_sol = (profile.avg_position_sol * (profile.total_trades - 1) as f64 + amount_sol) / profile.total_trades as f64;
        profile.last_seen = Utc::now();
        profile.favorite_tokens.insert(token.to_string());
        
        // Return whale address if they're successful
        if profile.total_trades > 5 && profile.win_rate > self.min_whale_win_rate {
            Some(*from)
        } else {
            None
        }
    }
    
    pub async fn get_top_whales(&self, limit: usize) -> Vec<WhaleProfile> {
        let whales = self.whales.read().await;
        let mut whales_vec: Vec<WhaleProfile> = whales.values().cloned().collect();
        whales_vec.sort_by(|a, b| b.win_rate.partial_cmp(&a.win_rate).unwrap());
        whales_vec.into_iter().take(limit).collect()
    }
    
    pub async fn should_copy_whale(&self, whale: &Pubkey, token: &str) -> bool {
        let whales = self.whales.read().await;
        if let Some(profile) = whales.get(&whale.to_string()) {
            profile.win_rate > self.min_whale_win_rate && profile.favorite_tokens.contains(token)
        } else {
            false
        }
    }
}
