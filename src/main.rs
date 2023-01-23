mod config;
mod hash;
use config::Config;
use std::{env, path::Path};

fn main() {
    let config = Config::get();
    println!("{:?}", config);

    // Test Hashing
    let arg = env::args().nth(1).unwrap();
    let result = hash::hash_file_blake3(Path::new(&arg));
    println!("{:?}", result);
}
