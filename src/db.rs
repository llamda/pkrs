use crate::post::Post;
use rusqlite::{Connection, Error, Result};

#[derive(Debug)]
pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn connect(path: &String) -> Self {
        let conn = Connection::open(path).expect("Failed to open the sqlite database?");
        let db = Database { conn };

        db.create_tables().expect("Failed to create a table?");
        db
    }

    fn create_tables(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS posts (
            post_id INTEGER PRIMARY KEY,
            blake3 BLOB NOT NULL UNIQUE,
            extension TEXT
        )",
            (),
        )?;

        self.conn.execute(
            "CREATE  TABLE IF NOT EXISTS tags (
            tag_id INTEGER PRIMARY KEY,
            tag_name TEXT NOT NULL UNIQUE
        )",
            (),
        )?;

        Ok(())
    }

    pub fn insert_post(&mut self, post: &Post) -> Result<i64, Error> {
        let mut stmt = self
            .conn
            .prepare_cached("INSERT OR IGNORE INTO posts (blake3, extension) VALUES (?1, ?2)")?;

        stmt.execute((&post.blake3_bytes, &post.extension))?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_tag(&mut self, name: &String) -> Result<i64, Error> {
        let mut stmt = self
            .conn
            .prepare_cached("INSERT OR IGNORE INTO tags (tag_name) VALUES (?1)")?;

        stmt.execute((name,))?;
        Ok(self.conn.last_insert_rowid())
    }
}
