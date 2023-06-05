use std::error::Error;

use crate::{db::Database, post::Post};

pub fn new(tags: Vec<String>, db: &mut Database) -> Result<Vec<Post>, Box<dyn Error>> {
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

    Ok(db.search(include, exclude)?)
}
