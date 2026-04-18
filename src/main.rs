use std::{
    env::args,
    fs::{self, DirEntry},
    io::{Write, stdout},
};

use anyhow::Result;
use image::{DynamicImage, GenericImageView, ImageReader, Rgba, imageops::FilterType};
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
    let count = squares.len() as u32;
    let width_tiles = (count as f32).sqrt().ceil() as u32;
    let height_tiles = width_tiles;

    let width_px = width_tiles * TILE_SIZE;
    let height_px = height_tiles * TILE_SIZE;
    let mut canvas = image::RgbaImage::new(width_px, height_px);
    let mut occupied = vec![false; (width_tiles * height_tiles) as usize];

    for (img, rgba) in squares {
        let (h, _s, l) = rgb_to_hsl(rgba[0], rgba[1], rgba[2]);
        let x = ((h / 360.0) * (width_tiles as f32 - 1.0)).round() as i32;
        let y = (l * (height_tiles as f32 - 1.0)).round() as i32;

        if let Some((fx, fy)) =
            find_free_spot(x, y, width_tiles as i32, height_tiles as i32, &mut occupied)
        {
            let idx = (fy as u32 * width_tiles + fx as u32) as usize;
            occupied[idx] = true;

            let px = fx as u32 * TILE_SIZE;
            let py = fy as u32 * TILE_SIZE;

            image::imageops::overlay(&mut canvas, &img, px.into(), py.into());
        }
    }

    canvas.save(OUTPUT_PATH)?;
    println!("Saved {OUTPUT_PATH}");

    Ok(())
}

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let max = r.max(g.max(b));
    let min = r.min(g.min(b));
    let delta = max - min;

    let lightness = (max + min) / 2.0;

    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / delta) % 6.0
    } else if max == g {
        ((b - r) / delta) + 2.0
    } else {
        ((r - g) / delta) + 4.0
    } * 60.0;

    let hue = if hue < 0.0 { hue + 360.0 } else { hue };

    let saturation = if delta == 0.0 {
        0.0
    } else {
        delta / (1.0 - (2.0 * lightness - 1.0).abs())
    };

    (hue, saturation, lightness)
}

fn find_free_spot(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    occupied: &mut [bool],
) -> Option<(i32, i32)> {
    let idx = |x: i32, y: i32| (y * width + x) as usize;

    if x >= 0 && x < width && y >= 0 && y < height && !occupied[idx(x, y)] {
        return Some((x, y));
    }

    for radius in 1..50 {
        for dx in -radius..=radius {
            for dy in -radius..=radius {
                let nx = x + dx;
                let ny = y + dy;

                if nx >= 0 && nx < width && ny >= 0 && ny < height {
                    let i = idx(nx, ny);
                    if !occupied[i] {
                        return Some((nx, ny));
                    }
                }
            }
        }
    }

    None
}
