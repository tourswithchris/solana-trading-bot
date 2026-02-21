use anyhow::{anyhow, Result};
use rusqlite::{Connection, params};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub id: i64,
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
    pub pnl: f64,
}

pub struct TradeDatabase {
    conn: Arc<Mutex<Connection>>,
}

impl TradeDatabase {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Create tables if they don't exist
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS trades (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                signature TEXT NOT NULL UNIQUE,
                timestamp TEXT NOT NULL,
                input_token TEXT NOT NULL,
                output_token TEXT NOT NULL,
                input_amount REAL NOT NULL,
                output_amount REAL NOT NULL,
                price REAL NOT NULL,
                fee_sol REAL NOT NULL,
                success INTEGER NOT NULL,
                strategy TEXT NOT NULL,
                pnl REAL NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            [],
        )?;

        // Create index for faster queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_trades_timestamp ON trades(timestamp);",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_trades_success ON trades(success);",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_trades_strategy ON trades(strategy);",
            [],
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn insert_trade(&self, trade: &TradeRecord) -> Result<()> {
        let conn = self.conn.lock().await;
        
        conn.execute(
            r#"
            INSERT INTO trades (signature, timestamp, input_token, output_token, 
                               input_amount, output_amount, price, fee_sol, success, strategy, pnl)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                &trade.signature,
                trade.timestamp.to_rfc3339(),
                &trade.input_token,
                &trade.output_token,
                trade.input_amount,
                trade.output_amount,
                trade.price,
                trade.fee_sol,
                if trade.success { 1 } else { 0 },
                &trade.strategy,
                trade.pnl,
            ],
        )?;
        
        Ok(())
    }

    pub async fn get_recent_trades(&self, limit: i64) -> Result<Vec<TradeRecord>> {
        let conn = self.conn.lock().await;
        
        let mut stmt = conn.prepare(
            r#"
            SELECT id, signature, timestamp, input_token, output_token,
                   input_amount, output_amount, price, fee_sol, success, strategy, pnl
            FROM trades
            ORDER BY timestamp DESC
            LIMIT ?
            "#
        )?;

        let rows = stmt.query_map(params![limit], |row| {
            let timestamp_str: String = row.get(2)?;
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            
            Ok(TradeRecord {
                id: row.get(0)?,
                signature: row.get(1)?,
                timestamp,
                input_token: row.get(3)?,
                output_token: row.get(4)?,
                input_amount: row.get(5)?,
                output_amount: row.get(6)?,
                price: row.get(7)?,
                fee_sol: row.get(8)?,
                success: row.get::<_, i32>(9)? == 1,
                strategy: row.get(10)?,
                pnl: row.get(11)?,
            })
        })?;

        let mut trades = Vec::new();
        for trade in rows {
            trades.push(trade?);
        }

        Ok(trades)
    }

    pub async fn get_stats(&self) -> Result<serde_json::Value> {
        let conn = self.conn.lock().await;
        
        let total_trades: i64 = conn.query_row(
            "SELECT COUNT(*) FROM trades",
            [],
            |row| row.get(0),
        )?;

        let successful_trades: i64 = conn.query_row(
            "SELECT COUNT(*) FROM trades WHERE success = 1",
            [],
            |row| row.get(0),
        )?;

        let total_pnl: f64 = conn.query_row(
            "SELECT COALESCE(SUM(pnl), 0) FROM trades",
            [],
            |row| row.get(0),
        )?;

        let total_fees: f64 = conn.query_row(
            "SELECT COALESCE(SUM(fee_sol), 0) FROM trades",
            [],
            |row| row.get(0),
        )?;

        let best_trade: f64 = conn.query_row(
            "SELECT COALESCE(MAX(pnl), 0) FROM trades",
            [],
            |row| row.get(0),
        )?;

        let worst_trade: f64 = conn.query_row(
            "SELECT COALESCE(MIN(pnl), 0) FROM trades",
            [],
            |row| row.get(0),
        )?;

        Ok(serde_json::json!({
            "total_trades": total_trades,
            "successful_trades": successful_trades,
            "failed_trades": total_trades - successful_trades,
            "win_rate": if total_trades > 0 {
                (successful_trades as f64 / total_trades as f64) * 100.0
            } else { 0.0 },
            "total_pnl_sol": total_pnl,
            "total_fees_sol": total_fees,
            "net_pnl_sol": total_pnl - total_fees,
            "best_trade_sol": best_trade,
            "worst_trade_sol": worst_trade,
        }))
    }
}
