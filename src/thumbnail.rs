use std::error::Error;
use std::fs::{self, File};
use std::path::Path;

pub static THUMBNAIL_SIZE: u32 = 180;

pub fn create(from: &Path, to: &Path) -> Result<(), Box<dyn Error>> {
    let image = image::open(from)?;
    let thumbnail = image.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE);
    fs::create_dir_all(to.parent().unwrap())?;
    let mut out = File::create(to)?;
    thumbnail.write_to(&mut out, image::ImageFormat::Jpeg)?;
    Ok(())
}
