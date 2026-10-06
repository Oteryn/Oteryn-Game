//! Export the committed proficiency data through the native server importer, without activation.
use oteryn_game_server::content::import_committed_weapon_proficiency_data;
use oteryn_game_server::durability::sqlstate_codes::ToolKind;
use std::io::Write;
use std::process::ExitCode;

fn export() -> Result<(), Box<dyn std::error::Error>> {
    let data = import_committed_weapon_proficiency_data()?;
    serde_json::to_writer_pretty(std::io::stdout().lock(), &data)?;
    std::io::stdout().write_all(b"\n")?;
    Ok(())
}

fn main() -> ExitCode {
    oteryn_game_server::node::serve::run_tool(
        "oteryn-game-import-proficiencies",
        env!("CARGO_PKG_VERSION"),
        ToolKind::ProficiencyImportFailed.code(),
        export,
    )
}
