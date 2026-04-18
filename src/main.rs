use std::{
    fs::{self, DirEntry},
    io::{Write, stdout},
};

use image::{DynamicImage, GenericImageView, ImageReader, imageops::FilterType};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

const IMG_SIZE: u32 = 100;

fn main() -> anyhow::Result<()> {
    let cropped_images = fs::read_dir("./img")?
        .collect::<Result<Vec<_>, _>>()?
        .par_iter()
        .map(process_entry)
        .collect::<Result<Vec<_>, _>>()?;

    println!("\r\x1b[KProcessed {} images", cropped_images.len());
    Ok(())
}

fn process_entry(entry: &DirEntry) -> anyhow::Result<DynamicImage> {
    let path = entry.path();
    let image = ImageReader::open(&path)?.with_guessed_format()?.decode()?;
    let (width, height) = image.dimensions();
    let dim = u32::min(width, height);
    let cropped = image.crop_imm((width - dim) / 2, (height - dim) / 2, dim, dim);
    print!("\r{path:?} : cropped to {dim}x{dim}\x1b[K");
    stdout().flush()?;

    let scaled = cropped.resize_exact(IMG_SIZE, IMG_SIZE, FilterType::Lanczos3);
    print!("\r{path:?} : scaled to {IMG_SIZE}x{IMG_SIZE}\x1b[K");
    stdout().flush()?;

    Ok(scaled)
}
