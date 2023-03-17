mod cli;
mod config;
mod db;
mod gui;
mod hash;
mod message;
mod post;
mod thumbnail;
mod worker;
use cli::Cli;
use config::Config;
use db::Database;

fn main() {
    let config = Config::get();
    let db = Database::connect(config);

    if let Err(e) = Cli::run(db) {
        eprintln!("{:#?}", e);
    }
}
