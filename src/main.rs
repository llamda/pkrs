mod config;
mod db;
mod hash;
mod post;
use clap::{Parser, ValueEnum};
use config::Config;
use db::Database;
use post::Post;

#[derive(Parser, Debug)]
#[clap(trailing_var_arg = true)]
struct Args {
    #[arg(value_enum, short, long)]
    mode: Mode,

    #[arg(required = true)]
    args: Vec<String>,
}

#[derive(Clone, ValueEnum, Debug)]
enum Mode {
    File,
    Tag,
}

fn main() {
    let cli = Args::parse();

    let config = Config::get();
    let mut db = Database::connect(&config.db_sql_path);

    match cli.mode {
        Mode::File => {
            for file in cli.args {
                let post = match Post::new(&file) {
                    Ok(post) => post,
                    Err(e) => {
                        eprintln!("{}", e);
                        continue;
                    }
                };

                match db.insert_post(&post) {
                    Ok(0) => println!("File already exists in database: {}", file),
                    Ok(n) => println!("Added {} to row {}.", file, n),
                    Err(e) => eprintln!("{}", e),
                }
            }
        }

        Mode::Tag => {
            for tag in cli.args {
                match db.insert_tag(&tag) {
                    Ok(0) => println!("Tag already exists in database: {}", tag),
                    Ok(n) => println!("Added {} to row {}.", tag, n),
                    Err(e) => eprintln!("{}", e),
                }
            }
        }
    }
}
