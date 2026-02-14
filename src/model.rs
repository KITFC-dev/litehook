use std::collections::HashMap;

use serde::Serialize;

pub struct TmePage {
    pub channel: Channel,
    pub posts: Vec<Post>,
}

#[derive(Serialize, Clone)]
pub struct Post {
    pub id: String,
    pub author: Option<String>,
    pub text: Option<String>,
    pub media: Option<Vec<String>>,
    pub reactions: Option<Vec<HashMap<String, String>>>,
    pub views: Option<String>,
    pub date: Option<String>,
}

#[derive(Serialize)]
pub struct ChannelCounters {
    pub subscribers: Option<String>,
    pub photos: Option<String>,
    pub videos: Option<String>,
    pub links: Option<String>,
}

#[derive(Serialize)]
pub struct Channel {
    pub id: String,
    pub name: Option<String>,
    pub image: Option<String>,
    pub counters: ChannelCounters,
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct WebhookPayload<'a> {
    pub channel: &'a Channel,
    pub new_posts: &'a Vec<Post>,
}
