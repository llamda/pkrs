use crate::db::Database;
use crate::hash;
use crate::Config;
use std::error::Error;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub struct Post {
    pub id: i64,
    pub blake3_bytes: [u8; 32],
    pub extension: Option<String>,
    pub original_name: String,
}

impl Post {
    pub fn new(path: &String, config: &Config, db: &mut Database) -> Result<Self, Box<dyn Error>> {
        let path = Path::new(path);
        let hash = hash::hash_file_blake3(path)?;

        let extension = path
            .extension()
            .and_then(|s| s.to_os_string().into_string().ok());

        let original_name = path
            .file_name()
            .unwrap()
            .to_os_string()
            .into_string()
            .unwrap();

        let mut post = Post {
            id: 0,
            blake3_bytes: *hash.as_bytes(),
            extension,
            original_name,
        };

        let row_id = db.insert_post(&post)?;
        if row_id == 0 {
            println!("File already in database...");
            return Ok(db.get_post_blake3(post.blake3_bytes)?);
        }

        post.id = row_id;

        let hex = hash.to_hex();
        let db_folder = Path::new(&config.db_file_path)
            .join(hex.get(0..2).unwrap())
            .join(hex.get(2..4).unwrap());

        fs::create_dir_all(&db_folder)?;
        let mut db_location = db_folder.join(hex.as_str());
        if let Some(ext) = &post.extension {
            db_location.set_extension(&ext);
        }

        fs::copy(path, db_location)?;

        Ok(post)
    }

    pub fn set_tags(&self, tags: &Vec<String>, db: &mut Database) -> Result<(), Box<dyn Error>> {
        db.begin()?;
        for tag in tags {
            let tag_id = db.get_or_create_tag(&tag)?;
            let tagging_id = db.insert_tagging(self.id, tag_id)?;
            match tagging_id {
                0 => println!("Post {} already had tag {}", self.id, tag),
                _ => println!(
                    "Post {} now has tag {} (id: {}) tagging: {}",
                    self.id, tag, tag_id, tagging_id
                ),
            }
        }
        db.commit()?;
        Ok(())
    }
}
