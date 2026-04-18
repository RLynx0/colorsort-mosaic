use std::{
    env::args,
    f32::consts::PI,
    fs::{self, DirEntry},
    io::{Write, stdout},
};

use anyhow::Result;
use image::{DynamicImage, GenericImageView, ImageReader, Rgba, imageops::FilterType};
use palette::{FromColor, Lab, Srgb};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

const OUTPUT_PATH: &str = "mosaic.png";
const CLEAR_LINE: &str = "\x1b[K";
const TILE_SIZE: u32 = 50;

fn main() -> Result<()> {
    let img_dir = args().skip(1).next().unwrap_or(String::from("./img"));
    let dir_entries = fs::read_dir(img_dir)?.collect::<Result<Vec<_>, _>>()?;
    let process_results = dir_entries.par_iter().map(process_dir_entry);
    let processed_images = process_results.collect::<Result<Vec<_>, _>>()?;
    println!("\r{CLEAR_LINE}Processed {} images", processed_images.len());
    build_mosaic(processed_images)
}

fn process_dir_entry(entry: &DirEntry) -> Result<(DynamicImage, Lab)> {
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

    let lab = Lab::from_color(Srgb::new(
        rgba[0] as f32 / 255.0,
        rgba[1] as f32 / 255.0,
        rgba[2] as f32 / 255.0,
    ));

    Ok((scaled, lab))
}

fn build_mosaic(squares: Vec<(DynamicImage, Lab)>) -> Result<()> {
    let count = squares.len() as u32;
    let width_tiles = (count as f32).sqrt().ceil() as u32;
    let height_tiles = width_tiles;

    let width_px = width_tiles * TILE_SIZE;
    let height_px = height_tiles * TILE_SIZE;
    let canvas = image::RgbaImage::new(width_px, height_px);

    canvas.save(OUTPUT_PATH)?;
    println!("Saved {OUTPUT_PATH}");

    Ok(())
}

fn lab_to_angle_distance(lab: Lab) -> (f32, f32) {
    let hue = lab.b.atan2(lab.a); // -pi..pi
    let hue_norm = (hue + PI) / (2.0 * PI);
    let light = lab.l / 100.0;
    (hue_norm, light)
}
