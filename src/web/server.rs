use axum::{
    Router,
    routing::get,
    http::Method,
};
use tower_http::cors::{CorsLayer, Any};
use std::sync::Arc;
use anyhow::Result;
use crate::db::trades::TradeDatabase;

use super::handlers::{index_handler, api_stats_handler, api_trades_handler};

pub async fn start_web_server(db: Arc<TradeDatabase>, port: u16) -> Result<()> {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/stats", get(api_stats_handler))
        .route("/api/trades", get(api_trades_handler))
        .layer(cors)
        .with_state(db);

    let addr = format!("0.0.0.0:{}", port);
    println!("🌐 Web dashboard starting at http://{}", addr);
    
    axum::serve(
        tokio::net::TcpListener::bind(&addr).await?,
        app
    )
    .await?;

    Ok(())
}
