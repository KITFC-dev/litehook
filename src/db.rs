use anyhow::Result;
use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::types::Json;
use std::path::Path;
use tokio::fs;

use crate::model::{Post, PostRow, ListenerRow};

/// SQLite database
#[derive(Clone)]
pub struct Db {
    /// SQLite connection pool
    pub pool: SqlitePool,
}

impl Db {
    /// Create a new instance of [Db].
    ///
    /// Creates tables if they don't exist.
    pub async fn new(path: &str) -> Result<Self> {
        // Ensure path exists
        let path_ = Path::new(path);
        if let Some(parent) = path_.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(path_)
            .await?;

        // Configure connection pool
        let (url, conns) = if path == "memory" {
            (":memory:".to_string(), 1)
        } else {
            (format!("sqlite://{}", path), 32)
        };

        let pool = SqlitePoolOptions::new()
            .max_connections(conns)
            .connect(&url)
            .await?;

        // Create tables
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS posts (
                id TEXT PRIMARY KEY,
                author TEXT,
                text TEXT,
                media TEXT,
                reactions TEXT,
                views TEXT,
                date TEXT
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS listeners (
                id TEXT PRIMARY KEY,
                active BOOLEAN,
                poll_interval INTEGER,
                channel_url TEXT,
                proxy_list_url TEXT,
                webhook_url TEXT
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        Ok(Self { pool })
    }

    /// Insert a post into the database
    pub async fn insert_post(&self, post: &Post) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO posts 
            (id, author, text, media, reactions, views, date)
            VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&post.id)
        .bind(&post.author)
        .bind(&post.text)
        .bind(Json(&post.media))
        .bind(Json(&post.reactions))
        .bind(&post.views)
        .bind(&post.date)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Select a post from the database
    pub async fn get_posts(&self, id: &str) -> Result<Option<Post>> {
        let row: Option<PostRow> = sqlx::query_as(
            "SELECT id, author, text, media, reactions, views, date 
            FROM posts WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn insert_listener(&self, cfg: ListenerRow) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO listeners
            (id, active, poll_interval, channel_url, proxy_list_url, webhook_url)
            VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&cfg.id)
        .bind(&cfg.active)
        .bind(cfg.poll_interval)
        .bind(&cfg.channel_url)
        .bind(&cfg.proxy_list_url)
        .bind(&cfg.webhook_url)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_listener(&self, id: &str) -> Result<Option<ListenerRow>> {
        let row: Option<ListenerRow> = sqlx::query_as(
            "SELECT id, active, poll_interval, channel_url, proxy_list_url, webhook_url
            FROM listeners WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn get_all_listeners(&self) -> Result<Vec<ListenerRow>> {
        let rows: Vec<ListenerRow> = sqlx::query_as(
            "SELECT id, active, poll_interval, channel_url, proxy_list_url, webhook_url
            FROM listeners",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    pub async fn delete_listener(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM listeners WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::model::PostReaction;

    use super::*;

    fn sample_post(id: &str) -> Post {
        Post {
            id: id.to_string(),
            author: Some("Author".to_string()),
            text: Some("This is a test!".to_string()),
            media: Some(vec!["https://example.com/image.png".to_string()]),
            reactions: Some(vec![
                PostReaction {
                    emoji: Some("👍".to_string()),
                    count: Some("5.7K".to_string()),
                },
                PostReaction {
                    emoji: Some("🩷".to_string()),
                    count: Some("39".to_string()),
                },
            ]),
            views: Some("1.5K".to_string()),
            date: Some("2026-02-14T15:45:21+00:00".to_string()),
        }
    }

    #[tokio::test]
    async fn test_insert_and_select() {
        let db = Db::new(":memory:").await.unwrap();
        let post = sample_post("test/1");

        db.insert_post(&post).await.unwrap();
        let fetched = db.get_posts(&post.id).await.unwrap();

        assert_eq!(fetched, Some(post));
    }

    #[tokio::test]
    async fn test_nonexistent_post() {
        let db = Db::new(":memory:").await.unwrap();
        let post = db.get_posts("test/-1").await.unwrap();

        assert!(post.is_none());
    }
}
