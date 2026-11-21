//! Exact source contract for the stateless Arbaziloth party entry.
use super::{Callback, Document, EncounterPhase};

pub(super) const KEY: &str = "oteryn:quest-callback/nrwicked/arbaziloth";
pub(super) const DOCUMENT: &str =
    include_str!("../../../../../../content/quests/world-callbacks/nrwicked-arbaziloth.json");

pub(super) fn exact(candidate: &Callback) -> bool {
    if candidate.key != KEY
        || candidate
            .effects
            .encounter
            .as_ref()
            .and_then(|encounter| encounter.phase)
            != Some(EncounterPhase::Arbaziloth)
    {
        return false;
    }
    // Decode the typed witness directly because the public parser uses this predicate.
    let Ok(Document {
        schema: 1,
        callbacks,
    }) = serde_json::from_str(DOCUMENT)
    else {
        return false;
    };
    callbacks.as_slice() == std::slice::from_ref(candidate)
}

#[cfg(test)]
mod tests {
    use super::super::{load, parse, validate_callbacks};
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn arbaziloth_source_party_is_admitted_only_with_exact_callback() -> Result<(), String> {
        let callbacks = load()?;
        let callback = callbacks
            .iter()
            .find(|callback| callback.key == KEY)
            .ok_or("Arbaziloth")?;
        assert!(exact(callback));
        assert!(callback.lever.is_none());
        assert!(!callback.has_personal_state());
        assert!(!callback.requires_world_admission);
        let encounter = callback.effects.encounter.as_ref().ok_or("encounter")?;
        assert_eq!(encounter.minimum_level, 0);
        assert_eq!(encounter.players.len(), 5);
        assert!(
            encounter
                .players
                .iter()
                .all(|player| player.to == encounter.players[0].to)
        );

        let document: Value = serde_json::from_str(DOCUMENT).map_err(|error| error.to_string())?;
        for (path, replacement) in [
            (
                "/key",
                json!("oteryn:quest-callback/nrwicked/another-entry"),
            ),
            ("/quest", json!("oteryn:quest.the_ancient_tombs_quest")),
            (
                "/sources/1",
                json!("docs/reference/content-prep/another-source.md"),
            ),
            ("/selector/ordinal", json!(1)),
            ("/effects/encounter/room_max/x", json!(34044)),
            ("/effects/encounter/players/0/to/x", json!(34035)),
            ("/effects/encounter/monsters/0/position/y", json!(32331)),
        ] {
            let mut edited = document.clone();
            *edited
                .pointer_mut(&format!("/callbacks/0{path}"))
                .ok_or(path)? = replacement;
            let key = edited["callbacks"][0]["key"]
                .as_str()
                .ok_or("callback key")?;
            assert_eq!(
                parse(&edited.to_string()),
                Err(format!("invalid callback {key}")),
                "{path}"
            );
        }
        Ok(())
    }

    #[test]
    fn arbaziloth_phase_refuses_generic_lever_and_personal_admission() -> Result<(), String> {
        let mut lever = parse(DOCUMENT)?.pop().ok_or("Arbaziloth")?;
        lever.key = "oteryn:quest-callback/nrwicked/generic-lever".into();
        let mut personal = lever.clone();
        lever.lever = Some([8911, 8912]);
        personal.key = "oteryn:quest-callback/nrwicked/generic-personal".into();
        personal.requires_world_admission = true;
        personal.storage.push(
            serde_json::from_value(json!({
                "track": "oteryn:quest-progress/nrwicked/generic-personal-test",
                "minimum": 0, "maximum": 0, "set": 1
            }))
            .map_err(|error| error.to_string())?,
        );
        personal
            .effects
            .encounter
            .as_mut()
            .ok_or("encounter")?
            .players
            .truncate(1);

        for mut callback in [lever, personal] {
            assert!(
                validate_callbacks(std::slice::from_ref(&callback)).is_err(),
                "{}",
                callback.key
            );
            callback
                .effects
                .encounter
                .as_mut()
                .ok_or("encounter")?
                .phase = Some(EncounterPhase::Cults);
            validate_callbacks(std::slice::from_ref(&callback))?;
            callback.key = KEY.into();
            assert!(validate_callbacks(&[callback]).is_err());
        }
        Ok(())
    }
}
