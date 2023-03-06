use std::error::Error;

use clap::{Parser, Subcommand};

use crate::{config::Config, db::Database, post::Post};

#[derive(Parser, Debug)]
#[clap(trailing_var_arg = true)]
pub struct Cli {
    #[command(subcommand)]
    command: Mode,
}

#[derive(Subcommand, Debug)]
enum Mode {
    Add {
        #[command(subcommand)]
        mode: Type,
    },
    Remove {
        #[command(subcommand)]
        mode: Type,
    },
    Tag {
        #[arg(long, short)]
        remove: bool,

        #[arg(required = true)]
        post_id: i64,

        #[arg(required = true)]
        tags: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum Type {
    File {
        #[arg(required = true)]
        files: Vec<String>,
    },
    Tag {
        #[arg(required = true)]
        tags: Vec<String>,
    },
}

impl Cli {
    pub fn run(config: &Config, mut db: &mut Database) -> Result<(), Box<dyn Error>> {
        let cli = Cli::parse();
        db.begin()?;

        match cli.command {
            Mode::Add { mode } => match mode {
                Type::File { files } => {
                    for file in files {
                        let post = Post::new(&file, &config, &mut db)?;
                        println!("{} -> Post #{}", file, post.id);
                    }
                }
                Type::Tag { tags } => {
                    for tag in tags {
                        let tag_id = db.get_or_create_tag(&tag)?;
                        println!("{} -> Tag #{}", tag, tag_id);
                    }
                }
            },
            Mode::Remove { mode } => match mode {
                Type::File { files } => {
                    todo!();
                }
                Type::Tag { tags } => {
                    todo!();
                }
            },
            Mode::Tag {
                remove,
                post_id,
                tags,
            } => {
                let mut post = db.get_post_id(post_id)?;
                let tag_count = post.tags.len();

                let (_, action) = match remove {
                    true => (post.remove_tags(&tags, &mut db), "Removed"),
                    false => (post.add_tags(&tags, &mut db), "Added"),
                };
                let diff = tag_count.abs_diff(post.tags.len());
                let plural = match diff {
                    1 => "",
                    _ => "s",
                };

                println!("{} {} tag{}. New {}", action, diff, plural, post);
            }
        }

        db.commit()?;
        Ok(())
    }
}
