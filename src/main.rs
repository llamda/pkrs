mod config;
mod db;
mod hash;
use config::Config;
use db::Database;
use std::{env, path::Path};

fn main() {
    let config = Config::get();

    println!("{:?}", config);

    let mut db = Database::connect(&config.db_sql_path);
    println!("{:?}", db);

    // Test hash storage
    let arg = env::args().nth(1).unwrap();
    let result = hash::hash_file_blake3(Path::new(&arg));
    println!("{:?}", result);

    let result = db.insert_post(result.unwrap().as_bytes());
    println!("{:?}", result);
}
