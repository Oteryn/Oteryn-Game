//! Exact owner-authored Asura callbacks; no donor implementation is claimed.
use super::Callback;

pub(super) fn admitted(callback: &Callback) -> bool {
    if callback.quest != "oteryn:quest.authored.asura_palace_quest"
        || !callback
            .key
            .starts_with("oteryn:quest-callback/engenc/asura/")
        || !callback
            .sources
            .iter()
            .any(|s| s == "authored per owner rule")
    {
        return false;
    }
    let Ok(document) = serde_json::from_str::<serde_json::Value>(include_str!(
        "../../../../../../content/quests/authored-bindings/engenc-asura-palace.json"
    )) else {
        return false;
    };
    document["quests"][0]["stages"]
        .as_array()
        .is_some_and(|stages| {
            stages.iter().any(|stage| {
                serde_json::from_value::<Callback>(stage["trigger"]["callback"].clone())
                    .is_ok_and(|bound| bound == *callback)
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orchid_native_loader_keeps_gate_reward_and_completion() -> Result<(), String> {
        use crate::quest::{authored_bindings::Bindings, loader, world_callbacks};

        let bindings = Bindings::parse(include_bytes!(
            "../../../../../../content/quests/authored-bindings/engenc-asura-palace.json"
        ))?;
        let loaded = loader::load_embedded_quest_state_with_bindings("content-1", &bindings)
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let callbacks = world_callbacks::load_for(catalogue)?;
        let find = |part: &str| {
            let key = format!("oteryn:quest-callback/engenc/asura/orchid/{part}");
            callbacks
                .iter()
                .find(|callback| callback.key == key)
                .ok_or(key)
        };
        let entry = find("entry")?;
        assert_eq!(entry.trigger, world_callbacks::Trigger::Use);
        assert_eq!(
            entry
                .selector
                .native
                .as_ref()
                .ok_or("native door")?
                .position
                .x,
            32959
        );
        let encounter = entry.effects.encounter.as_ref().ok_or("Orchid encounter")?;
        assert_eq!(encounter.monsters.len(), 1);
        assert_eq!(
            encounter.monsters[0].creature,
            "oteryn:creature.the_flaming_orchid"
        );
        let death = catalogue
            .transition("oteryn:quest-transition/native-kills/engenc-asura-orchid")
            .ok_or("Orchid death")?;
        assert!(!death.completes);
        let ruby = find("ruby")?;
        assert!(
            catalogue
                .transition(&ruby.key)
                .ok_or("ruby claim")?
                .completes
        );
        assert_eq!(ruby.item_reward().as_ref().ok_or("ruby reward")?.count, 3);
        assert_eq!(
            find("sapphire")?
                .item_reward()
                .as_ref()
                .ok_or("sapphire reward")?
                .count,
            3
        );
        let door = find("library-door")?;
        assert_eq!(door.minimum_level, Some(250));
        assert_eq!(door.effects.transform, Some(1677));
        Ok(())
    }

    #[test]
    fn engenc_authored_admission_is_exact_and_cannot_rebind_native_doors() -> Result<(), String> {
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../content/quests/authored-bindings/engenc-asura-palace.json"
        ))
        .map_err(|e| e.to_string())?;
        let mut count = 0;
        for stage in document["quests"][0]["stages"].as_array().ok_or("stages")? {
            let Ok(mut callback) =
                serde_json::from_value::<Callback>(stage["trigger"]["callback"].clone())
            else {
                continue;
            };
            if !callback
                .key
                .starts_with("oteryn:quest-callback/engenc/asura/")
            {
                continue;
            }
            assert!(admitted(&callback));
            count += 1;
            if let Some(position) = callback.selector.position.as_mut() {
                position.x += 1;
            } else {
                callback
                    .selector
                    .native
                    .as_mut()
                    .ok_or("native position")?
                    .position
                    .x += 1;
            }
            assert!(!admitted(&callback));
        }
        assert_eq!(count, 10);
        Ok(())
    }
}
