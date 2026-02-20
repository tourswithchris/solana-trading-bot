use dotenv::dotenv;
use std::env;
use std::time::Duration;
use anyhow::{Result, anyhow};

#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_https_url: String,
    pub rpc_wss_url: String,
    pub private_key: Option<String>,
    pub unit_price: u64,
    pub unit_limit: u32,
    pub slippage_bps: u64,
    pub target_pubkey: Option<String>,
    pub jup_pubkey: String,
    
    // Elite features
    pub connection_timeout: Duration,
    pub rate_limit_delay: Duration,
    pub max_retries: u32,
    pub dry_run: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv().ok(); // Load .env file
        
        // Check if we're in dry-run mode (can be set by env var)
        let dry_run = env::var("DRY_RUN")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);
        
        Ok(Config {
            rpc_https_url: env::var("RPC_HTTPS_URL")
                .map_err(|_| anyhow!("RPC_HTTPS_URL not set"))?,
            
            rpc_wss_url: env::var("RPC_WSS_URL")
                .map_err(|_| anyhow!("RPC_WSS_URL not set"))?,
            
            private_key: env::var("PRIVATE_KEY").ok(),
            
            unit_price: env::var("UNIT_PRICE")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .unwrap_or(1000),
            
            unit_limit: env::var("UNIT_LIMIT")
                .unwrap_or_else(|_| "200000".to_string())
                .parse()
                .unwrap_or(200000),
            
            slippage_bps: env::var("SLIPPAGE_BPS")
                .unwrap_or_else(|_| "500".to_string())
                .parse()
                .unwrap_or(500),
            
            target_pubkey: env::var("TARGET_PUBKEY").ok(),
            
            jup_pubkey: env::var("JUP_PUBKEY")
                .unwrap_or_else(|_| "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()),
            
            // Elite defaults
            connection_timeout: Duration::from_secs(10),
            rate_limit_delay: Duration::from_millis(100),
            max_retries: 3,
            dry_run,
        })
    }
    
    pub fn validate(&self) -> Result<()> {
        if self.rpc_https_url.is_empty() {
            return Err(anyhow!("RPC_HTTPS_URL cannot be empty"));
        }
        if self.rpc_wss_url.is_empty() {
            return Err(anyhow!("RPC_WSS_URL cannot be empty"));
        }
        if self.private_key.is_none() && !self.dry_run {
            println!("⚠️  Warning: No private key set. Dry-run mode recommended.");
        }
        Ok(())
    }
    
    // Helper to check if we can execute real trades
    pub fn can_trade(&self) -> bool {
        self.private_key.is_some() && !self.dry_run
    }
}
