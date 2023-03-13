use blake3::{Hash, Hasher};
use std::{error::Error, fs, io, path::Path};

pub fn hash_file_blake3(path: &Path) -> Result<Hash, Box<dyn Error>> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Hasher::new();
    io::copy(&mut file, &mut hasher)?;
    Ok(hasher.finalize())
}
