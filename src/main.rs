use std::{
    env::args,
    fs::{self, DirEntry},
    io::{Write, stdout},
};

use anyhow::Result;
use image::{DynamicImage, GenericImageView, ImageReader, Rgba, imageops::FilterType};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

const CLEAR_LINE: &str = "\x1b[K";
const TILE_SIZE: u32 = 100;

fn main() -> Result<()> {
    let img_dir = args().skip(1).next().unwrap_or(String::from("./img"));
    let dir_entries = fs::read_dir(img_dir)?.collect::<Result<Vec<_>, _>>()?;
    let process_results = dir_entries.par_iter().map(process_dir_entry);
    let processed_images = process_results.collect::<Result<Vec<_>, _>>()?;
    println!("\r{CLEAR_LINE}Processed {} images", processed_images.len());
    build_mosaic(processed_images)
}

fn process_dir_entry(entry: &DirEntry) -> Result<(DynamicImage, Rgba<u8>)> {
    let path = entry.path();
    let image = ImageReader::open(&path)?.with_guessed_format()?.decode()?;
    let (width, height) = image.dimensions();
    let dim = u32::min(width, height);

    let cropped = image.crop_imm((width - dim) / 2, (height - dim) / 2, dim, dim);
    print!("\r{path:?} : cropped to {dim}x{dim}{CLEAR_LINE}");
    stdout().flush()?;

    let scaled = cropped.resize_exact(TILE_SIZE, TILE_SIZE, FilterType::Lanczos3);
    print!("\r{path:?} : scaled to {TILE_SIZE}x{TILE_SIZE}{CLEAR_LINE}");
    stdout().flush()?;

    let single_pixel = scaled.resize_exact(1, 1, FilterType::Lanczos3);
    let (_, _, rgba) = single_pixel.pixels().next().unwrap();
    print!("\r{path:?} : value {rgba:?}{CLEAR_LINE}");
    stdout().flush()?;

    Ok((scaled, rgba))
}

fn build_mosaic(squares: Vec<(DynamicImage, Rgba<u8>)>) -> Result<()> {
    fs::create_dir_all("./output")?;
    for (i, (square, _rgba)) in squares.iter().enumerate() {
        let output_name = format!("./output/img-{i}.png");
        square.save(&output_name)?;
        print!("\rsaved {output_name}{CLEAR_LINE}");
        stdout().flush()?;
    }

    let dim = (squares.len() as f64).sqrt().ceil() as u64;
    println!("\r{CLEAR_LINE}Would try to construct {dim}x{dim} mosaic");
    Ok(())
}
