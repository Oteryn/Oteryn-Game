use super::*;
use crate::quest::{QuestComparison, QuestEffectKind};

fn document() -> Result<serde_json::Value, String> {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/kill.json")).map_err(|e| e.to_string())?;
    let q = &mut value["quests"][0];
    let mut restart = q["stages"][0]["trigger"]["callback"].clone();
    restart["key"] = crate::quest::shared_world_progress::save_event_restart_key(
        q["quest"].as_str().ok_or("quest")?,
    )
    .into();
    restart["keywords"] = serde_json::json!(["restart"]);
    restart["storage"]=serde_json::Value::Array(q["tracks"].as_array().ok_or("tracks")?.iter().map(|t|serde_json::json!({"track":t["key"],"minimum":t["minimum"],"maximum":t["maximum"],"set":t["initial"]})).collect());
    q["tracks"].as_array_mut().ok_or("tracks")?.push(serde_json::json!({"key":"oteryn:quest-progress/authored-kills/private","initial":49,"minimum":0,"maximum":100}));
    q["occurrence"] =
        serde_json::json!({"kind":"server_save","world_change":"grimvale","restart":restart});
    Ok(value)
}

#[test]
fn qm55_authored_daily_profile_binds_every_stage_and_resets_only_declared_missions()
-> Result<(), String> {
    let bindings = Bindings::parse(&serde_json::to_vec(&document()?).map_err(|e| e.to_string())?)?;
    let catalogue = bindings.attach(
        &QuestStateCatalogue::new("fixture", vec![], vec![]).map_err(|e| format!("{e:?}"))?,
    )?;
    let quest = &bindings.quests[0].quest;
    let restart = catalogue
        .save_event_restart(quest)
        .ok_or("daily restart transition")?;
    assert_eq!(restart.world_change.as_deref(), Some("grimvale"));
    assert!(!restart.completes);
    assert_eq!(restart.effects.len(), 2);
    assert!(
        restart
            .effects
            .iter()
            .all(|effect| effect.from == QuestComparison::Any
                && effect.effect == QuestEffectKind::Set(0))
    );
    assert_eq!(
        catalogue
            .transition("oteryn:quest-reward/authored-kills-start")
            .ok_or("NPC stage")?
            .world_change
            .as_deref(),
        Some("grimvale")
    );
    assert_eq!(
        catalogue
            .transition("oteryn:quest-transition/native-kills/authored-engine-rat")
            .ok_or("kill stage")?
            .world_change
            .as_deref(),
        Some("grimvale")
    );
    let callbacks = crate::quest::reward_callbacks::load_for(&catalogue)?;
    assert!(callbacks.iter().any(|callback| callback.key == restart.key));
    let stored = std::collections::BTreeMap::from([
        ("oteryn:quest-progress/authored-kills/phase".into(), 5),
        ("oteryn:quest-progress/authored-kills/count".into(), 3),
        ("oteryn:quest-progress/authored-kills/private".into(), 49),
    ]);
    let changes = catalogue
        .evaluate(restart, &stored, 0)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(changes.len(), 2);
    assert!(
        changes
            .iter()
            .all(|change| change.after == 0 && !change.track.ends_with("/private"))
    );
    Ok(())
}

#[test]
fn qm55_daily_restart_rejects_rewards_foreign_tracks_and_noninitial_resets() -> Result<(), String> {
    let original = document()?;
    for mutation in 0..12 {
        let mut value = original.clone();
        let profile = &mut value["quests"][0]["occurrence"];
        match mutation {
            0 => {
                profile["restart"]["key"] =
                    "oteryn:quest-transition/save-event-restart/foreign".into()
            }
            1 => profile["restart"]["quest"] = "oteryn:quest.foreign".into(),
            2 => profile["restart"]["storage"][0]["track"] = "oteryn:quest-progress/foreign".into(),
            3 => profile["restart"]["storage"][0]["set"] = 1.into(),
            4 => profile["restart"]["storage"][0]["minimum"] = 1.into(),
            5 => profile["restart"]["storage"][0]["preserve"] = true.into(),
            6 => profile["restart"]["storage"][0]["increment"] = true.into(),
            7 => profile["restart"]["storage"][0]["at_least"] = true.into(),
            8 => {
                profile["restart"]["give"] =
                    serde_json::json!({"item":"oteryn:item.tibia.i3035","count":10})
            }
            9 => profile["restart"]["fee"] = serde_json::json!({"gold":1,"include_bank":false}),
            10 => profile["restart"]["completes"] = true.into(),
            _ => profile["world_change"] = "".into(),
        }
        assert!(
            Bindings::parse(&serde_json::to_vec(&value).map_err(|e| e.to_string())?).is_err(),
            "mutation {mutation} accepted"
        );
    }
    Ok(())
}
