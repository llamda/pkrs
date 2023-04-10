use crate::config::Config;
use crate::db::Database;
use crate::hash;
use crate::thumbnail;
use arrayvec::ArrayString;
use blake3::Hash;
use std::collections::HashSet;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Post {
    pub id: i64,
    pub blake3_bytes: [u8; 32],
    pub extension: Option<String>,
    pub original_name: String,
    pub tags: HashSet<String>,
}

impl Post {
    pub fn new(path: &Path, db: &mut Database) -> Result<Self, Box<dyn Error>> {
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
            tags: HashSet::new(),
        };

        let row_id = db.insert_post(&post)?;
        if row_id == 0 {
            println!("File already in database...");
            return Ok(db.get_post_blake3(post.blake3_bytes)?);
        }

        post.id = row_id;

        let file_location = post.get_db_file(&db.config);
        fs::create_dir_all(file_location.parent().unwrap())?;
        fs::copy(path, &file_location)?;

        let thumbnail_location = post.get_db_thumbnail(&db.config);
        if let Err(e) = thumbnail::create(&file_location, &thumbnail_location) {
            eprintln!("{} {}", e, post.original_name);
        }

        Ok(post)
    }

    pub fn add_tag(&mut self, tag: &String, db: &Database) -> Result<i64, Box<dyn Error>> {
        if self.tags.contains(tag) {
            return Ok(0);
        }

        self.tags.insert(tag.to_string());
        let tag_id = db.get_or_create_tag(tag)?;
        Ok(db.insert_tagging(self.id, tag_id)?)
    }

    pub fn add_tags(
        &mut self,
        tags: &Vec<String>,
        db: &mut Database,
    ) -> Result<(), Box<dyn Error>> {
        for tag in tags {
            self.add_tag(tag, db)?;
        }
        Ok(())
    }

    pub fn remove_tag(&mut self, tag: &String, db: &Database) -> Result<(), Box<dyn Error>> {
        if !self.tags.contains(tag) {
            return Ok(());
        }

        self.tags.remove(tag);
        let tag_id = db.get_tag_id(tag)?;
        db.remove_tagging(self.id, tag_id)?;
        Ok(())
    }

    pub fn remove_tags(
        &mut self,
        tags: &Vec<String>,
        db: &mut Database,
    ) -> Result<(), Box<dyn Error>> {
        for tag in tags {
            self.remove_tag(tag, db)?;
        }
        Ok(())
    }

    fn get_hash(&self) -> Hash {
        Hash::from(self.blake3_bytes)
    }

    pub fn get_db_file(&self, config: &Config) -> PathBuf {
        let hex = self.get_hash().to_hex();
        let mut path = Path::new(&config.db_file_path)
            .join(self.get_db_folder(hex))
            .join(hex.as_str());

        if let Some(ext) = &self.extension {
            path.set_extension(ext);
        }
        path
    }

    pub fn get_db_thumbnail(&self, config: &Config) -> PathBuf {
        let hex = self.get_hash().to_hex();
        let mut path = Path::new(&config.db_thumbnail_path)
            .join(self.get_db_folder(hex))
            .join(hex.as_str());
        path.set_extension("jpg");
        path
    }

    fn get_db_folder(&self, hex: ArrayString<64>) -> PathBuf {
        Path::new(&hex[0..2]).join(&hex[2..4])
    }

    pub fn get_tag_string(&self) -> String {
        let mut tags = self.tags.clone().into_iter().collect::<Vec<String>>();
        tags.sort();
        tags.join(",")
    }

    pub fn delete(self, db: &Database) -> Result<(), Box<dyn Error>> {
        db.remove_post(self.id)?;
        Ok(fs::remove_file(self.get_db_file(&db.config))?)
    }
}

impl fmt::Display for Post {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Post {{\n  id:{}\n  file: {}{}\n  tags: [{}]\n}}",
            self.id,
            self.get_hash().to_hex(),
            self.extension
                .clone()
                .map(|e| format!(".{}", e))
                .unwrap_or(String::new()),
            self.get_tag_string(),
        )
    }
}
