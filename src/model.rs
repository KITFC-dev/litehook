use serde::Serialize;

pub struct Post {
    pub id: String,
    pub author: Option<String>,
    pub text: Option<String>,
    pub views: Option<String>,
    pub date: Option<String>,
}

#[derive(Serialize)]
pub struct WebhookPayload {
    pub id: String,
    pub author: Option<String>,
    pub text: Option<String>,
    pub views: Option<String>,
    pub date: Option<String>,
}
