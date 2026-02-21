use anyhow::{Ok, Result};
use html_to_markdown_rs::convert;
use reqwest::Client;
use scraper::{ElementRef, Html, Selector};
use std::sync::LazyLock as Lazy;

use crate::model::{Channel, ChannelCounters, Post, PostReaction, TmePage};

static ID_SEL: Lazy<Selector> =
    Lazy::new(|| Selector::parse("div.tgme_channel_info_header_username a").unwrap());
static COUNTERS_SEL: Lazy<Selector> =
    Lazy::new(|| Selector::parse("div.tgme_channel_info_counters").unwrap());
static IMAGE_SEL: Lazy<Selector> =
    Lazy::new(|| Selector::parse("i.tgme_page_photo_image img").unwrap());
static NAME_SEL: Lazy<Selector> =
    Lazy::new(|| Selector::parse("div.tgme_channel_info_header_title span").unwrap());
static DESC_SEL: Lazy<Selector> =
    Lazy::new(|| Selector::parse("div.tgme_channel_info_description").unwrap());

static MSG_SEL: Lazy<Selector> = Lazy::new(|| Selector::parse("div.tgme_widget_message").unwrap());
static AUTHOR_SEL: Lazy<Selector> = Lazy::new(|| {
    Selector::parse("div.tgme_widget_message_author a.tgme_widget_message_owner_name span").unwrap()
});
static TEXT_SEL: Lazy<Selector> =
    Lazy::new(|| Selector::parse("div.tgme_widget_message_text").unwrap());
static MEDIA_SEL: Lazy<Selector> =
    Lazy::new(|| Selector::parse("a.tgme_widget_message_photo_wrap").unwrap());
static REACTIONS_SEL: Lazy<Selector> =
    Lazy::new(|| Selector::parse("div.tgme_widget_message_reactions").unwrap());
static VIEWS_SEL: Lazy<Selector> =
    Lazy::new(|| Selector::parse("span.tgme_widget_message_views").unwrap());
static DATE_SEL: Lazy<Selector> =
    Lazy::new(|| Selector::parse("a.tgme_widget_message_date time").unwrap());

static COUNTER_BLOCK_SEL: Lazy<Selector> =
    Lazy::new(|| Selector::parse("div.tgme_channel_info_counter").unwrap());
static VALUE_SEL: Lazy<Selector> = Lazy::new(|| Selector::parse("span.counter_value").unwrap());
static TYPE_SEL: Lazy<Selector> = Lazy::new(|| Selector::parse("span.counter_type").unwrap());

static REACTION_SEL: Lazy<Selector> = Lazy::new(|| Selector::parse("span.tgme_reaction").unwrap());
static EMOJI_SEL: Lazy<Selector> = Lazy::new(|| Selector::parse("i.emoji b").unwrap());

static CNL_SEL: Lazy<Selector> = Lazy::new(|| Selector::parse("div.tgme_channel_info").unwrap());
static POST_SEL: Lazy<Selector> =
    Lazy::new(|| Selector::parse("div.tgme_widget_message_wrap").unwrap());

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
    let mut data = ChannelCounters {
        subscribers: None,
        photos: None,
        videos: None,
        links: None,
    };

    for block in container.select(&COUNTER_BLOCK_SEL) {
        let value = block
            .select_first(&VALUE_SEL)
            .map(|v| v.whole_text())
            .unwrap_or_default();

        let kind = block
            .select_first(&TYPE_SEL)
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
    let mut data: Vec<PostReaction> = Vec::new();

    for reaction in container.select(&REACTION_SEL) {
        let emoji = reaction
            .select_first(&EMOJI_SEL)
            .map(|v| v.whole_text())
            .unwrap_or("unknown".to_string());

        let count = reaction
            .whole_text()
            .replace(emoji.as_str(), "")
            .trim()
            .to_string();

        data.push(PostReaction {
            emoji: Some(emoji),
            count: Some(count),
        });
    }

    Ok(data)
}

fn parse_media(container: ElementRef<'_>) -> Result<Option<String>> {
    if let Some(style) = container.value().attr("style")
        && let Some(start) = style.find("url('")
    {
        let start = start + 5;
        let end = style[start..].find("')").unwrap();
        let url = style[start..start + end].to_string();
        return Ok(Some(url));
    }

    Ok(None)
}

fn parse_channel(channel: ElementRef<'_>) -> Result<Channel> {
    let id = channel
        .select_first(&ID_SEL)
        .map(|v| v.whole_text())
        .expect("channel id not found")
        .replace("@", "");

    let counters = channel
        .select_first(&COUNTERS_SEL)
        .map(parse_counters)
        .transpose()?
        .unwrap();

    let name = channel.select_first(&NAME_SEL).map(|v| v.whole_text());

    let image = channel
        .select_first(&IMAGE_SEL)
        .map(|v| v.value().attr("src").unwrap().to_string());

    let description = channel
        .select_first(&DESC_SEL)
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
    let id = post
        .select_first(&MSG_SEL)
        .expect("post not found")
        .value()
        .attr("data-post")
        .expect("post id not found")
        .to_string();

    let author = post.select_first(&AUTHOR_SEL).map(|el| el.whole_text());

    let text = post
        .select_first(&TEXT_SEL)
        .map(|html| convert(&html.inner_html(), None))
        .transpose()?;

    let media_vec: Vec<String> = post
        .select(&MEDIA_SEL)
        .filter_map(|el| parse_media(el).ok().flatten())
        .collect();
    let media = (!media_vec.is_empty()).then_some(media_vec);

    let reactions = post
        .select_first(&REACTIONS_SEL)
        .map(parse_reactions)
        .transpose()?;

    let views = post.select_first(&VIEWS_SEL).map(|el| el.whole_text());

    let date = post
        .select_first(&DATE_SEL)
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
/// Returns [TmePage] or None if page is invalid
pub async fn parse_page(html: &str) -> Result<Option<TmePage>> {
    let document = Html::parse_document(html);
    let mut posts = Vec::new();

    // Try to parse channel, return None if invalid
    let channel = match document
        .select(&CNL_SEL)
        .next()
        .map(parse_channel)
        .transpose()?
    {
        Some(c) => c,
        None => return Ok(None),
    };

    for post in document.select(&POST_SEL) {
        posts.push(parse_post(post).await?);
    }

    Ok(Some(TmePage { channel, posts }))
}
