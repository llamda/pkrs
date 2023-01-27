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
            postId INTEGER PRIMARY KEY,
            blake3 BLOB NOT NULL UNIQUE
        )",
            (),
        )?;
        Ok(())
    }

    pub fn insert_post(&mut self, hash: &[u8]) -> Result<i64, Error> {
        let mut stmt = self
            .conn
            .prepare_cached("INSERT OR IGNORE INTO posts (blake3) VALUES (?1)")?;

        stmt.execute(&[hash])?;
        Ok(self.conn.last_insert_rowid())
    }
}
