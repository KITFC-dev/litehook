use rusqlite::{params, Connection};
use serde_json;
use anyhow::Result;

use crate::model::Post;

pub struct Db {
    conn: Connection,
}

impl Db {
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
