use std::env;

fn main() {
    println!("✅ trading-bot is running.");
    println!("Args: {:?}", env::args().collect::<Vec<_>>());

    // Optional: show that env loading works later
    if let Ok(url) = env::var("RPC_HTTPS_URL") {
        println!("RPC_HTTPS_URL set: {}", url);
    } else {
        println!("RPC_HTTPS_URL not set (ok for now).");
    }
}
