use scraper::{ElementRef, Html, Selector};
use tokio::time::{sleep, Duration};
use anyhow::{Ok, Result, anyhow};
use html_to_markdown_rs::convert;
use reqwest::Client;

use crate::model::{Channel, ChannelCounters, Post, TmePage, WebhookPayload};

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

pub async fn fetch_html(client: &Client, url: &str) -> Result<String> {
    Ok(client.get(url).send().await?.text().await?)
}

pub async fn send_webhook(
    client: &Client,
    url: &str, 
    channel: &Channel,
    new_posts: &Vec<Post>,
    secret: Option<&str>
) -> Result<reqwest::Response> {
    let payload = WebhookPayload {
        channel,
        new_posts
    };
    
    let res = client
        .post(url)
        .header("x-secret", secret.unwrap_or(""))
        .json(&payload)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(anyhow!(res.status()));
    }

    Ok(res)
}

pub async fn send_webhook_retry(
    client: &Client,
    url: &str, 
    channel: &Channel,
    new_posts: &Vec<Post>,
    secret: Option<&str>,
    max_retries: u64
) -> Result<reqwest::Response> {
    for att in 1..=max_retries {
        let res = send_webhook(client, url, channel, new_posts, secret).await;
        if res.is_ok() {
            return res;
        } else if att < max_retries {
            tracing::warn!("webhook failed ({}/{}): {}", att, max_retries, res.unwrap_err());
            sleep(Duration::from_secs(1 * att)).await;
        }
    }

    Err(anyhow!("webhook failed after {} attempts", max_retries))
}

fn parse_counters(container: ElementRef<'_>) -> Result<ChannelCounters> {
    let counter_block_sel = Selector::parse("div.tgme_channel_info_counter").unwrap();
    let value_sel = Selector::parse("span.counter_value").unwrap();
    let type_sel = Selector::parse("span.counter_type").unwrap();
    let mut data = ChannelCounters {
        subscribers: None,
        photos: None,
        videos: None,
        links: None,
    };

    for block in container.select(&counter_block_sel) {
        let value = block
            .select_first(&value_sel)
            .map(|v| v.whole_text())
            .unwrap_or_default();

        let kind = block
            .select_first(&type_sel)
            .map(|v| v.whole_text())
            .unwrap_or_default();

        match kind.as_str() {
            "subscriber" => data.subscribers = Some(value),
            "subscribers" => data.subscribers = Some(value),
            "photo" => data.photos = Some(value),
            "photos" => data.photos = Some(value),
            "video" => data.videos = Some(value),
            "videos" => data.videos = Some(value),
            "link" => data.links = Some(value),
            "links" => data.links = Some(value),
            _ => {}
        }
    }

    Ok(data)
}

fn parse_channel(channel: ElementRef<'_>) -> Result<Channel> {
    let id_sel = Selector::parse("div.tgme_channel_info_header_username a").unwrap();
    let counters_sel = Selector::parse("div.tgme_channel_info_counters").unwrap();
    let image_sel = Selector::parse("i.tgme_page_photo_image img").unwrap();
    let name_sel = Selector::parse("div.tgme_channel_info_header_title span").unwrap();
    let desc_sel = Selector::parse("div.tgme_channel_info_description").unwrap();

    let id = channel
        .select_first(&id_sel)
        .map(|v| v.whole_text())
        .expect("channel id not found")
        .replace("@", "");

    let counters = channel
        .select_first(&counters_sel)
        .map(parse_counters)
        .transpose()?
        .unwrap();

    let name = channel
        .select_first(&name_sel)
        .map(|v| v.whole_text());

    let image = channel
        .select_first(&image_sel)
        .map(|v| v.value().attr("src").unwrap().to_string());

    let description = channel
        .select_first(&desc_sel)
        .map(|html| convert(&html.inner_html(), None))
        .transpose()?;

    let data = Channel {
        id,
        name,
        image,
        counters,
        description,
    };

    Ok(data)
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
        .expect("post id not found")
        .to_string();

    let author = element
        .select_first(&author_sel)
        .map(|el| el.whole_text());

    let text = element
        .select_first(&text_sel)
        .map(|html| convert(&html.inner_html(), None))
        .transpose()?;

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

pub async fn parse_page(html: &str) -> Result<TmePage> {
    let cnl_sel = Selector::parse("div.tgme_channel_info").unwrap();
    let post_sel = Selector::parse("div.tgme_widget_message_wrap").unwrap();
    let document = Html::parse_document(html);
    let mut posts = Vec::new();

    let channel = document
        .select(&cnl_sel)
        .next()
        .map(parse_channel)
        .transpose()?
        .unwrap();

    for post in document.select(&post_sel) {
        posts.push(parse_post(post).await?);
    }

    Ok(TmePage {
        channel,
        posts,
    })
}
