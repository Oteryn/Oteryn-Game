//! The complete pinned UID30024 source lever contract through production USE.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::{Position, Trigger};

#[test]
fn giant_smithhammer_source_lever_opens_and_restores_the_wall() -> TestResult {
    let shard: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/quests/definitions/quests-00100-00199.json"
    ))?;
    let definitions = shard["records"]
        .as_array()
        .ok_or("quest records")?
        .iter()
        .filter(|row| {
            row["definition"]["identity"]["key"] == "oteryn:quest.giant_smithhammer_quest"
        })
        .collect::<Vec<_>>();
    assert_eq!(definitions.len(), 1);
    let definition = &definitions[0]["definition"];
    assert_eq!(definition["kind"], "script_only");
    assert_eq!(definition["claims"], serde_json::json!([]));
    assert_eq!(definition["source_refs"]["gates"], serde_json::json!([]));
    assert_eq!(definition["reported_source_readiness"]["interactions"], 1);
    assert_eq!(
        definition["reported_source_readiness"]["features"],
        serde_json::json!(["trigger_use", "world_object"])
    );
    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load()?;
    let source = callbacks
        .iter()
        .filter(|c| c.quest == "oteryn:quest.giant_smithhammer_quest")
        .collect::<Vec<_>>();
    assert_eq!(source.len(), 1);
    let callback = source[0];
    assert_eq!(callback.key, "oteryn:quest-callback/giant-smithhammer");
    assert_eq!(callback.trigger, Trigger::Use);
    assert_eq!(callback.selector.unique, Some(30024));
    assert_eq!(callback.selector.action, None);
    assert_eq!(callback.selector.appearance, Some(2772));
    assert_eq!(callback.selector.position, None);
    assert_eq!(callback.lever, Some([2772, 2773]));
    assert!(callback.reversible);
    assert_eq!(callback.passages.len(), 1);
    assert_eq!(
        callback.passages[0].position,
        Position {
            x: 32780,
            y: 32231,
            floor: -8
        }
    );
    assert_eq!(callback.passages[0].appearance, 389);
    assert_eq!(callback.passages[0].replacement, None);
    assert!(callback.storage.is_empty() && callback.offerings.is_empty());
    assert!(callback.teleport.is_none() && callback.item_on_target.is_none());
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("giant-smithhammer-lever", || {
        super::quest_world_callbacks::scenario(
            "giant-smithhammer",
            Position {
                x: 32778,
                y: 32232,
                floor: -8,
            },
            false,
        )
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED giant_smithhammer_source_lever_opens_and_restores_the_wall duration={:?}",
        started.elapsed()
    );
    Ok(())
}
