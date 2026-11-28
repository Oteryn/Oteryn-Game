use super::{Error, replay};

pub(super) fn run() -> Result<(), Error> {
    std::thread::Builder::new()
        .name("n07d-minimap".into())
        .stack_size(64 << 20)
        .spawn(|| replay().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "N07d minimap test thread panicked")??;
    Ok(())
}
