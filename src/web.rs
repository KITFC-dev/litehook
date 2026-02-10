use scraper::{ElementRef, Html, Selector};
use anyhow::{Ok, Result, anyhow};
use reqwest::Client;
use std::time::Duration;

use crate::model::Post;

pub async fn fetch_html(url: &str) -> Result<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("litehook/0.1")
        .build()?;

    let html = client.get(url).send().await?.text().await?;
    Ok(html)
}

async fn parse_post(post: ElementRef<'_>) -> Result<Post> {
    let msg_sel = Selector::parse("div.tgme_widget_message").unwrap();
    let element = post
        .select(&msg_sel)
        .next()
        .ok_or_else(|| anyhow!("No message found in post"))?;

    let id = element
        .value()
        .attr("data-post")
        .ok_or_else(|| anyhow!("data-post attribute missing"))?
        .to_string();

    Ok(Post{
        id,
        metadata: None,
        author: None,
        images: None,
        text: None,
        reactions: None,
        views: None,
        date: None,
        
    })
}

pub async fn parse_posts(html: &str) -> Result<Vec<Post>> {
    let document = Html::parse_document(html);
    let mut posts = Vec::new();

    let post_sel = Selector::parse("div.tgme_widget_message_wrap").unwrap();

    for post in document.select(&post_sel) {
        posts.push(parse_post(post).await?);
    }

    Ok(posts)
}
