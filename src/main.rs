use std::{
    fs::{self, DirEntry},
    io::{Write, stdout},
    path::PathBuf,
};

use image::ImageReader;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

fn main() -> anyhow::Result<()> {
    println!(
        "\r\x1b[K{:#?}",
        fs::read_dir("./img")?
            .collect::<Result<Vec<_>, _>>()?
            .par_iter()
            .map(process_entry)
            .collect::<Result<Vec<_>, _>>()?
    );
    Ok(())
}

fn process_entry(entry: &DirEntry) -> anyhow::Result<(PathBuf, u32)> {
    let path = entry.path();
    let image = ImageReader::open(&path)?.with_guessed_format()?.decode()?;
    let dim = image.width().max(image.height());
    print!("\r{path:?} : {dim}\x1b[K");
    stdout().flush()?;
    Ok((path, dim))
}
