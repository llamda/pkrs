mod config;
mod db;
mod hash;
mod post;
use clap::{Parser, ValueEnum};
use config::Config;
use db::Database;
use post::Post;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

#[derive(Parser, Debug)]
#[clap(trailing_var_arg = true)]
struct Args {
    #[arg(value_enum, short, long)]
    mode: Mode,

    #[arg(long, short, default_value_t = false)]
    file: bool,

    #[arg(required = true)]
    args: Vec<String>,
}

#[derive(Clone, ValueEnum, Debug)]
enum Mode {
    File,
    Tag,
    Tagging,
}

fn main() {
    let cli = Args::parse();

    let config = Config::get();
    let mut db = Database::connect(&config.db_sql_path);

    match cli.mode {
        Mode::File => {
            for file in cli.args {
                match Post::new(&file, &config, &mut db) {
                    Ok(post) => post,
                    Err(e) => {
                        eprintln!("{}", e);
                        continue;
                    }
                };
            }
        }

        Mode::Tag => {
            let tags;
            if cli.file {
                tags = read_lines(cli.args[0].as_str())
                    .unwrap()
                    .filter_map(|line| line.ok())
                    .collect();
            } else {
                tags = cli.args;
            }

            db.begin().unwrap();
            for tag in tags {
                match db.insert_tag(&tag) {
                    Ok(0) => println!("Tag already exists in database: {}", tag),
                    Ok(n) => println!("Added {} to row {}.", tag, n),
                    Err(e) => eprintln!("{}", e),
                }
            }
            db.commit().unwrap();
        }

        Mode::Tagging => {
            let ints: Vec<u32> = cli
                .args
                .into_iter()
                .filter_map(|s| s.parse::<u32>().ok())
                .collect();

            let post_id = ints[0];
            let tag_id = ints[1];
            db.insert_tagging(post_id, tag_id).unwrap();
        }
    }
}

fn read_lines(path: &str) -> io::Result<io::Lines<BufReader<File>>> {
    let file = File::open(path)?;
    Ok(io::BufReader::new(file).lines())
}
