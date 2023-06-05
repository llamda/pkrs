use std::{error::Error, path::Path};

use clap::{Parser, Subcommand};

use crate::{db::Database, gui, post::Post, search};

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
    Gui,
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
    pub fn run(mut db: Database) -> Result<(), Box<dyn Error>> {
        let cli = Cli::parse();

        match cli.command {
            Mode::Add { mode } => match mode {
                AddType::File { files } => {
                    db.begin()?;
                    for file in files {
                        let post = Post::new(Path::new(&file), &mut db)?;
                        println!("{} -> Post #{}", file, post.id);
                    }
                    db.commit()?;
                }
                AddType::Tag { tags } => {
                    db.begin()?;
                    for tag in tags {
                        let tag_id = db.get_or_create_tag(&tag)?;
                        println!("{} -> Tag #{}", tag, tag_id);
                    }
                    db.commit()?;
                }
            },
            Mode::Remove { mode } => match mode {
                RemoveType::File { post_ids } => {
                    db.begin()?;
                    for post_id in post_ids {
                        let post = db.get_post_id(post_id)?;
                        println!("Removing post #{}", post.id);
                        post.delete(&db)?;
                    }
                    db.commit()?;
                }
                RemoveType::Tag { tags } => {
                    db.begin()?;
                    for tag in tags {
                        let tag_id = db.remove_tag(&tag)?;
                        println!("Removing '{}' #{}", tag, tag_id);
                    }
                    db.commit()?;
                }
            },
            Mode::Tag {
                remove,
                post_id,
                tags,
            } => {
                let mut post = db.get_post_id(post_id)?;
                let tag_count = post.tags.len();

                db.begin()?;
                let (_, action) = match remove {
                    true => (post.remove_tags(&tags, &mut db), "Removed"),
                    false => (post.add_tags(&tags, &mut db), "Added"),
                };
                db.commit()?;

                let diff = tag_count.abs_diff(post.tags.len());
                let plural = match diff {
                    1 => "",
                    _ => "s",
                };

                println!("{} {} tag{}. New {}", action, diff, plural, post);
            }

            Mode::Search { tags } => {
                let posts = search::new(tags, &mut db)?;
                for post in posts {
                    println!("{}", post);
                }
            }

            Mode::Gui => {
                gui::run(db)?;
            }
        }

        Ok(())
    }
}
