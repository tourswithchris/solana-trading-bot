use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::collections::{HashSet, HashMap};
use chrono::{DateTime, Utc, Duration};
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct TokenLaunch {
    pub mint: Pubkey,
    pub dev_wallet: Pubkey,
    pub launch_time: DateTime<Utc>,
    pub initial_liquidity_sol: f64,
    pub social_mentions: u32,
    pub verified: bool,
    pub blacklisted: bool,
}

pub struct LaunchSniper {
    blacklisted_devs: HashSet<Pubkey>,
    min_liquidity_sol: f64,
    max_launch_age_secs: i64,
    http_client: Client,
}

impl LaunchSniper {
    pub fn new() -> Self {
        Self {
            blacklisted_devs: HashSet::new(),
            min_liquidity_sol: 5.0,
            max_launch_age_secs: 60, // 60 seconds window
            http_client: Client::new(),
        }
    }

    pub async fn evaluate_launch(&self, launch: &TokenLaunch) -> Option<f64> {
        // Quick rejection checks
        if self.blacklisted_devs.contains(&launch.dev_wallet) {
            return None;
        }
        
        if launch.initial_liquidity_sol < self.min_liquidity_sol {
            return None;
        }
        
        let age = Utc::now().signed_duration_since(launch.launch_time);
        if age > Duration::seconds(self.max_launch_age_secs) {
            return None;
        }
        
        // Score the launch (0-100)
        let mut score = 50.0;
        
        // Liquidity bonus
        score += (launch.initial_liquidity_sol / 10.0).min(20.0);
        
        // Social mentions bonus (would integrate Twitter/Telegram API)
        score += (launch.social_mentions as f64 / 10.0).min(20.0);
        
        // Verified contract bonus
        if launch.verified {
            score += 10.0;
        }
        
        if score > 70.0 {
            Some(score)
        } else {
            None
        }
    }
    
    pub fn add_blacklisted_dev(&mut self, dev: Pubkey) {
        self.blacklisted_devs.insert(dev);
    }
}
