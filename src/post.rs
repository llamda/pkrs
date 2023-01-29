use crate::hash;
use std::error::Error;
use std::path::Path;

#[derive(Debug)]
pub struct Post {
    pub blake3_bytes: [u8; 32],
    pub extension: Option<String>,
}

impl Post {
    pub fn new(path: &Path) -> Result<Self, Box<dyn Error>> {
        let hash = hash::hash_file_blake3(path)?;
        println!("{:?}", hash);

        let extension = path
            .extension()
            .and_then(|s| s.to_os_string().into_string().ok());

        let post = Post {
            blake3_bytes: *hash.as_bytes(),
            extension,
        };

        Ok(post)
    }
}
