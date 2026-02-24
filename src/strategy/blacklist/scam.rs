use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct BlacklistedDev {
    pub wallet_address: Pubkey,
    pub reason: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub rug_count: i32,
    pub confidence_score: f64,
}

#[derive(Debug, Clone)]
pub struct RugPull {
    pub token_mint: Pubkey,
    pub dev_wallet: Pubkey,
    pub timestamp: DateTime<Utc>,
    pub liquidity_removed_sol: f64,
    pub holders_affected: i32,
}

#[derive(Debug, Clone)]
pub struct KnownScam {
    pub token_mint: Pubkey,
    pub token_symbol: String,
    pub dev_wallet: Pubkey,
    pub detection_method: String,
    pub timestamp: DateTime<Utc>,
}

pub struct ScamDetector {
    db_conn: Arc<RwLock<Connection>>,
    blacklist: Arc<RwLock<HashMap<String, BlacklistedDev>>>,
    known_scams: Arc<RwLock<HashMap<String, KnownScam>>>,
}

impl ScamDetector {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        
        Ok(Self {
            db_conn: Arc::new(RwLock::new(conn)),
            blacklist: Arc::new(RwLock::new(HashMap::new())),
            known_scams: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    pub async fn load_blacklist(&self) -> Result<()> {
        let conn = self.db_conn.read().await;
        let mut stmt = conn.prepare(
            "SELECT wallet_address, reason, first_seen, last_seen, rug_count, confidence_score FROM blacklisted_devs"
        )?;
        
        let rows = stmt.query_map([], |row| {
            let wallet_str: String = row.get(0)?;
            let wallet = Pubkey::from_str(&wallet_str).unwrap_or_default();
            let first_seen_str: String = row.get(2)?;
            let last_seen_str: String = row.get(3)?;
            
            Ok(BlacklistedDev {
                wallet_address: wallet,
                reason: row.get(1)?,
                first_seen: DateTime::parse_from_rfc3339(&first_seen_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or(Utc::now()),
                last_seen: DateTime::parse_from_rfc3339(&last_seen_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or(Utc::now()),
                rug_count: row.get(4)?,
                confidence_score: row.get(5)?,
            })
        })?;
        
        let mut blacklist = self.blacklist.write().await;
        for dev in rows {
            if let Ok(dev) = dev {
                blacklist.insert(dev.wallet_address.to_string(), dev);
            }
        }
        
        Ok(())
    }
    
    pub async fn is_blacklisted(&self, dev_wallet: &Pubkey) -> bool {
        let blacklist = self.blacklist.read().await;
        blacklist.contains_key(&dev_wallet.to_string())
    }
    
    pub async fn add_blacklisted_dev(&self, dev: BlacklistedDev) -> Result<()> {
        let conn = self.db_conn.write().await;
        conn.execute(
            "INSERT OR REPLACE INTO blacklisted_devs (wallet_address, reason, first_seen, last_seen, rug_count, confidence_score) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                dev.wallet_address.to_string(),
                dev.reason,
                dev.first_seen.to_rfc3339(),
                dev.last_seen.to_rfc3339(),
                dev.rug_count,
                dev.confidence_score,
            ],
        )?;
        
        let mut blacklist = self.blacklist.write().await;
        blacklist.insert(dev.wallet_address.to_string(), dev);
        
        Ok(())
    }
    
    pub async fn detect_rug_pull(&self, token_mint: &Pubkey, dev_wallet: &Pubkey, liquidity_removed: f64, holders: i32) -> Result<bool> {
        // Check if this looks like a rug pull
        let is_rug = liquidity_removed > 50.0 && holders > 100;
        
        if is_rug {
            let conn = self.db_conn.write().await;
            conn.execute(
                "INSERT INTO rug_pulls (token_mint, dev_wallet, timestamp, liquidity_removed_sol, holders_affected) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    token_mint.to_string(),
                    dev_wallet.to_string(),
                    Utc::now().to_rfc3339(),
                    liquidity_removed,
                    holders,
                ],
            )?;
        }
        
        Ok(is_rug)
    }
    
    pub async fn check_token_safety(&self, token_mint: &Pubkey, dev_wallet: &Pubkey) -> (bool, Vec<String>) {
        let mut warnings = Vec::new();
        let mut safe = true;
        
        // Check if dev is blacklisted
        if self.is_blacklisted(dev_wallet).await {
            warnings.push("Dev wallet is blacklisted".to_string());
            safe = false;
        }
        
        // Check known scams
        let scams = self.known_scams.read().await;
        if scams.contains_key(&token_mint.to_string()) {
            warnings.push("Token is a known scam".to_string());
            safe = false;
        }
        
        (safe, warnings)
    }
}
