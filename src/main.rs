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

    let arg = env::args().nth(1).unwrap();
    let post = post::Post::new(Path::new(&arg)).unwrap();
    println!("{:?}", post);

    db.insert_post(&post).unwrap();
}
