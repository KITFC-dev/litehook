use scraper::{ElementRef, Html, Selector};
use anyhow::{Ok, Result};
use html_to_markdown_rs::{convert};
use reqwest::Client;

use crate::model::{Channel, ChannelCounters, Post, PostReaction, TmePage};

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

fn parse_reactions(container: ElementRef<'_>) -> Result<Vec<PostReaction>> {
    let reaction_sel = Selector::parse("span.tgme_reaction").unwrap();
    let emoji_sel = Selector::parse("i.emoji b").unwrap();
    let mut data: Vec<PostReaction> = Vec::new();

    for reaction in container.select(&reaction_sel) {
        let emoji = reaction
            .select_first(&emoji_sel)
            .map(|v| v.whole_text())
            .unwrap_or("unknown".to_string());

        let count = reaction
            .whole_text()
            .replace(emoji.as_str(), "")
            .trim()
            .to_string();

        data.push(PostReaction { emoji: Some(emoji), count: Some(count) });
    }

    Ok(data)
}

fn parse_media(container: ElementRef<'_>) -> Result<Option<String>> {
    if let Some(style) = container.value().attr("style") {
        if let Some(start) = style.find("url('") {
            let start = start + 5;
            if let Some(end) = style[start..].find("')") {
                let url = style[start..start + end].to_string();
                return Ok(Some(url));
            }
        }
    }

    Ok(None)
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
    let media_sel = Selector::parse("a.tgme_widget_message_photo_wrap").unwrap();
    let reactions_sel = Selector::parse("div.tgme_widget_message_reactions").unwrap();
    let views_sel = Selector::parse("span.tgme_widget_message_views").unwrap();
    let date_sel = Selector::parse("a.tgme_widget_message_date time").unwrap();

    let id = post
        .select_first(&msg_sel)
        .expect("post not found")
        .value()
        .attr("data-post")
        .expect("post id not found")
        .to_string();

    let author = post
        .select_first(&author_sel)
        .map(|el| el.whole_text());

    let text = post
        .select_first(&text_sel)
        .map(|html| convert(&html.inner_html(), None))
        .transpose()?;

    let media_vec: Vec<String> = post
        .select(&media_sel)
        .filter_map(|el| parse_media(el).ok().flatten())
        .collect();

    let media = if media_vec.is_empty() {
        None
    } else {
        Some(media_vec)
    };

    let reactions = post
        .select_first(&reactions_sel)
        .map(parse_reactions)
        .transpose()?;

    let views = post
        .select_first(&views_sel)
        .map(|el| el.whole_text());

    let date = post
        .select_first(&date_sel)
        .and_then(|el| el.value().attr("datetime"))
        .map(|s| s.to_string());

    Ok(Post {
        id,
        author,
        text,
        media,
        reactions,
        views,
        date,
    })
}

/// Parse Telegram channel page
/// 
/// Parses the channel information, all visible posts on page (no scrolling),
/// 
/// Returns [TmePage]
pub async fn parse_page(html: &str) -> Result<Option<TmePage>> {
    let cnl_sel = Selector::parse("div.tgme_channel_info").unwrap();
    let post_sel = Selector::parse("div.tgme_widget_message_wrap").unwrap();
    let document = Html::parse_document(html);
    let mut posts = Vec::new();

    // Try to parse channel, return None if invalid
    let channel = match document
        .select(&cnl_sel)
        .next()
        .map(parse_channel)
        .transpose()? {
        Some(c) => c,
        None => {
            tracing::warn!("could not parse channel");
            return Ok(None);
        }
    };

    for post in document.select(&post_sel) {
        posts.push(parse_post(post).await?);
    }

    Ok(Some(TmePage {
        channel,
        posts,
    }))
}
