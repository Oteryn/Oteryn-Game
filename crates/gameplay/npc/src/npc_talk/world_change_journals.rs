//! Presentation membership only: every quest value effect stays with the fenced quest owner.
use std::collections::BTreeSet;

pub(super) const SOURCE: &str =
    include_str!("../../../../../content/quests/world-change-journals.json");

#[derive(serde::Deserialize)]
struct Document {
    schema: u32,
    programs: Vec<Programme>,
}
#[derive(serde::Deserialize)]
struct Programme {
    key: String,
    quest: String,
    mode: String,
    repeat: String,
    runtime_enabled: bool,
    npc_callbacks: Vec<Callback>,
}
#[derive(serde::Deserialize)]
struct Callback {
    key: String,
    quest: String,
    npc: String,
}

fn slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

pub(super) fn npc_keys(source: &str) -> Option<BTreeSet<String>> {
    if source.len() > 2 * 1024 * 1024 {
        return None;
    }
    let document: Document = serde_json::from_str(source).ok()?;
    if document.schema != 1 || document.programs.len() > 16 {
        return None;
    }
    let mut npcs = BTreeSet::new();
    let mut callbacks = BTreeSet::new();
    for programme in document.programs {
        if programme.npc_callbacks.len() > 16 {
            return None;
        }
        if !programme.runtime_enabled {
            continue;
        }
        if !slug(&programme.key)
            || programme.mode != "personal_on_demand"
            || programme.repeat != "once"
            || !programme.quest.starts_with("oteryn:quest.")
            || programme.quest.len() > 256
            || programme
                .quest
                .bytes()
                .any(|b| !b.is_ascii_alphanumeric() && !b"._:-".contains(&b))
        {
            return None;
        }
        let prefix = format!("oteryn:quest-reward/worldchg-m31/{}/", programme.key);
        for callback in programme.npc_callbacks {
            if callback.quest != programme.quest
                || !callback.key.starts_with(&prefix)
                || callback.key.len() > 256
                || callback.key.len() == prefix.len()
                || !callback.npc.strip_prefix("oteryn:npc.").is_some_and(slug)
                || !callbacks.insert(callback.key)
            {
                return None;
            }
            npcs.insert(callback.npc);
        }
    }
    Some(npcs)
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn worldchg_enabled_journal_npc_opens_focus_without_a_standalone_script() -> Result<(), String>
    {
        let catalogue = crate::content::NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .map_err(|e| format!("{e:?}"))?;
        let model = catalogue
            .service_model_for_project()
            .map_err(|e| format!("{e:?}"))?;
        let mut source: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/world-change-journals.json"
        ))
        .map_err(|e| e.to_string())?;
        for p in source["programs"].as_array_mut().ok_or("programmes")? {
            p["runtime_enabled"] = (p["key"] == "lumberjack").into();
        }
        let book = NpcTalkBook::from_model_with_journal_source(&model, &[], &source.to_string());
        let chip = book
            .get("oteryn:npc.chip")
            .ok_or("missing enabled journal NPC focus")?;
        assert!(!chip.greeting_reply.is_empty());
        assert!(!chip.admitted_dialogue);
        assert!(chip.offers().is_empty());
        assert!(!chip.bank());
        assert!(chip.travel_routes.is_empty());
        assert!(chip.premium_services.is_empty());
        let at = ChatPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        let npcs = [TalkNpc {
            id: 1u8,
            position: at,
            entry: chip,
        }];
        let mut focus = None;
        let reply = talk(at, "hi", &npcs, &mut focus).ok_or("missing native greeting")?;
        assert_eq!(focus, Some(1));
        assert_eq!(
            chip.render_source_text(&reply.text, "Helper"),
            "Greetings, Helper."
        );
        for p in source["programs"].as_array_mut().ok_or("programmes")? {
            p["runtime_enabled"] = false.into();
        }
        let disabled =
            NpcTalkBook::from_model_with_journal_source(&model, &[], &source.to_string());
        assert!(disabled.get("oteryn:npc.chip").is_none());
        assert_ne!(book.catalogue_revision, disabled.catalogue_revision);
        Ok(())
    }
    #[test]
    fn worldchg_journal_npc_membership_rejects_foreign_or_unbounded_programmes()
    -> Result<(), String> {
        for case in 0..10 {
            let mut source: serde_json::Value =
                serde_json::from_str(super::SOURCE).map_err(|e| e.to_string())?;
            for p in source["programs"].as_array_mut().ok_or("programmes")? {
                p["runtime_enabled"] = (p["key"] == "lumberjack").into();
            }
            if case == 0 {
                source["schema"] = 2.into();
            } else if case == 1 {
                let row = source["programs"][0].clone();
                source["programs"] = serde_json::json!(vec![row; 17]);
            } else {
                let p = source["programs"]
                    .as_array_mut()
                    .ok_or("programmes")?
                    .iter_mut()
                    .find(|p| p["key"] == "lumberjack")
                    .ok_or("lumberjack")?;
                match case {
                    2 => p["mode"] = "shared_calendar".into(),
                    3 => p["repeat"] = "daily".into(),
                    4 => p["npc_callbacks"][0]["quest"] = "oteryn:quest.foreign".into(),
                    5 => p["npc_callbacks"][0]["key"] = "oteryn:quest-reward/foreign".into(),
                    6 => p["npc_callbacks"][0]["npc"] = "oteryn:npc.".into(),
                    7 => {
                        let row = p["npc_callbacks"][0].clone();
                        p["npc_callbacks"]
                            .as_array_mut()
                            .ok_or("callbacks")?
                            .push(row);
                    }
                    8 => {
                        let row = p["npc_callbacks"][0].clone();
                        p["npc_callbacks"] = serde_json::json!(vec![row; 17]);
                    }
                    _ => p["key"] = "../foreign".into(),
                }
            }
            assert!(
                super::npc_keys(&source.to_string()).is_none(),
                "accepted invalid programme {case}"
            );
        }
        assert!(super::npc_keys("{broken").is_none());
        assert!(super::npc_keys(&" ".repeat(2 * 1024 * 1024 + 1)).is_none());
        Ok(())
    }
}
