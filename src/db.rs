use rusqlite::{params, Connection, Result};

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
                images TEXT,
                text TEXT,
                reactions TEXT,
                views TEXT,
                date TEXT
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    pub fn insert_post(&self, post: &Post) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO posts (id, author, text, views, date)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                post.id,
                post.author,
                post.text,
                post.views,
                post.date
            ],
        )?;
        
        Ok(())
    }

    pub fn get_posts(&self, id: &str) -> Result<Option<Post>> {
        let mut statement = self.conn.prepare(
            "SELECT id, author, text, views, date FROM posts WHERE id = ?1",
        )?;
        let mut rows = statement.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Post {
                id: row.get(0)?,
                author: row.get(1)?,
                text: row.get(2)?,
                views: row.get(3)?,
                date: row.get(4)?,
                images: None,
                reactions: None,
            }))
        } else {
            return Ok(None);
        }
    }
}
