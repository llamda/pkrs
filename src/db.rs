use crate::post::Post;
use rusqlite::{Connection, Error, Result, Row};

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
            extension TEXT,
            original_name TEXT
        )",
            (),
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS tags (
            tag_id INTEGER PRIMARY KEY,
            tag_name TEXT NOT NULL UNIQUE
        )",
            (),
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS taggings (
            tagging_id INTEGER PRIMARY KEY,
            post_id INTEGER NOT NULL,
            tag_id INTEGER NOT NULL,
            UNIQUE(post_id, tag_id) ON CONFLICT IGNORE
        )",
            (),
        )?;

        Ok(())
    }

    pub fn begin(&self) -> Result<()> {
        Ok(self.conn.execute_batch("BEGIN TRANSACTION;")?)
    }

    pub fn commit(&self) -> Result<()> {
        Ok(self.conn.execute_batch("COMMIT TRANSACTION;")?)
    }

    pub fn insert_post(&self, post: &Post) -> Result<i64, Error> {
        let mut stmt = self.conn.prepare_cached(
            "INSERT OR IGNORE INTO posts (blake3, extension, original_name) VALUES (?1, ?2, ?3)",
        )?;

        stmt.execute((&post.blake3_bytes, &post.extension, &post.original_name))?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_tag(&self, name: &String) -> Result<i64, Error> {
        let mut stmt = self
            .conn
            .prepare_cached("INSERT OR IGNORE INTO tags (tag_name) VALUES (?1)")?;

        stmt.execute((name,))?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_tagging(&self, post_id: i64, tag_id: i64) -> Result<i64, Error> {
        let mut stmt = self
            .conn
            .prepare_cached("INSERT OR IGNORE INTO taggings (post_id, tag_id) VALUES (?1, ?2)")?;

        stmt.execute((post_id, tag_id))?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_post_id(&self, post_id: i64) -> Result<Post, Error> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT * FROM posts WHERE (post_id) = (?1)")?;

        Ok(stmt.query_row((post_id,), |row| self.row_to_post(row))?)
    }

    pub fn get_post_blake3(&self, blake3_bytes: [u8; 32]) -> Result<Post, Error> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT * FROM posts WHERE (blake3) = (?1)")?;

        Ok(stmt.query_row((blake3_bytes,), |row| self.row_to_post(row))?)
    }

    fn row_to_post(&self, row: &Row) -> Result<Post, Error> {
        let post_id = row.get(0)?;
        Ok(Post {
            id: post_id,
            blake3_bytes: row.get(1)?,
            extension: row.get(2)?,
            original_name: row.get(3)?,
            tags: self.get_post_tags(post_id)?,
        })
    }

    pub fn get_tag_id(&self, name: &String) -> Result<i64, Error> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT tag_id FROM tags WHERE (tag_name) = (?1)")?;

        Ok(stmt.query_row((name,), |row| row.get(0))?)
    }

    pub fn get_or_create_tag(&mut self, name: &String) -> Result<i64, Error> {
        match self.get_tag_id(name) {
            Ok(existing) => Ok(existing),
            Err(_) => self.insert_tag(name),
        }
    }

    pub fn get_post_tags(&self, post_id: i64) -> Result<Vec<String>, Error> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT tags.tag_name
            FROM tags, taggings
            WHERE tags.tag_id = taggings.tag_id
            AND taggings.post_id = (?1)",
        )?;

        let rows = stmt.query_map([post_id], |row| row.get(0))?;
        let mut tags = Vec::new();
        for tag in rows {
            tags.push(tag?);
        }

        Ok(tags)
    }
}
