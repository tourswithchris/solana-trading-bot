use anyhow::Result;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_rpc_client_api::config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_rpc_client_api::response::{Response, RpcLogsResponse};
use solana_sdk::commitment_config::CommitmentConfig;
use futures_util::stream::StreamExt;
use std::pin::Pin;

pub async fn listen_logs(ws_url: &str) -> Result<()> {
    println!("📡 Connecting PubSub: {}", ws_url);

    // Create the client
    let client = PubsubClient::new(ws_url).await?;
    println!("✅ PubSub client created");

    let config = RpcTransactionLogsConfig {
        commitment: Some(CommitmentConfig::processed()),
    };

    // Subscribe and get the stream - using the exact type from the output
    let (stream, _subscription) = client
        .logs_subscribe(RpcTransactionLogsFilter::All, config)
        .await?;

    println!("✅ Subscribed via solana_pubsub_client");

    // Pin the stream and process messages
    let mut stream = Pin::from(stream);
    
    while let Some(response) = stream.next().await {
        let response: Response<RpcLogsResponse> = response;
        
        println!("\n=== 🔔 LOG EVENT ===");
        println!("signature: {}", response.value.signature);
        
        if let Some(err) = response.value.err {
            println!("err: {:?}", err);
        }
        
        for log in response.value.logs {
            println!("  - {}", log);
        }
    }

    Ok(())
}
