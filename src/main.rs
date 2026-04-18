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
    tiles.sort_by(|a, b| a.light.partial_cmp(&b.light).unwrap());
    let (width_tiles, height_tiles) = find_grid(tiles.len() as u32);

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

    let assignment = auction_assign(&tiles, &cells);

    for (tile, &cell_idx) in tiles.iter().zip(assignment.iter()) {
        let cell = cells[cell_idx];

        let px = cell.x * TILE_SIZE;
        let py = cell.y * TILE_SIZE;

        overlay(&mut canvas, &tile.scaled, px.into(), py.into());
    }

    canvas.save(OUTPUT_PATH)?;
    println!("Saved {OUTPUT_PATH}");

    Ok(())
}

fn find_grid(n: u32) -> (u32, u32) {
    let mut best_w = n;
    let mut best_h = 1;
    let mut best_score = f32::INFINITY;
    let sqrt = (n as f32).sqrt() as u32;

    for w in 1..=sqrt * 2 {
        let h = (n + w - 1) / w;
        let area = w * h;
        let waste = (area - n) as f32;

        let aspect = w as f32 / h as f32;
        let aspect_penalty = (aspect.ln()).abs(); // prefers ~1.0
        let score = waste * 2.0 + aspect_penalty * 10.0;

        if score < best_score {
            best_score = score;
            best_w = w;
            best_h = h;
        }
    }

    (best_w, best_h)
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

    assignment.into_iter().map(|x| x.unwrap()).collect()
}

fn dist(a_hue: f32, a_light: f32, b_hue: f32, b_light: f32) -> f32 {
    let dh = (a_hue - b_hue).abs().min(1.0 - (a_hue - b_hue).abs());
    let dl = a_light - b_light;
    dl * dl * 6.0 + dh * dh
}
