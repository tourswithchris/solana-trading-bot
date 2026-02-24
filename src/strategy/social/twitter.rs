use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use urlencoding;

// Twitter API response structs
#[derive(Debug, Deserialize)]
struct TwitterResponse {
    data: Option<Vec<TwitterTweetData>>,
    meta: Option<TwitterMeta>,
    errors: Option<Vec<TwitterError>>,
}

#[derive(Debug, Deserialize)]
struct TwitterTweetData {
    id: String,
    text: String,
    author_id: Option<String>,
    created_at: Option<String>,
    public_metrics: Option<TwitterMetrics>,
}

#[derive(Debug, Deserialize)]
struct TwitterMetrics {
    retweet_count: i32,
    reply_count: i32,
    like_count: i32,
    quote_count: i32,
}

#[derive(Debug, Deserialize)]
struct TwitterMeta {
    result_count: i32,
}

#[derive(Debug, Deserialize)]
struct TwitterError {
    title: String,
    detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tweet {
    pub id: String,
    pub text: String,
    pub author: String,
    pub author_followers: i32,
    pub created_at: DateTime<Utc>,
    pub retweet_count: i32,
    pub like_count: i32,
    pub reply_count: i32,
    pub quote_count: i32,
    pub sentiment_score: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct TokenSocialMetrics {
    pub token_mint: String,
    pub tweet_count_24h: i32,
    pub unique_authors_24h: i32,
    pub total_likes_24h: i32,
    pub total_retweets_24h: i32,
    pub average_sentiment: f64,
    pub mentions: Vec<Tweet>,
    pub last_updated: DateTime<Utc>,
}

pub struct TwitterAPIClient {
    bearer_token: String,
    client: reqwest::Client,
    cache: Arc<RwLock<HashMap<String, (TokenSocialMetrics, DateTime<Utc>)>>>,
    cache_ttl_minutes: i64,
}

impl TwitterAPIClient {
    pub fn new(bearer_token: String) -> Self {
        Self {
            bearer_token,
            client: reqwest::Client::new(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl_minutes: 15,
        }
    }

    pub async fn search_token_mentions(&self, token_symbol: &str, token_mint: &str, _hours: i64) -> Result<TokenSocialMetrics> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some((metrics, timestamp)) = cache.get(token_mint) {
                if Utc::now().signed_duration_since(*timestamp) < Duration::minutes(self.cache_ttl_minutes) {
                    return Ok(metrics.clone());
                }
            }
        }

        // Search Twitter for token mentions
        let query = format!("${} OR {} solana -filter:retweets", token_symbol, token_symbol);
        let encoded_query = urlencoding::encode(&query);
        let url = format!(
            "https://api.twitter.com/2/tweets/search/recent?query={}&max_results=100&tweet.fields=created_at,public_metrics,author_id&user.fields=public_metrics",
            encoded_query
        );

        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.bearer_token))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("Twitter API error ({}): {}", status, error_text));
        }

        // Parse response
        let twitter_resp: TwitterResponse = response.json().await?;
        
        // Check for API errors
        if let Some(errors) = twitter_resp.errors {
            if !errors.is_empty() {
                return Err(anyhow!("Twitter API error: {:?}", errors));
            }
        }

        // Process tweets
        let mut mentions = Vec::new();
        let mut unique_authors = std::collections::HashSet::new();
        let mut total_likes = 0;
        let mut total_retweets = 0;

        if let Some(tweets) = twitter_resp.data {
            for tweet_data in tweets {
                if let Some(metrics) = tweet_data.public_metrics {
                    let created_at = tweet_data.created_at
                        .and_then(|ts| DateTime::parse_from_rfc3339(&ts).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or(Utc::now());

                                            let author_id = tweet_data.author_id.clone().unwrap_or_default();
                        let tweet = Tweet {
                            id: tweet_data.id,
                            text: tweet_data.text,
                            author: author_id.clone(),
                            author_followers: 0, // Would need separate user lookup
                            created_at,
                            retweet_count: metrics.retweet_count,
                            like_count: metrics.like_count,
                            reply_count: metrics.reply_count,
                            quote_count: metrics.quote_count,
                            sentiment_score: None, // Would need NLP analysis
                        };
                        
                        mentions.push(tweet);
                        if !author_id.is_empty() {
                            unique_authors.insert(author_id);
                        }
                    total_likes += metrics.like_count;
                    total_retweets += metrics.retweet_count;
                }
            }
        }

        // Calculate average sentiment (placeholder - would need NLP)
        let avg_sentiment = 0.5; // Neutral for now

        let metrics = TokenSocialMetrics {
            token_mint: token_mint.to_string(),
            tweet_count_24h: mentions.len() as i32,
            unique_authors_24h: unique_authors.len() as i32,
            total_likes_24h: total_likes,
            total_retweets_24h: total_retweets,
            average_sentiment: avg_sentiment,
            mentions,
            last_updated: Utc::now(),
        };

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(token_mint.to_string(), (metrics.clone(), Utc::now()));
        }

        Ok(metrics)
    }

        pub async fn analyze_sentiment(&self, token_symbol: &str, token_mint: &str) -> Result<f64> {
        let metrics = self.search_token_mentions(token_symbol, token_mint, 24).await?;

        // Simple sentiment scoring based on engagement
        let mut score: f64 = 0.5; // Neutral baseline

        // Engagement multiplier
        if metrics.tweet_count_24h > 100 {
            score += 0.2;
        }
        if metrics.total_likes_24h > 1000 {
            score += 0.1;
        }
        if metrics.unique_authors_24h > 50 {
            score += 0.1;
        }
        if metrics.total_retweets_24h > 500 {
            score += 0.1;
        }

        // Cap between 0 and 1
        score = score.min(1.0).max(0.0);

        Ok(score)
    }
}

