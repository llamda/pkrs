mod config;
mod db;
mod hash;
mod post;
use config::Config;
use db::Database;
use std::{env, path::Path};

fn main() {
    let config = Config::get();

    println!("{:?}", config);

    let mut db = Database::connect(&config.db_sql_path);
    println!("{:?}", db);

    let results: Vec<i64> = env::args()
        .into_iter()
        .skip(1)
        .filter_map(|s| db.insert_tag(&s).ok())
        .collect();

    println!("Added tags in rows: {:?}", results);
}
