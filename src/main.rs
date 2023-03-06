mod cli;
mod config;
mod db;
mod hash;
mod post;
use cli::Cli;
use config::Config;
use db::Database;

fn main() {
    let config = Config::get();
    let mut db = Database::connect(&config.db_sql_path);

    if let Err(e) = Cli::run(&config, &mut db) {
        eprintln!("{:#?}", e);
    }
}
