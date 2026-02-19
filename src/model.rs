use sqlx::FromRow;
use sqlx::types::Json;
use serde::{Deserialize, Serialize};

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