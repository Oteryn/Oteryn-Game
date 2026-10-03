//! Export the committed proficiency data through the native server importer, without activation.
use oteryn_game_server::content::import_committed_weapon_proficiency_data;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = import_committed_weapon_proficiency_data()?;
    serde_json::to_writer_pretty(std::io::stdout().lock(), &data)?;
    std::io::stdout().write_all(b"\n")?;
    Ok(())
}
