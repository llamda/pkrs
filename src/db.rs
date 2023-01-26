use rusqlite::Connection;

#[derive(Debug)]
pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn connect(path: String) -> Self {
        let conn = Connection::open(path).expect("Failed to open the sqlite database?");
        Database { conn }
    }
}
