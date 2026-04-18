use std::{
    fs::{self, DirEntry},
    io::{Write, stdout},
};

use image::{DynamicImage, GenericImageView, ImageReader, imageops::FilterType};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

const CLEAR_LINE: &str = "\x1b[K";
const IMG_SIZE: u32 = 100;

fn main() -> anyhow::Result<()> {
    let dir_entries = fs::read_dir("./img")?.collect::<Result<Vec<_>, _>>()?;
    let process_results = dir_entries.par_iter().map(process_dir_entry);
    let processed_images = process_results.collect::<Result<Vec<_>, _>>()?;
    println!("\r{CLEAR_LINE}Processed {} images", processed_images.len());
    Ok(())
}

fn process_dir_entry(entry: &DirEntry) -> anyhow::Result<DynamicImage> {
    let path = entry.path();
    let image = ImageReader::open(&path)?.with_guessed_format()?.decode()?;
    let (width, height) = image.dimensions();
    let dim = u32::min(width, height);

    let cropped = image.crop_imm((width - dim) / 2, (height - dim) / 2, dim, dim);
    print!("\r{path:?} : cropped to {dim}x{dim}{CLEAR_LINE}");
    stdout().flush()?;

    let scaled = cropped.resize_exact(IMG_SIZE, IMG_SIZE, FilterType::Lanczos3);
    print!("\r{path:?} : scaled to {IMG_SIZE}x{IMG_SIZE}{CLEAR_LINE}");
    stdout().flush()?;

    Ok(scaled)
}
