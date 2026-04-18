use std::{
    fs::{self, DirEntry},
    io::{Write, stdout},
};

use image::{DynamicImage, ImageReader};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

fn main() -> anyhow::Result<()> {
    let cropped_images = fs::read_dir("./img")?
        .collect::<Result<Vec<_>, _>>()?
        .par_iter()
        .map(process_entry)
        .collect::<Result<Vec<_>, _>>()?;

    println!("\r\x1b[KGot {} images", cropped_images.len());
    Ok(())
}

fn process_entry(entry: &DirEntry) -> anyhow::Result<DynamicImage> {
    let path = entry.path();
    let image = ImageReader::open(&path)?.with_guessed_format()?.decode()?;
    let width = image.width();
    let height = image.height();
    let dim = width.max(height);

    print!("\r{path:?} : {dim}\x1b[K");
    stdout().flush()?;

    let cropped = image.crop_imm((width - dim) / 2, (height - dim) / 2, dim, dim);

    Ok(cropped)
}
