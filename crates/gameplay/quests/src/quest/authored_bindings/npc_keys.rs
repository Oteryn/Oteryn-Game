//! NPC greeting admission follows the same closed binding registry as the executor.
use super::*;

/// Canonical NPC keys with an admitted authored TALK executor at native boot.
/// Carried-use callbacks and world stages do not admit NPC greetings.
pub fn native_dialogue_npcs() -> Result<Vec<String>, String> {
    let Some(bindings) = embedded::load()? else {
        return Ok(Vec::new());
    };
    bindings.verify_embedded_recipe_evidence()?;
    Ok(bindings
        .reward_callbacks()
        .filter(|callback| callback.carried_item.is_none())
        .map(|callback| callback.npc.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_boot_admits_the_production_talk_npc_without_a_source_program() -> Result<(), String> {
        let keys = native_dialogue_npcs()?;
        assert!(keys.iter().any(|key| key == "oteryn:npc.smaralda"));
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
        let bindings = embedded::load()?.ok_or("production bindings")?;
        let expected = bindings
            .reward_callbacks()
            .filter(|callback| callback.carried_item.is_none())
            .map(|callback| callback.npc.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(keys.into_iter().collect::<BTreeSet<_>>(), expected);
        Ok(())
    }
}
