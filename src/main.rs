mod config;
mod db;
mod hash;
use config::Config;
use db::Database;
fn main() {
    let config = Config::get();

    println!("{:?}", config);

    let db = Database::connect(config.db_sql_path);
    println!("{:?}", db);
}
