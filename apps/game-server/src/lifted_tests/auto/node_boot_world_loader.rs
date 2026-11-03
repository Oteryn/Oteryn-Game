#[test]
fn node_boot_qualification_world_loader() -> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| crate::qualification::client_world::smoke().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "world loader thread failed")??;
    Ok(())
}
