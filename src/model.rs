use std::collections::HashMap;

use serde::Serialize;

/// Parsed Telegram channel public page
pub struct TmePage {
    pub channel: Channel,
    pub posts: Vec<Post>,
}

/// Telegram post
#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct Post {
    pub id: String,
    pub author: Option<String>,
    pub text: Option<String>,
    pub media: Option<Vec<String>>,
    pub reactions: Option<Vec<HashMap<String, String>>>,
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
