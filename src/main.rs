use std::{
    fs,
    io::{Write, stdout},
};

use image::ImageReader;

fn main() -> anyhow::Result<()> {
    for p in fs::read_dir("./img")? {
        let p = p?.path();
        let image = ImageReader::open(&p)?.with_guessed_format()?.decode()?;
        let dimension = image.width().max(image.height());
        print!("\r{p:?} : {dimension}\x1B[K");
        stdout().flush()?;
    }
    println!();
    Ok(())
}
