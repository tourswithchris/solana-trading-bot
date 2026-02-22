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

    // Try multiple addresses
    let addrs = [
        format!("0.0.0.0:{}", port),
        format!("127.0.0.1:{}", port),
        format!("localhost:{}", port),
    ];
    
    for addr in &addrs {
        println!("🌐 Trying to bind to http://{}", addr);
    }
    
    let listener = tokio::net::TcpListener::bind(&addrs[0]).await?;
    let local_addr = listener.local_addr()?;
    println!("✅ Web dashboard successfully bound to http://{}", local_addr);
    println!("   Try also: http://127.0.0.1:{}", port);
    println!("   Or WSL IP: http://172.19.39.96:{}", port);
    
    axum::serve(listener, app).await?;

    Ok(())
}
