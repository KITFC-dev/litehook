use sqlx::sqlite::SqlitePoolOptions;
use sqlx::types::Json;
use sqlx::{SqlitePool};
use anyhow::Result;

use crate::model::{Post, PostRow};

/// SQLite database
pub struct Db {
    /// SQLite connection pool
    pub pool: SqlitePool,
}

impl Db {
    /// Create a new instance of [Db].
    /// 
    /// Creates tables if they don't exist.
    pub async fn new(path: &str) -> Result<Self> {
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
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        Ok(Self { pool })
    }

    /// Insert a post into the database
    /// 
    /// Returns [Result]
    pub async fn insert_post(&self, post: &Post) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO posts 
            (id, author, text, media, reactions, views, date)
            VALUES (?, ?, ?, ?, ?, ?, ?)"
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
    /// 
    /// Returns [Option<Post>]
    pub async fn get_posts(&self, id: &str) -> Result<Option<Post>> {
        let row: Option<PostRow> = sqlx::query_as(
            "SELECT id, author, text, media, reactions, views, date 
            FROM posts WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sample_post(id: &str) -> Post {
        Post {
            id: id.to_string(),
            author: Some("Author".to_string()),
            text: Some("This is a test!".to_string()),
            media: Some(vec!["https://example.com/image.png".to_string()]),
            reactions: Some(vec![
                HashMap::from([
                    ("emoji".to_string(), "🩷".to_string()),
                    ("count".to_string(), "10".to_string()),
                ]),
                HashMap::from([
                    ("emoji".to_string(), "❄️".to_string()),
                    ("count".to_string(), "5".to_string()),
                ]),
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
