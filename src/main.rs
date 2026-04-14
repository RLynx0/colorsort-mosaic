use std::fs;

use image::ImageReader;

fn main() -> anyhow::Result<()> {
    for p in fs::read_dir("./img")? {
        let p = p?.path();
        let image = ImageReader::open(&p)?.with_guessed_format()?.decode()?;
        let dimension = image.width().max(image.height());
        println!("{p:?} : {dimension}");
    }
    Ok(())
}
