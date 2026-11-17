//! Canonical achievement identity and the existing account grant owner, in the NPC transaction.
use super::*;
use oteryn_gameplay_progression::achievement_catalogue::{Entry, SHARDS, parse_shards};
use oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect;
use std::collections::BTreeSet;

const MAX_ACHIEVEMENTS: usize = 8;

pub(super) fn keys(effects: &[NpcRuleEffect]) -> StoredResult<Vec<String>> {
    if !effects
        .iter()
        .any(|effect| matches!(effect, NpcRuleEffect::Achievement { .. }))
    {
        return Ok(Vec::new());
    }
    static CATALOGUE: std::sync::LazyLock<Option<BTreeMap<String, Entry>>> =
        std::sync::LazyLock::new(|| parse_shards(&SHARDS).ok());
    let catalogue = CATALOGUE
        .as_ref()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let mut keys = BTreeSet::new();
    for effect in effects {
        let NpcRuleEffect::Achievement { name } = effect else {
            continue;
        };
        // Source Lua provides a display name, not authority to invent a slug/key.
        let mut matches = catalogue
            .iter()
            .filter(|(_, entry)| entry.name == *name && !entry.retired);
        let (key, _) = matches.next().ok_or(DurabilityError::InvalidStoredState)?;
        if matches.next().is_some() {
            return Err(DurabilityError::InvalidStoredState);
        }
        keys.insert(key.clone());
        if keys.len() > MAX_ACHIEVEMENTS {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    Ok(keys.into_iter().collect())
}

pub(super) async fn grant(
    tx: &mut super::super::super::item_mint_audit::Type2Transaction<'_>,
    character: CharacterId,
    request: &NpcProgramRequest,
    keys: &[String],
) -> StoredResult<()> {
    let keys = keys.iter().map(String::as_str).collect::<Vec<_>>();
    super::super::super::account_achievement::grant_missing(
        tx,
        character,
        &keys,
        "oteryn:quest-npc",
        &request.binding(character),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    fn achievement(name: &str) -> NpcRuleEffect {
        NpcRuleEffect::Achievement { name: name.into() }
    }
    #[test]
    fn m41_resolves_exact_active_catalogue_names_and_coalesces_duplicates() -> StoredResult<()> {
        assert_eq!(
            keys(&[
                achievement("Bastard"),
                achievement("Sculptor Apprentice"),
                achievement("Bastard")
            ])?,
            vec![
                "oteryn:achievement/bastard",
                "oteryn:achievement/sculptor_apprentice"
            ]
        );
        assert!(keys(&[])?.is_empty());
        Ok(())
    }
    #[test]
    fn m41_unknown_names_and_key_or_case_substitutions_refuse() {
        for name in [
            "bastard",
            "oteryn:achievement/bastard",
            "M41 invented achievement",
        ] {
            assert!(keys(&[achievement(name)]).is_err(), "{name}");
        }
    }
}
