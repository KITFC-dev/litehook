use scraper::{ElementRef, Html, Selector};
use anyhow::{Ok, Result, anyhow};
use reqwest::Client;
use std::time::Duration;

use crate::model::Post;

trait ElementRefExt {
    fn whole_text(&self) -> String;
    fn select_first(&self, selector: &Selector) -> Result<ElementRef<'_>>;
}

impl ElementRefExt for ElementRef<'_> {
    fn whole_text(&self) -> String {
        self.text().collect::<Vec<_>>().join("").trim().to_string()
    }

    fn select_first(&self, selector: &Selector) -> Result<ElementRef<'_>> {
        self.select(selector).next().ok_or_else(|| {
            anyhow!("No element found with selector {:?}", selector)
        })
    }
}

pub async fn fetch_html(url: &str) -> Result<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(format!(
            "{}/{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        ))
        .build()?;

    let html = client.get(url).send().await?.text().await?;
    Ok(html)
}

async fn parse_post(post: ElementRef<'_>) -> Result<Post> {
    let msg_sel = Selector::parse("div.tgme_widget_message").unwrap();
    let author_sel = Selector::parse(
        "div.tgme_widget_message_author a.tgme_widget_message_owner_name span"
    ).unwrap();
    let text_sel = Selector::parse("div.tgme_widget_message_text").unwrap();
    let views_sel = Selector::parse("div.tgme_widget_message_views").unwrap();
    let date_sel = Selector::parse("div.tgme_widget_message_date time").unwrap();

    let element = post
        .select(&msg_sel)
        .next()
        .ok_or_else(|| anyhow!("No message found in post"))?;

    let id = element
        .value()
        .attr("data-post")
        .ok_or_else(|| anyhow!("ID not found in post"))?
        .to_string();

    let author = element
        .select_first(&author_sel)?
        .whole_text();

    let text = element
        .select_first(&text_sel)?
        .whole_text();

    let views = element
        .select_first(&views_sel)?
        .whole_text();

    let date = element
        .select(&date_sel)
        .next()
        .ok_or_else(|| anyhow!("Date not found in post"))?
        .value()
        .attr("datetime")
        .ok_or_else(|| anyhow!("Date attribute not found in post"))?
        .to_string();

    Ok(Post{
        id,
        metadata: None,
        author: Some(author),
        images: None,
        text: Some(text),
        reactions: None,
        views: Some(views),
        date: Some(date),
        
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
