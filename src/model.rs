use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::types::Json;

use crate::config::ListenerConfig;

/// Post reactions
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
pub struct PostReaction {
    pub emoji: Option<String>,
    pub count: Option<String>,
}

/// DB row for Telegram Post
#[derive(FromRow)]
pub struct PostRow {
    pub id: String,
    pub author: String,
    pub text: String,
    pub media: Json<Option<Vec<String>>>,
    pub reactions: Json<Option<Vec<PostReaction>>>,
    pub views: String,
    pub date: String,
}

/// DB row for Listener
#[derive(Serialize, FromRow)]
pub struct ListenerRow {
    pub id: String,
    pub active: bool,
    pub poll_interval: i64,
    pub channel_url: String,
    pub proxy_list_url: Option<String>,
    pub webhook_url: String,
}

/// Telegram post
#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct Post {
    pub id: String,
    pub author: Option<String>,
    pub text: Option<String>,
    pub media: Option<Vec<String>>,
    pub reactions: Option<Vec<PostReaction>>,
    pub views: Option<String>,
    pub date: Option<String>,
}

/// Telegram channel counters
///
/// Values are strings from channel's page counters (e.g. "1.8M", "1.2k")
#[derive(Serialize, Debug)]
pub struct ChannelCounters {
    pub subscribers: Option<String>,
    pub photos: Option<String>,
    pub videos: Option<String>,
    pub links: Option<String>,
}

/// Telegram channel
#[derive(Serialize, Debug)]
pub struct Channel {
    pub id: String,
    pub name: Option<String>,
    pub image: Option<String>,
    pub counters: ChannelCounters,
    pub description: Option<String>,
}

/// Webhook payload with channel and new posts
#[derive(Serialize, Debug)]
pub struct WebhookPayload<'a> {
    pub channel: &'a Channel,
    pub new_posts: &'a [Post],
}

/// Parsed Telegram channel public page
pub struct TmePage {
    pub channel: Channel,
    pub posts: Vec<Post>,
}

/// Health check result
#[derive(Serialize)]
pub struct Health {
    pub ok: bool,
    pub listeners: usize,
}

/// Convert PostRow to Post
impl From<PostRow> for Post {
    fn from(row: PostRow) -> Self {
        Self {
            id: row.id,
            author: Some(row.author),
            text: Some(row.text),
            media: row.media.0,
            reactions: row.reactions.0,
            views: Some(row.views),
            date: Some(row.date),
        }
    }
}

/// Convert ListenerConfig to ListenerRow
impl From<ListenerConfig> for ListenerRow {
    fn from(cfg: ListenerConfig) -> Self {
        Self {
            id: cfg.id,
            active: true,
            poll_interval: cfg.poll_interval.expect("valid poll interval"),
            channel_url: cfg.channel_url,
            proxy_list_url: cfg.proxy_list_url,
            webhook_url: cfg.webhook_url.expect("valid webhook url"),
        }
    }
}

/// Convert ListenerRow to ListenerConfig
impl From<ListenerRow> for ListenerConfig {
    fn from(row: ListenerRow) -> Self {
        Self {
            id: row.id,
            poll_interval: Some(row.poll_interval),
            channel_url: row.channel_url,
            proxy_list_url: row.proxy_list_url,
            webhook_url: Some(row.webhook_url),
            webhook_secret: None,
        }
    }
}
