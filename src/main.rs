mod config;
use config::Config;

fn main() {
    let config = Config::get();
    println!("{:?}", config);
}
