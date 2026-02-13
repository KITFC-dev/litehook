use serde::Serialize;

pub struct TmePage {
    pub channel: Channel,
    pub posts: Vec<Post>,
}

pub struct Post {
    pub id: String,
    pub author: Option<String>,
    pub text: Option<String>,
    pub views: Option<String>,
    pub date: Option<String>,
}

pub struct ChannelCounters {
    pub subscribers: Option<String>,
    pub photos: Option<String>,
    pub videos: Option<String>,
    pub links: Option<String>,
}

pub struct Channel {
    pub id: String,
    pub name: Option<String>,
    pub image: Option<String>,
    pub counters: ChannelCounters,
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct WebhookPayload {
    pub id: String,
    pub author: Option<String>,
    pub text: Option<String>,
    pub views: Option<String>,
    pub date: Option<String>,
}
