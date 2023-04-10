use image::io::Reader as ImageReader;
use image::EncodableLayout;
use std::error::Error;
use std::fs;
use std::path::Path;

pub static THUMBNAIL_SIZE: u32 = 180;

pub fn create(from: &Path, to: &Path) -> Result<(), Box<dyn Error>> {
    let image = ImageReader::open(from)?.decode()?;
    let thumbnail = image.thumbnail(THUMBNAIL_SIZE, THUMBNAIL_SIZE);
    fs::create_dir_all(to.parent().unwrap())?;
    image::save_buffer(
        to,
        thumbnail.to_rgb8().as_bytes(),
        thumbnail.width(),
        thumbnail.height(),
        image::ColorType::Rgb8,
    )?;
    Ok(())
}
