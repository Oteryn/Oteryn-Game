use oteryn_synthetic_assets::SyntheticImage;
use serde_json::json;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.len() != 4 {
        return Err("usage: oteryn-synthetic-asset-compiler <manifest> <rgba> <output>".into());
    }
    let manifest_path = PathBuf::from(&arguments[1]);
    let rgba_path = PathBuf::from(&arguments[2]);
    let output_path = PathBuf::from(&arguments[3]);
    let manifest = fs::read(manifest_path)?;
    let rgba = fs::read(rgba_path)?;
    let image = SyntheticImage::from_manifest_and_rgba(&manifest, rgba)?;
    let output = json!({
        "schema": "synthetic-compiled-v1",
        "asset_id": image.asset_id.as_str(),
        "width": image.width,
        "height": image.height,
        "rgba_sha256": image.sha256_hex(),
    });
    fs::write(output_path, serde_json::to_vec_pretty(&output)?)?;
    Ok(())
}
