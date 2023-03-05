use crate::db::Database;
use crate::hash;
use crate::Config;
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
            tags: HashSet::new(),
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
        let db_location = db_folder.join(post.get_file());

        fs::copy(path, db_location)?;

        Ok(post)
    }

    pub fn add_tag(&mut self, tag: &String, db: &Database) -> Result<i64, Box<dyn Error>> {
        if self.tags.contains(tag) {
            return Ok(0);
        }

        self.tags.insert(tag.to_string());
        let tag_id = db.get_or_create_tag(&tag)?;
        Ok(db.insert_tagging(self.id, tag_id)?)
    }

    pub fn add_tags(
        &mut self,
        tags: &Vec<String>,
        db: &mut Database,
    ) -> Result<(), Box<dyn Error>> {
        db.begin()?;
        for tag in tags {
            self.add_tag(&tag, &db)?;
        }
        db.commit()?;
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
        db.begin()?;
        for tag in tags {
            self.remove_tag(&tag, &db)?;
        }
        db.commit()?;
        Ok(())
    }

    pub fn get_file(&self) -> PathBuf {
        let hex = Hash::from(self.blake3_bytes).to_hex().to_string();
        let mut path = Path::new(&hex).to_owned();

        if let Some(ext) = &self.extension {
            path.set_extension(&ext);
        }
        return path;
    }

    pub fn get_file_string(&self) -> String {
        self.get_file().into_os_string().into_string().unwrap()
    }

    pub fn get_tag_string(&self) -> String {
        self.tags
            .clone()
            .into_iter()
            .collect::<Vec<String>>()
            .join(",")
    }
}

impl fmt::Display for Post {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Post {{\n  id:{}\n  file: {}\n  tags: [{}]\n}}",
            self.id,
            self.get_file_string(),
            self.get_tag_string(),
        )
    }
}