// Telegram Channel Monitor Implementation
pub struct TelegramChannelMonitor {
    client: reqwest::Client,
    channels: Vec<String>,
    bot_token: Option<String>,
}

impl TelegramChannelMonitor {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            channels: Vec::new(),
            bot_token: None,
        }
    }

    pub fn with_bot_token(bot_token: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            channels: Vec::new(),
            bot_token: Some(bot_token),
        }
    }

    pub fn add_channel(&mut self, channel_username: &str) {
        // Ensure channel username starts with @
        let channel = if channel_username.starts_with('@') {
            channel_username.to_string()
        } else {
            format!("@{}", channel_username)
        };
        self.channels.push(channel);
    }

    pub async fn search_channel_mentions(&self, channel: &str, token_symbol: &str) -> Result<i32> {
        if let Some(bot_token) = &self.bot_token {
            // This would require a Telegram bot that's a member of the channel
            // and has message reading permissions
            // For now, return placeholder
            println!("   📱 Searching Telegram channel {} for {}", channel, token_symbol);
            
            // In production, you would use Telegram Bot API to fetch messages
            // https://core.telegram.org/bots/api#getupdates
            let url = format!(
                "https://api.telegram.org/bot{}/getUpdates",
                bot_token
            );
            
            match self.client.get(&url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        // Parse response and count mentions
                        // This is simplified - would need proper JSON parsing
                        println!("   ✅ Telegram API connected");
                        Ok(10) // Placeholder count
                    } else {
                        Ok(0)
                    }
                }
                Err(_) => Ok(0),
            }
        } else {
            // No bot token, return mock data
            Ok(5)
        }
    }

    pub async fn monitor_all_channels(&self, token_symbol: &str) -> Result<i32> {
        let mut total_mentions = 0;
        for channel in &self.channels {
            let mentions = self.search_channel_mentions(channel, token_symbol).await?;
            total_mentions += mentions;
        }
        Ok(total_mentions)
    }
}

// Simple sentiment analyzer (placeholder)
pub fn calculate_sentiment(text: &str) -> f64 {
    // Very basic sentiment analysis based on keywords
    let positive_words = ["moon", "gem", "buy", "bullish", "profit", "gain", "🚀", "💎"];
    let negative_words = ["rug", "scam", "sell", "dump", "bearish", "loss", "💩", "🚫"];
    
    let text_lower = text.to_lowercase();
    let mut score: f64 = 0.5;
    
    for word in positive_words.iter() {
        if text_lower.contains(word) {
            score += 0.1;
        }
    }
    
    for word in negative_words.iter() {
        if text_lower.contains(word) {
            score -= 0.1;
        }
    }
    
    score.min(1.0).max(0.0)
}

