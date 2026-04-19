use std::{
    cmp::Ordering,
    f32::consts::PI,
    fs::{DirEntry, read_dir},
    io::{Write, stdout},
    path::PathBuf,
};

use anyhow::{Result, anyhow};
use clap::Parser;
use image::{
    DynamicImage, GenericImageView, ImageReader, RgbaImage,
    imageops::{FilterType, overlay},
};
use palette::{FromColor, Lab, Srgb};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

const CLEAR_LINE: &str = "\x1b[K";

#[derive(clap::Parser)]
struct Cli {
    /// Path to the input image directory
    img_dir: Vec<PathBuf>,
    /// Path of the output file
    #[arg(short, long, default_value = "./mosaic.png")]
    output: PathBuf,
    /// Size of each mosaic tile in pixels
    #[arg(short, long, default_value_t = 50)]
    size: u32,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let dir_results = cli.img_dir.iter().map(read_dir);
    let dirs = dir_results.collect::<Result<Vec<_>, _>>()?;
    let dir_entries = dirs.into_iter().flatten().collect::<Result<Vec<_>, _>>()?;
    let images = dir_entries.par_iter().map(image_from_dir_entry).flatten();
    let process_results = images.map(|(i, p)| process_image(i, &p, cli.size));
    let processed_tiles = process_results.collect::<Result<Vec<_>, _>>()?;
    if processed_tiles.is_empty() {
        return Err(anyhow!("Exiting because no images where found"));
    }

    println!("\r{CLEAR_LINE}Processed {} images", processed_tiles.len());

    build_mosaic(processed_tiles, cli.size)?.save(&cli.output)?;
    println!("Saved {:?}", cli.output);
    Ok(())
}

fn image_from_dir_entry(entry: &DirEntry) -> Result<(DynamicImage, PathBuf)> {
    let path = entry.path();
    let image = ImageReader::open(&path)?.with_guessed_format()?.decode()?;
    Ok((image, path))
}

struct Tile {
    scaled: DynamicImage,
    light: f32,
    hue: f32,
}

fn process_image(image: DynamicImage, path: &PathBuf, tile_size: u32) -> Result<Tile> {
    let (width, height) = image.dimensions();
    let dim = u32::min(width, height);

    let cropped = image.crop_imm((width - dim) / 2, (height - dim) / 2, dim, dim);
    print!("\r{path:?} : cropped to {dim}x{dim}{CLEAR_LINE}");
    stdout().flush()?;

    let scaled = cropped.resize_exact(tile_size, tile_size, FilterType::Lanczos3);
    print!("\r{path:?} : scaled to {tile_size}x{tile_size}{CLEAR_LINE}");
    stdout().flush()?;

    let single_pixel = scaled.resize_exact(1, 1, FilterType::Lanczos3);
    let (_, _, rgba) = single_pixel.pixels().next().ok_or(anyhow!("0px"))?;
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

fn build_mosaic(mut tiles: Vec<Tile>, tile_size: u32) -> Result<RgbaImage> {
    tiles.sort_by(|a, b| a.light.partial_cmp(&b.light).unwrap_or(Ordering::Equal));
    let (width_tiles, height_tiles) = find_grid(tiles.len() as u32);
    let width_px = width_tiles * tile_size;
    let height_px = height_tiles * tile_size;
    println!("Creating mosaic with {width_tiles}x{height_tiles} tiles");
    println!("Total size: {width_px}x{height_px} px");

    let mut canvas = RgbaImage::new(width_px, height_px);
    let cells: Vec<Cell> = (0..width_tiles)
        .flat_map(|x| (0..height_tiles).map(move |y| (x, y)))
        .map(|(x, y)| {
            let hue = x as f32 / (width_tiles - 1) as f32;
            let light = y as f32 / (height_tiles - 1) as f32;
            Cell { x, y, hue, light }
        })
        .collect();

    let assignment = auction_assign(&tiles, &cells);
    for (tile, &cell_idx) in tiles.iter().zip(assignment.iter()) {
        let cell = cells[cell_idx];

        let px = cell.x * tile_size;
        let py = cell.y * tile_size;

        overlay(&mut canvas, &tile.scaled, px.into(), py.into());
    }

    Ok(canvas)
}

fn find_grid(n: u32) -> (u32, u32) {
    let mut best_a = n;
    let mut best_b = 1;
    let mut best_score = f32::INFINITY;
    let sqrt = (n as f32).sqrt() as u32;

    for a in 1..=sqrt * 2 {
        let b = n.div_ceil(a);
        let area = a * b;
        let waste = (area - n) as f32;

        let aspect = a as f32 / b as f32;
        let aspect_penalty = (aspect.ln()).abs(); // prefers ~1.0
        let score = waste * 2.0 + aspect_penalty * 10.0;

        if score < best_score {
            best_score = score;
            best_a = a;
            best_b = b;
        }
    }

    let width = u32::max(best_a, best_b);
    let height = u32::min(best_a, best_b);
    (width, height)
}

fn auction_assign(tiles: &[Tile], cells: &[Cell]) -> Vec<usize> {
    let n = tiles.len();
    let mut prices = vec![0.0f32; cells.len()];
    let mut assignment = vec![None; n];
    let mut cell_owner = vec![None; cells.len()];

    let epsilon = 0.005;
    let mut unassigned: Vec<usize> = (0..n).collect();

    while let Some(i) = unassigned.pop() {
        let tile = &tiles[i];
        let mut best_j = 0;
        let mut best_val = f32::NEG_INFINITY;
        let mut second_val = f32::NEG_INFINITY;

        for (j, cell) in cells.iter().enumerate() {
            let d = dist(tile.hue, tile.light, cell.hue, cell.light);
            let val = -d - prices[j];
            if val > best_val {
                second_val = best_val;
                best_val = val;
                best_j = j;
            } else if val > second_val {
                second_val = val;
            }
        }

        let bid = best_val - second_val + epsilon;
        prices[best_j] += bid;

        if let Some(prev_tile) = cell_owner[best_j] {
            assignment[prev_tile] = None;
            unassigned.push(prev_tile);
        }

        assignment[i] = Some(best_j);
        cell_owner[best_j] = Some(i);
    }

    assignment.into_iter().flatten().collect()
}

fn dist(a_hue: f32, a_light: f32, b_hue: f32, b_light: f32) -> f32 {
    let dh = (a_hue - b_hue).abs().min(1.0 - (a_hue - b_hue).abs());
    let dl = a_light - b_light;
    dl * dl + dh * dh
}
