use rusqlite::{params, Connection};
use serde_json;
use anyhow::Result;

use crate::model::Post;

/// SQLite database
pub struct Db {
    conn: Connection,
}

impl Db {
    /// Create a new instance of [Db].
    /// 
    /// Creates tables if they don't exist.
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS posts (
                id TEXT PRIMARY KEY,
                author TEXT,
                text TEXT,
                media TEXT,
                reactions TEXT,
                views TEXT,
                date TEXT
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    /// Insert a post into the database
    /// 
    /// Returns [Result]
    pub fn insert_post(&self, post: &Post) -> Result<()> {
        let media = self.to_str_json(&post.media)?;
        let reactions = self.to_str_json(&post.reactions)?;

        self.conn.execute(
            "INSERT OR REPLACE INTO posts (id, author, text, media, reactions, views, date)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                post.id,
                post.author,
                post.text,
                media,
                reactions,
                post.views,
                post.date
            ],
        )?;

        Ok(())
    }

    /// Select a post from the database
    /// 
    /// Returns [Option<Post>]
    pub fn get_posts(&self, id: &str) -> Result<Option<Post>> {
        let mut statement = self.conn.prepare(
            "SELECT id, author, text, media, reactions, views, date FROM posts WHERE id = ?1",
        )?;
        let mut rows = statement.query([id])?;

        if let Some(row) = rows.next()? {
            let media_json: String = row.get(3)?;
            let reactions_json: String = row.get(4)?;
            
            Ok(Some(Post {
                id: row.get(0)?,
                author: row.get(1)?,
                text: row.get(2)?,
                media: self.from_str_json(&media_json)?,
                reactions: self.from_str_json(&reactions_json)?,
                views: row.get(5)?,
                date: row.get(6)?,
            }))
        } else {
            return Ok(None);
        }
    }

    fn to_str_json<T: serde::Serialize>(&self, value: &Option<Vec<T>>) -> Result<String> 
    {
        let empty_vec = Vec::new();
        Ok(serde_json::to_string(value.as_ref().unwrap_or(&empty_vec))?)
    }

    fn from_str_json<T>(&self, json_str: &str) -> Result<Option<Vec<T>>> 
    where 
        T: for<'de> serde::Deserialize<'de>
    {
        Ok(serde_json::from_str(json_str).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn setup() -> Db {
        // Open in-memory connection
        let conn = Connection::open_in_memory().unwrap();
        // Manually create the same table schema
        conn.execute(
            "CREATE TABLE IF NOT EXISTS posts (
                id TEXT PRIMARY KEY,
                author TEXT,
                text TEXT,
                media TEXT,
                reactions TEXT,
                views TEXT,
                date TEXT
            )",
            [],
        ).unwrap();

        Db { conn }
    }

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

    #[test]
    fn test_insert_and_select() {
        let db = setup();
        let post = sample_post("test/1");

        db.insert_post(&post).unwrap();
        let fetched = db.get_posts(&post.id).unwrap().unwrap();

        assert_eq!(fetched, post);
    }

    #[test]
    fn test_nonexistent_post() {
        let db = setup();
        let post = db.get_posts("test/-1").unwrap();

        assert!(post.is_none());
    }
}
