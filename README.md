# 🚀 Solana Auto Trading Bot

**Advanced automated trading bot for Solana tokens using Helius WebSocket feeds with PumpFun and Raydium support**

---

## ✨ Features

### 🔥 Core Trading Features
- **Real-time Token Detection**: Monitors Helius WebSocket for new token launches and trading opportunities
- **Multi-DEX Support**: Supports both PumpFun and Raydium DEX for maximum trading flexibility
- **Automated Trading**: Executes buy/sell orders based on market conditions and strategies
- **Risk Management**: Built-in stop-loss and take-profit mechanisms
- **Position Tracking**: Monitors active positions and PnL in real-time
- **Multi-token Support**: Handles multiple concurrent token positions

### 🚀 Advanced Features
- **Nozomi Integration**: MEV protection and transaction prioritization through Nozomi
- **Zero Slot Support**: Ultra-fast transaction execution with Zero Slot integration
- **Telegram Notifications**: Real-time alerts for trades, errors, and status updates
- **Jito Integration**: MEV protection and transaction bundling support

### 🛡️ Safety & Security
- **Slippage Protection**: Configurable slippage tolerance for trades
- **Liquidity Checks**: Validates minimum liquidity before trading
- **Error Handling**: Comprehensive error handling and recovery
- **Rate Limiting**: Built-in rate limiting to prevent API abuse
- **Transaction Retry Logic**: Automatic retry with exponential backoff

### 📊 Monitoring & Analytics
- **Real-time Logging**: Detailed logging with configurable levels using tracing
- **Portfolio Tracking**: Track total PnL and trade statistics
- **Performance Metrics**: Monitor success rates and profitability
- **Transaction Monitoring**: Real-time transaction status and confirmation tracking

### 🔧 Technical Features
- **Rust Performance**: High-performance, memory-safe implementation
- **Modular Architecture**: Clean, maintainable code structure
- **WebSocket Reconnection**: Automatic reconnection with exponential backoff
- **Transaction Optimization**: Optimized for Solana's transaction model
- **Async/Await**: Full async support for concurrent operations

---

## 🚀 Installation

### Prerequisites
- **Rust** (latest stable version)
- **Cargo** (comes with Rust)
- **Solana Wallet** with SOL for trading
- **Helius API Key** (for WebSocket feeds)
- **Telegram Bot** (optional, for notifications)

### Quick Start

1. **Clone the repository**

```bash
git clone https://github.com/tourswithchris/solana-trading-bot.git
cd solana-trading-bot
```

