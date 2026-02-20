use futures_util::{SinkExt, StreamExt};
use futures_util::stream::{SplitSink, SplitStream};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream};
use tokio_tungstenite::MaybeTlsStream;
use tokio::net::TcpStream;
use url::Url;
use anyhow::Result;

pub async fn connect_and_listen(ws_url: &str) -> Result<()> {
    println!("📡 Connecting to WebSocket: {}", ws_url);

    let url = Url::parse(ws_url)?;
    let (ws_stream, _) = connect_async(url).await?;

    let (mut write, mut read): (
        SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
        SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>
    ) = ws_stream.split();

    // Subscribe to transaction logs (all programs)
    let subscribe_msg = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "logsSubscribe",
        "params": [
            {"mentions": ["all"]},
            {"commitment": "processed"}
        ]
    }).to_string();

    write.send(Message::Text(subscribe_msg)).await?;
    println!("✅ Subscribed to logs");

    while let Some(message) = read.next().await {
        match message {
            Ok(Message::Text(text)) => {
                // Parse the log message
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(params) = v.get("params") {
                        if let Some(result) = params.get("result") {
                            if let Some(value) = result.get("value") {
                                let signature = value.get("signature")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("unknown");
                                
                                // Fix: Create empty vec as a separate binding
                                let empty_logs = vec![];
                                let logs = value.get("logs")
                                    .and_then(|l| l.as_array())
                                    .unwrap_or(&empty_logs);
                                
                                println!("📝 Transaction: {}", signature);
                                println!("   Logs: {} entries", logs.len());
                                
                                // Check for Program logs (like swaps)
                                for log in logs {
                                    if let Some(log_str) = log.as_str() {
                                        if log_str.contains("Program log:") {
                                            println!("   🔔 {}", log_str);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                write.send(Message::Pong(data)).await?;
            }
            Ok(Message::Close(_)) => {
                println!("⚠️ WebSocket closed");
                break;
            }
            Err(e) => {
                println!("❌ WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
