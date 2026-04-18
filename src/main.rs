use std::{
    env::args,
    f32::consts::PI,
    fs::{self, DirEntry},
    io::{Write, stdout},
};

use anyhow::Result;
use image::{
    DynamicImage, GenericImageView, ImageReader,
    imageops::{FilterType, overlay},
};
use palette::{FromColor, Lab, Srgb};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

const OUTPUT_PATH: &str = "mosaic.png";
const CLEAR_LINE: &str = "\x1b[K";
const TILE_SIZE: u32 = 50;

fn main() -> Result<()> {
    let img_dir = args().nth(1).unwrap_or(String::from("./img"));
    let dir_entries = fs::read_dir(img_dir)?.collect::<Result<Vec<_>, _>>()?;
    let process_results = dir_entries.par_iter().map(process_dir_entry);
    let processed_images = process_results.collect::<Result<Vec<_>, _>>()?;
    println!("\r{CLEAR_LINE}Processed {} images", processed_images.len());
    build_mosaic(processed_images)
}

struct Tile {
    scaled: DynamicImage,
    light: f32,
    hue: f32,
}

fn process_dir_entry(entry: &DirEntry) -> Result<Tile> {
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
    let lab = Lab::from_color(
        Srgb::new(
            rgba[0] as f32 / 255.0,
            rgba[1] as f32 / 255.0,
            rgba[2] as f32 / 255.0,
        )
        .into_linear(),
    );

    let light = lab.l / 100.0;
    let hue_radians = lab.b.atan2(lab.a);
    let hue = (hue_radians + PI) / (2.0 * PI);
    print!("\r{path:?} : hue:{hue} light:{light}{CLEAR_LINE}");
    stdout().flush()?;

    Ok(Tile { scaled, light, hue })
}

#[derive(Clone, Copy)]
struct Cell {
    x: u32,
    y: u32,
    hue: f32,
    light: f32,
}

fn build_mosaic(mut tiles: Vec<Tile>) -> Result<()> {
    tiles.sort_by(|a, b| a.hue.partial_cmp(&b.hue).unwrap());

    let count = tiles.len() as u32;
    let width_tiles = (count as f32).sqrt().ceil() as u32;
    let height_tiles = width_tiles;

    let width_px = width_tiles * TILE_SIZE;
    let height_px = height_tiles * TILE_SIZE;
    let mut canvas = image::RgbaImage::new(width_px, height_px);

    let cells: Vec<Cell> = (0..width_tiles)
        .flat_map(|x| (0..height_tiles).map(move |y| (x, y)))
        .map(|(x, y)| {
            let hue = x as f32 / (width_tiles - 1) as f32;
            let light = y as f32 / (height_tiles - 1) as f32;
            Cell { x, y, hue, light }
        })
        .collect();

    let mut used = vec![false; cells.len()];
    for tile in tiles {
        insert_tile(&mut canvas, &mut used, &cells, tile);
    }

    canvas.save(OUTPUT_PATH)?;
    println!("Saved {OUTPUT_PATH}");

    Ok(())
}

fn insert_tile(
    canvas: &mut image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    used: &mut Vec<bool>,
    cells: &[Cell],
    tile: Tile,
) {
    let mut best_i = None;
    let mut best_d = f32::MAX;
    for (i, cell) in cells.iter().enumerate() {
        if (*used)[i] {
            continue;
        }
        let d = dist(tile.hue, tile.light, cell.hue, cell.light);
        if d < best_d {
            best_d = d;
            best_i = Some(i);
        }
    }

    let i = best_i.unwrap();
    (*used)[i] = true;

    let cell = cells[i];
    let px = cell.x * TILE_SIZE;
    let py = cell.y * TILE_SIZE;
    overlay(canvas, &tile.scaled, px.into(), py.into());
}

fn dist(a_hue: f32, a_light: f32, b_hue: f32, b_light: f32) -> f32 {
    let dist_hue = (a_hue - b_hue).abs();
    let dist_hue = dist_hue.min(1.0 - dist_hue);
    let dist_light = a_light - b_light;
    dist_hue * dist_hue + dist_light * dist_light * 8.0
}