2. **Install Rust** (if not already installed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

3. **Set up environment variables** — Create a `.env` file in the project root:

```bash
touch .env
```

4. **Configure your environment** — Edit `.env` file with your settings:

```env
# Required
SOL_PUBKEY=your_solana_public_key_here
RPC_ENDPOINT=your_helius_rpc_endpoint
RPC_WEBSOCKET_ENDPOINT=your_helius_websocket_endpoint
TARGET_PUBKEY=target_wallet_to_monitor
JUP_PUBKEY=jupiter_aggregator_pubkey

# Optional
NOZOMI_URL=your_nozomi_endpoint
NOZOMI_TIP_VALUE=0.001
ZERO_SLOT_URL=your_zeroslot_endpoint
ZERO_SLOT_TIP_VALUE=0.001
TELEGRAM_BOT_TOKEN=your_telegram_bot_token
TELEGRAM_CHAT_ID=your_telegram_chat_id
```

5. **Build and run**

```bash
# Development
cargo run

# Release build
cargo build --release
./target/release/trading-bot
```

---

## ⚙️ Configuration

### Environment Variables

| Variable | Description | Default | Required |
|---|---|---|---|
| `SOL_PUBKEY` | Your Solana public key | - | ✅ |
| `RPC_ENDPOINT` | Helius RPC endpoint | - | ✅ |
| `RPC_WEBSOCKET_ENDPOINT` | Helius WebSocket endpoint | - | ✅ |
| `TARGET_PUBKEY` | Target wallet to monitor | - | ✅ |
| `JUP_PUBKEY` | Jupiter aggregator public key | - | ✅ |
| `NOZOMI_URL` | Nozomi MEV protection endpoint | - | ❌ |
| `NOZOMI_TIP_VALUE` | Nozomi tip amount in SOL | `0.001` | ❌ |
| `ZERO_SLOT_URL` | Zero Slot endpoint | - | ❌ |
| `ZERO_SLOT_TIP_VALUE` | Zero Slot tip amount in SOL | `0.001` | ❌ |
| `TELEGRAM_BOT_TOKEN` | Telegram bot token | - | ❌ |
| `TELEGRAM_CHAT_ID` | Telegram chat ID | - | ❌ |

### Trading Configuration

You can modify trading parameters in `src/common/constants.rs`:

```rust
pub const BUY_AMOUNT_SOL: f64 = 0.01;           // SOL per trade
pub const MAX_CONCURRENT_TRADES: usize = 5;     // Max positions
pub const STOP_LOSS_PERCENTAGE: f64 = 20.0;     // Stop loss %
pub const TAKE_PROFIT_PERCENTAGE: f64 = 50.0;   // Take profit %
```

### DEX Configuration

The bot supports both PumpFun and Raydium DEX:
- **PumpFun**: For new token launches and meme coins
- **Raydium**: For established tokens with liquidity pools
- **Automatic Detection**: Bot automatically detects which DEX to use based on token characteristics

---

## 🎯 Usage

### Basic Usage

```bash
# Start the bot in development mode
cargo run

# Build and run in release mode
cargo build --release
./target/release/trading-bot
```

### Advanced Usage

The bot runs as a single executable that:
1. Connects to Helius WebSocket for real-time transaction monitoring
2. Monitors target wallet for trading opportunities
3. Executes trades on PumpFun or Raydium based on detected patterns
4. Sends notifications via Telegram (if configured)

### Command Line Options

```bash
# Run with specific configuration
cargo run -- --config custom_config.toml

# Run with debug logging
RUST_LOG=debug cargo run

# Run with specific log level
RUST_LOG=info cargo run
```

### Monitoring

The bot provides real-time monitoring through:
- **Console Logs**: Detailed logging with timestamps using the `tracing` crate
- **Telegram Notifications**: Real-time alerts for trades, errors, and status updates
- **Transaction Tracking**: Real-time transaction status and confirmation monitoring
- **Performance Metrics**: Built-in performance monitoring and statistics

---

## 📊 API Reference

### Core Modules

**Trading Engine (`src/engine/`)**
- `strategy.rs`: Contains trading strategies and swap logic
- `sniper.rs`: Implements sniper trading functionality
- `swap.rs`: Handles swap execution for both PumpFun and Raydium

**DEX Integrations (`src/dex/`)**
- `pumpfun.rs`: PumpFun DEX integration and trading logic
- `raydium.rs`: Raydium DEX integration and AMM operations

**Services (`src/services/`)**
- `nozomi.rs`: Nozomi MEV protection service
- `zeroslot.rs`: Zero Slot ultra-fast transaction service
- `telegram.rs`: Telegram notification service
- `jito.rs`: Jito MEV protection and bundling
- `rpc_client.rs`: RPC client utilities and connection management

---

## 🔧 Development

### Project Structure

```
src/
├── common/             # Common utilities and configuration
│   ├── cache.rs       # Caching utilities
│   ├── constants.rs   # Configuration constants
│   ├── logger.rs      # Logging utilities
│   ├── mod.rs         # Module declarations
│   └── utils.rs       # Helper functions
├── core/               # Core trading logic
│   ├── mod.rs         # Module declarations
│   ├── token.rs       # Token handling
│   └── tx.rs          # Transaction utilities
├── dex/                # DEX integrations
│   ├── mod.rs         # Module declarations
│   ├── pumpfun.rs     # PumpFun DEX integration
│   └── raydium.rs     # Raydium DEX integration
├── engine/             # Trading engine
│   ├── mod.rs         # Module declarations
│   ├── sniper.rs      # Sniper trading logic
│   ├── strategy.rs    # Trading strategies
│   └── swap.rs        # Swap execution logic
├── services/           # External service integrations
│   ├── bloxroute.rs   # BloxRoute integration
│   ├── jito.rs        # Jito MEV protection
│   ├── mod.rs         # Module declarations
│   ├── nozomi.rs      # Nozomi MEV protection
│   ├── rpc_client.rs  # RPC client utilities
│   ├── telegram.rs    # Telegram notifications
│   └── zeroslot.rs    # Zero Slot integration
├── lib.rs              # Library entry point
└── main.rs             # Main executable entry point
```

### Building

```bash
# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Clean build artifacts
cargo clean

# Check code without building
cargo check
```

### Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

---

## 🛡️ Security Considerations

### Wallet Security
- Never commit private keys to version control
- Use environment variables for sensitive data
- Consider using a hardware wallet for large amounts
- Regularly rotate keys and monitor transactions

### Trading Risks
- Start with small amounts to test the bot
- Monitor performance regularly
- Set appropriate stop-losses to limit downside
- Understand the risks of automated trading

### Best Practices
- Test on devnet before mainnet
- Monitor logs for errors and anomalies
- Keep the bot updated with latest changes
- Backup your configuration regularly

---

## 📈 Performance Tips

### Optimization
- Use a fast RPC endpoint for better performance
- Monitor memory usage for long-running instances
- Adjust trade frequency based on market conditions
- Use appropriate slippage settings for your strategy

### Monitoring
- Set up alerts for critical errors
- Monitor PnL regularly
- Check transaction success rates
- Review trade logs for patterns

---

## 📝 License

This project is licensed under the MIT License.

---

## ⚠️ Disclaimer

**This software is for educational purposes only. Trading cryptocurrencies involves substantial risk of loss and is not suitable for all investors. The high degree of leverage can work against you as well as for you. Before deciding to trade cryptocurrencies, you should carefully consider your investment objectives, level of experience, and risk appetite. The possibility exists that you could sustain a loss of some or all of your initial investment and therefore you should not invest money that you cannot afford to lose. You should be aware of all the risks associated with cryptocurrency trading and seek advice from an independent financial advisor if you have any doubts.**

---

## 🆘 Support

### Getting Help
- **Issues**: GitHub Issues
- **Discussions**: GitHub Discussions
- **Documentation**: Wiki

### Common Issues
- **Connection Issues**: Check your RPC endpoint and internet connection
- **Transaction Failures**: Verify wallet balance and gas settings
- **WebSocket Disconnections**: Check network stability and reconnection settings

---

**Made with ❤️ for the Solana community**

⭐ Star this repo • 🐛 Report Bug • 💡 Request Feature

---
