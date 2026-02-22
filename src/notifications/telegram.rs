use anyhow::{anyhow, Result};
use reqwest::Client;

pub struct TelegramNotifier {
    bot_token: String,
    chat_id: String,
    client: Client,
}

impl TelegramNotifier {
    pub fn new(bot_token: String, chat_id: String) -> Self {
        Self {
            bot_token,
            chat_id,
            client: Client::new(),
        }
    }

    pub async fn send_message(&self, message: &str) -> Result<()> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let params = [
            ("chat_id", self.chat_id.as_str()),
            ("text", message),
            ("parse_mode", "HTML"),
        ];

        let response = self.client.post(&url).form(&params).send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("Failed to send Telegram message: {}", response.status()))
        }
    }

    pub async fn notify_trade(&self, signature: &str, input: &str, output: &str, amount: f64) -> Result<()> {
        let message = format!(
            "🔄 <b>Trade Executed</b>\n\n\
             💰 Amount: {} SOL\n\
             📥 Input: {}\n\
             📤 Output: {}\n\
             🔗 <a href='https://solscan.io/tx/{}'>View on Solscan</a>",
            amount, input, output, signature
        );
        self.send_message(&message).await
    }

    pub async fn notify_error(&self, error: &str) -> Result<()> {
        let message = format!("❌ <b>Bot Error</b>\n\n{}", error);
        self.send_message(&message).await
    }

    pub async fn notify_text(&self, text: &str) -> Result<()> {
        self.send_message(text).await
    }
}
