use axum::{
    extract::State,
    response::{Html, Json},
    http::StatusCode,
};
use serde_json::{json, Value};
use std::sync::Arc;
use crate::db::trades::TradeDatabase;

pub async fn index_handler() -> Html<String> {
    let html = include_str!("../../web/dashboard.html");
    Html(html.to_string())
}

pub async fn api_stats_handler(
    State(db): State<Arc<TradeDatabase>>,
) -> Result<Json<Value>, StatusCode> {
    match db.get_stats().await {
        Ok(stats) => Ok(Json(stats)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn api_trades_handler(
    State(db): State<Arc<TradeDatabase>>,
) -> Result<Json<Value>, StatusCode> {
    match db.get_recent_trades(100).await {
        Ok(trades) => {
            let trades_json: Vec<Value> = trades.into_iter()
                .map(|t| json!({
                    "id": t.id,
                    "signature": t.signature,
                    "timestamp": t.timestamp.to_rfc3339(),
                    "input_token": t.input_token,
                    "output_token": t.output_token,
                    "input_amount": t.input_amount,
                    "output_amount": t.output_amount,
                    "price": t.price,
                    "fee_sol": t.fee_sol,
                    "success": t.success,
                    "strategy": t.strategy,
                    "pnl": t.pnl,
                }))
                .collect();
            Ok(Json(json!({ "trades": trades_json })))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
