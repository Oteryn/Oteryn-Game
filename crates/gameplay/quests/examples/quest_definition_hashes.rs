//! Print embedded quest definition hashes for repository baseline regeneration.
use std::collections::BTreeMap;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    let mut hashes = BTreeMap::new();
    for key in std::env::args().skip(1) {
        let hash = lowered
            .catalogue()
            .definition_hash(&key)
            .ok_or_else(|| std::io::Error::other(format!("unknown quest: {key}")))?;
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
