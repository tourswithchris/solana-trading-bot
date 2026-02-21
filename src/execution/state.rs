use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ExecutionState {
    pub is_executing: Arc<Mutex<bool>>,
    pub last_execution: Arc<Mutex<Instant>>,
    pub cooldown_seconds: u64,
    pub max_trades_per_minute: u32,
    pub trade_count: Arc<Mutex<u32>>,
    pub minute_window: Arc<Mutex<Instant>>,
}

impl ExecutionState {
    pub fn new(cooldown_seconds: u64, max_trades_per_minute: u32) -> Self {
        Self {
            is_executing: Arc::new(Mutex::new(false)),
            last_execution: Arc::new(Mutex::new(Instant::now())),
            cooldown_seconds,
            max_trades_per_minute,
            trade_count: Arc::new(Mutex::new(0)),
            minute_window: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub async fn can_execute(&self) -> bool {
        // Check if already executing
        if *self.is_executing.lock().await {
            println!("⚠️ Already executing a trade");
            return false;
        }

        // Check cooldown
        let last = *self.last_execution.lock().await;
        if last.elapsed() < Duration::from_secs(self.cooldown_seconds) {
            println!("⚠️ Cooldown period active");
            return false;
        }

        // Check rate limit
        let mut window = self.minute_window.lock().await;
        if window.elapsed() > Duration::from_secs(60) {
            *window = Instant::now();
            *self.trade_count.lock().await = 0;
        }

        let count = *self.trade_count.lock().await;
        if count >= self.max_trades_per_minute {
            println!("⚠️ Rate limit reached ({} trades/min)", self.max_trades_per_minute);
            return false;
        }

        true
    }

    pub async fn start_execution(&self) -> Result<(), &'static str> {
        if !self.can_execute().await {
            return Err("Cannot execute now");
        }
        
        let mut executing = self.is_executing.lock().await;
        *executing = true;
        Ok(())
    }

    pub async fn end_execution(&self, success: bool) {
        let mut executing = self.is_executing.lock().await;
        *executing = false;
        
        if success {
            let mut last = self.last_execution.lock().await;
            *last = Instant::now();
            
            let mut count = self.trade_count.lock().await;
            *count += 1;
        }
    }
}
