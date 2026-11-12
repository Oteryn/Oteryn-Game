mod build_authored_bindings;
mod build_batch3_a;
mod build_npc_programs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    build_authored_bindings::generate(&root, &out)?;
    build_npc_programs::generate(&root, &out)?;
    build_batch3_a::generate(&root, &out)?;
    Ok(())
}
