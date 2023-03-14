mod cli;
mod config;
mod db;
mod hash;
mod post;
mod thumbnail;
use cli::Cli;
use config::Config;
use db::Database;

fn main() {
    let config = Config::get();
    let mut db = Database::connect(config);

    if let Err(e) = Cli::run(&mut db) {
        eprintln!("{:#?}", e);
    }
}
