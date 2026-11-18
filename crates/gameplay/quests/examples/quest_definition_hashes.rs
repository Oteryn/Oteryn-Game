//! Print embedded quest definition hashes for repository baseline regeneration.
use oteryn_gameplay_quests::quest::npc_rules::legacy_definition_hash;
use std::collections::BTreeMap;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    let mut hashes = BTreeMap::new();
    let mut args = std::env::args().skip(1).peekable();
    let predecessor = args.next_if(|arg| arg == "--npc-predecessor").is_some();
    for key in args {
        let catalogue = lowered.catalogue();
        let hash = if predecessor {
            legacy_definition_hash(catalogue, &key)
        } else {
            catalogue.definition_hash(&key)
        }
        .ok_or_else(|| std::io::Error::other(format!("no requested definition for {key}")))?;
        hashes.insert(
            key,
            hash.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
        );
    }
    if hashes.is_empty() {
        return Err(std::io::Error::other("supply at least one quest key").into());
    }
    println!("{}", serde_json::to_string(&hashes)?);
    Ok(())
}
