//! Emit the actual runtime NPC programme inventory for exact fixture refreshes.
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let source = oteryn_gameplay_quests::quest::npc_rules::NpcRuleCatalog::runtime()?;
    let rows: Vec<_> = source
        .programs()
        .iter()
        .map(|program| {
            serde_json::json!({
                "schema": program.document.schema,
                "quest": program.document.quest,
                "program_key": program.document.program_key,
            })
        })
        .collect();
    println!("{}", serde_json::to_string(&rows)?);
    Ok(())
}
