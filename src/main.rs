#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod cli;
mod config;
mod db;
mod gui;
mod hash;
mod message;
mod post;
mod search;
mod thumbnail;
mod worker;
use cli::Cli;
use config::Config;
use db::Database;
use std::env;

fn main() {
    let config = Config::get();
    let db = Database::connect(config);

    let run_result = match env::args().len() < 2 {
        true => gui::run(db).map_err(|e| e.into()),
        false => Cli::run(db),
    };

    if let Err(e) = run_result {
        eprintln!("{:#?}", e);
    }
}
