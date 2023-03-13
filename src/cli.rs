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
        mode: AddType,
    },
    Remove {
        #[command(subcommand)]
        mode: RemoveType,
    },
    Tag {
        #[arg(long, short)]
        remove: bool,

        #[arg(required = true)]
        post_id: i64,

        #[arg(required = true)]
        tags: Vec<String>,
    },

    Search {
        #[arg(required = true, allow_hyphen_values = true)]
        tags: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum AddType {
    File {
        #[arg(required = true)]
        files: Vec<String>,
    },
    Tag {
        #[arg(required = true)]
        tags: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum RemoveType {
    File {
        #[arg(required = true)]
        post_ids: Vec<i64>,
    },
    Tag {
        #[arg(required = true)]
        tags: Vec<String>,
    },
}

impl Cli {
    pub fn run(config: &Config, db: &mut Database) -> Result<(), Box<dyn Error>> {
        let cli = Cli::parse();
        db.begin()?;

        match cli.command {
            Mode::Add { mode } => match mode {
                AddType::File { files } => {
                    for file in files {
                        let post = Post::new(&file, config, db)?;
                        println!("{} -> Post #{}", file, post.id);
                    }
                }
                AddType::Tag { tags } => {
                    for tag in tags {
                        let tag_id = db.get_or_create_tag(&tag)?;
                        println!("{} -> Tag #{}", tag, tag_id);
                    }
                }
            },
            Mode::Remove { mode } => match mode {
                RemoveType::File { post_ids } => {
                    for post_id in post_ids {
                        let post = db.get_post_id(post_id)?;
                        println!("Removing post #{}", post.id);
                        post.delete(config, db)?;
                    }
                }
                RemoveType::Tag { tags } => {
                    for tag in tags {
                        let tag_id = db.remove_tag(&tag)?;
                        println!("Removing '{}' #{}", tag, tag_id);
                    }
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
                    true => (post.remove_tags(&tags, db), "Removed"),
                    false => (post.add_tags(&tags, db), "Added"),
                };
                let diff = tag_count.abs_diff(post.tags.len());
                let plural = match diff {
                    1 => "",
                    _ => "s",
                };

                println!("{} {} tag{}. New {}", action, diff, plural, post);
            }

            Mode::Search { tags } => {
                let mut include = Vec::new();
                let mut exclude = Vec::new();

                for tag in tags {
                    match tag.starts_with('-') {
                        true => exclude.push(tag[1..].to_owned()),
                        false => include.push(tag),
                    }
                }

                println!(
                    "Searching for '{}' Excluding: {}",
                    include.join(","),
                    exclude.join(",")
                );

                let posts = db.search(include, exclude)?;
                for post in posts {
                    println!("{}", post);
                }
            }
        }

        db.commit()?;
        Ok(())
    }
}
