use std::{collections::HashSet, rc::Rc};

use crate::post::Post;
use rusqlite::{types::Value, Connection, Error, Result, Row};

#[derive(Debug)]
pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn connect(path: &String) -> Self {
        let conn = Connection::open(path).expect("Failed to open the sqlite database?");
        let db = Database { conn };

        rusqlite::vtab::array::load_module(&db.conn)
            .expect("Failed to load virtual tables module?");

        db.create_tables().expect("Failed to create a table?");
        db
    }

    fn create_tables(&self) -> Result<()> {
        self.conn.execute_batch(
            "

            CREATE TABLE IF NOT EXISTS posts (
            post_id INTEGER PRIMARY KEY,
            blake3 BLOB NOT NULL UNIQUE,
            extension TEXT,
            original_name TEXT);

            CREATE TABLE IF NOT EXISTS tags (
            tag_id INTEGER PRIMARY KEY,
            tag_name TEXT NOT NULL UNIQUE);

            CREATE TABLE IF NOT EXISTS taggings (
            tagging_id INTEGER PRIMARY KEY,
            post_id INTEGER NOT NULL,
            tag_id INTEGER NOT NULL,
            UNIQUE(post_id, tag_id) ON CONFLICT IGNORE);

        ",
        )
    }

    pub fn begin(&self) -> Result<()> {
        self.conn.execute_batch("BEGIN TRANSACTION;")
    }

    pub fn commit(&self) -> Result<()> {
        self.conn.execute_batch("COMMIT TRANSACTION;")
    }

    pub fn insert_post(&self, post: &Post) -> Result<i64, Error> {
        self.conn.prepare_cached(
            "INSERT OR IGNORE INTO posts (blake3, extension, original_name) VALUES (?1, ?2, ?3)")?
            .execute((&post.blake3_bytes, &post.extension, &post.original_name))?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn remove_post(&self, post_id: i64) -> Result<(), Error> {
        self.conn
            .prepare_cached("DELETE FROM posts WHERE post_id = (?1)")?
            .execute([post_id])?;

        self.conn
            .prepare_cached("DELETE FROM taggings WHERE post_id = (?1)")?
            .execute([post_id])?;

        Ok(())
    }

    pub fn insert_tag(&self, name: &String) -> Result<i64, Error> {
        self.conn
            .prepare_cached("INSERT OR IGNORE INTO tags (tag_name) VALUES (?1)")?
            .execute([name])?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn remove_tag(&self, tag_name: &String) -> Result<i64, Error> {
        let tag_id = self.get_tag_id(tag_name)?;

        self.conn
            .prepare_cached("DELETE FROM tags WHERE tag_id = (?1)")?
            .execute([tag_id])?;

        self.conn
            .prepare_cached("DELETE FROM taggings WHERE tag_id = (?1)")?
            .execute([tag_id])?;

        Ok(tag_id)
    }

    pub fn insert_tagging(&self, post_id: i64, tag_id: i64) -> Result<i64, Error> {
        self.conn
            .prepare_cached("INSERT OR IGNORE INTO taggings (post_id, tag_id) VALUES (?1, ?2)")?
            .execute([post_id, tag_id])?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn remove_tagging(&self, post_id: i64, tag_id: i64) -> Result<(), Error> {
        self.conn
            .prepare_cached("DELETE FROM taggings WHERE post_id = (?1) AND tag_id = (?2)")?
            .execute([post_id, tag_id])?;

        Ok(())
    }

    pub fn get_post_id(&self, post_id: i64) -> Result<Post, Error> {
        self
            .conn
            .prepare_cached("SELECT * FROM posts WHERE (post_id) = (?1)")?
            .query_row([post_id], |row| self.row_to_post(row))
    }

    pub fn get_post_blake3(&self, blake3_bytes: [u8; 32]) -> Result<Post, Error> {
        self
            .conn
            .prepare_cached("SELECT * FROM posts WHERE (blake3) = (?1)")?
            .query_row([blake3_bytes], |row| self.row_to_post(row))
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
        self
            .conn
            .prepare_cached("SELECT tag_id FROM tags WHERE (tag_name) = (?1)")?
            .query_row([name], |row| row.get(0))
    }

    pub fn get_or_create_tag(&self, name: &String) -> Result<i64, Error> {
        match self.get_tag_id(name) {
            Ok(existing) => Ok(existing),
            Err(_) => self.insert_tag(name),
        }
    }

    pub fn get_post_tags(&self, post_id: i64) -> Result<HashSet<String>, Error> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT tags.tag_name
            FROM tags, taggings
            WHERE tags.tag_id = taggings.tag_id
            AND taggings.post_id = (?1)",
        )?;

        let rows = stmt.query_map([post_id], |row| row.get(0))?;
        let mut tags = HashSet::new();
        for tag in rows {
            tags.insert(tag?);
        }

        Ok(tags)
    }

    fn to_rc_vec(vec: Vec<String>) -> Rc<Vec<Value>> {
        Rc::new(
            vec.iter()
                .map(String::from)
                .map(Value::from)
                .collect::<Vec<Value>>(),
        )
    }

    pub fn search(&self, include: Vec<String>, exclude: Vec<String>) -> Result<Vec<Post>, Error> {
        let include = Self::to_rc_vec(include);
        let exclude = Self::to_rc_vec(exclude);
        let tag_count = include.len();

        let mut stmt = self.conn.prepare_cached(
            "SELECT posts.*
            FROM posts, taggings, tags
            WHERE taggings.tag_id = tags.tag_id
            AND (tags.tag_name IN rarray(?1))
            AND posts.post_id NOT IN (
                SELECT posts.post_id
                FROM posts, taggings, tags
                WHERE posts.post_id = taggings.post_id
                AND taggings.tag_id = tags.tag_id
                AND (tags.tag_name IN rarray(?2))
            )
            AND posts.post_id = taggings.post_id
            GROUP BY posts.post_id
            HAVING COUNT(posts.post_id) = (?3)",
        )?;

        let rows = stmt.query_map((include, exclude, tag_count), |row| self.row_to_post(row))?;
        let mut posts = Vec::new();
        for post in rows {
            posts.push(post?);
        }

        Ok(posts)
    }
}
