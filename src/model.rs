#[allow(dead_code)]
pub struct Post {
    pub id: String,
    pub author: Option<String>,
    pub images: Option<Vec<String>>,
    pub text: Option<String>,
    pub reactions: Option<Vec<String>>,
    pub views: Option<String>,
    pub date: Option<String>,
}
