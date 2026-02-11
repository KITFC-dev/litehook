use scraper::{ElementRef, Html, Selector};
use anyhow::{Ok, Result};
use reqwest::Client;
use std::time::Duration;

use crate::model::Post;

trait ElementRefExt {
    fn whole_text(&self) -> String;
    fn select_first(&self, selector: &Selector) -> Option<ElementRef<'_>>;
}

impl ElementRefExt for ElementRef<'_> {
    fn whole_text(&self) -> String {
        self.text().collect::<Vec<_>>().join("").to_string()
    }

    fn select_first(&self, selector: &Selector) -> Option<ElementRef<'_>> {
        self.select(selector).next()
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
    let views_sel = Selector::parse("span.tgme_widget_message_views").unwrap();
    let date_sel = Selector::parse("a.tgme_widget_message_date time").unwrap();

    let element = match post.select(&msg_sel).next() {
        Some(el) => el,
        None => return Ok(Post {
            id: "".to_string(),
            author: None,
            text: None,
            views: None,
            date: None,
        }),
    };

    let id = element
        .value()
        .attr("data-post")
        .unwrap()
        .to_string();

    let author = element
        .select_first(&author_sel)
        .map(|el| el.whole_text());

    let text = element
        .select_first(&text_sel)
        .map(|el| el.whole_text());

    let views = element
        .select_first(&views_sel)
        .map(|el| el.whole_text());

    let date = element
        .select_first(&date_sel)
        .and_then(|el| el.value().attr("datetime"))
        .map(|s| s.to_string());

    Ok(Post {
        id,
        author,
        text,
        views,
        date,
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
