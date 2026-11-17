//! Crystal brain mission through framed production USE and durable Ground custody.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    self,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) const NAME: &str = "brainout";
pub(super) const SOURCE: Position = Position {
    x: 33022,
    y: 32329,
    floor: -10,
};
pub(super) const START: Position = Position {
    x: 33022,
    y: 32330,
    floor: -10,
};
const KEY: &str = "oteryn:quest-callback/gravedigger-brain";

#[test]
fn brainout_ground_reward_atomic_replay_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    run(NAME)
}
#[test]
fn brainout_ground_reward_full_inventory_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    run("brainout-full")
}
#[test]
fn brainout_ground_reward_wrong_quest_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    run("brainout-wrong")
}
#[test]
fn brainout_ground_reward_east_control_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    run("brainout-east")
}
fn run(name: &'static str) -> TestResult {
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(
            name,
            if name.ends_with("-east") {
                Position { x: 33023, ..SOURCE }
            } else {
                SOURCE
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
pub(super) fn callback(name: &str) -> TestResult<Callback> {
    let key = if name.ends_with("-east") {
        "oteryn:quest-callback/gravedigger-brain-east"
    } else {
        KEY
    };
    quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key == key)
        .ok_or_else(|| "BRAINOUT production callback is missing".into())
}
pub(super) fn catalogue(
    original: &quest::QuestStateCatalogue,
    callback: &Callback,
    wrong: bool,
) -> TestResult<quest::QuestStateCatalogue> {
    // Arrange the NPC prerequisite only; the production USE owns the brain mission.
    let tracks = callback
        .storage
        .iter()
        .map(|storage| {
            let mut track = original
                .track(&storage.track)
                .cloned()
                .ok_or("brain mission track")?;
            if storage.preserve && !wrong {
                track.initial = 1;
            }
            Ok(track)
        })
        .collect::<Result<Vec<_>, &str>>()?;
    quest::QuestStateCatalogue::new(
        original.content_revision(),
        tracks,
        vec![
            original
                .transition(&callback.key)
                .cloned()
                .ok_or("brain mission transition")?,
        ],
    )
    .map_err(debug)
}
pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
    // The machine crosses the original x=33024 region boundary. Preserve both
    // source crops; neither the control stack nor the offering tiles are invented.
    native_scene(
        native,
        vec![
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z10-x128-y126.b3"
                )),
                region: (10, 128, 126),
                bounds: (33019, 32328, 33024, 32335),
                start: (START.x, START.y),
                expected: vec![
                    ExpectedTile {
                        position: (33022, 32329),
                        ids: vec![410, 17478, 7834, 17471, 19161],
                        counted_entry: None,
                    },
                    ExpectedTile {
                        position: (33023, 32329),
                        ids: vec![410, 17477, 7834, 17472, 19162],
                        counted_entry: None,
                    },
                    ExpectedTile {
                        position: (33020, 32332),
                        ids: vec![13988, 17477, 17478, 3522],
                        counted_entry: None,
                    },
                    ExpectedTile {
                        position: (33022, 32332),
                        ids: vec![410, 7833, 17479, 17469, 3531],
                        counted_entry: None,
                    },
                ],
            },
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z10-x129-y126.b3"
                )),
                region: (10, 129, 126),
                bounds: (33024, 32328, 33027, 32335),
                start: (START.x, START.y),
                expected: vec![ExpectedTile {
                    position: (33025, 32332),
                    ids: vec![32688, 17469, 7833, 3521],
                    counted_entry: None,
                }],
            },
        ],
        &[],
    )
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
    name: &str,
) -> TestResult {
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Brain", owner.owner_now());
    let source = callback
        .selector
        .native
        .as_ref()
        .ok_or("native brain control")?;
    let (key, placement) = {
        let map = owner.bundle_map().ok_or("map")?;
        let key = oteryn_world_bundle::bundle::placement_key(
            source.position.floor,
            source.position.x,
            source.position.y,
            source.ordinal,
        )
        .ok_or("control placement")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        (key, placement)
    };
    let payload = |revision| {
        oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: revision,
        })
        .map_err(debug)
    };
    let use_ = async |owner: &mut ComposedFreshAdmission<'_, '_, '_>,
                      admitted: &mut AdmittedSession,
                      command,
                      revision|
           -> TestResult<UseDisposition> {
        owner.clock_origin -= std::time::Duration::from_secs(2);
        let reply = Box::pin(super::quest_world_callbacks::wire(
            owner,
            admitted,
            command,
            2,
            payload(revision)?,
        ))
        .await?;
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(|e| debug(e).into())
    };
    let first = &callback.offerings[0];
    let second = &callback.offerings[1];
    super::quest_world_callbacks::input_item(owner, fence.item_fence(), first, 8701, false).await?;
    owner.states.loop_ground.invalidate();
    assert_eq!(
        use_(owner, admitted, 1, 0).await?,
        UseDisposition::Rejected,
        "one brain refuses"
    );
    let expected_refusal = if name.ends_with("-wrong") {
        "You have already got your brain"
    } else {
        "No brains"
    };
    assert!(
        owner
            .states
            .chat
            .drain(admitted.game_session_id)
            .iter()
            .any(|line| matches!(line,
        oteryn_protocol_oteryn::chat::ChatLine::Private {text,..} if text == expected_refusal))
    );
    assert_eq!(counts(h).await?, (1, 0, 0), "missing input must roll back");
    super::quest_world_callbacks::input_item(owner, fence.item_fence(), second, 8702, false)
        .await?;
    if name.ends_with("-full") {
        for n in 0..20 {
            super::quest_world_callbacks::input_item(
                owner,
                fence.item_fence(),
                &quest::world_callbacks::Offering {
                    position: START,
                    item: "oteryn:item.tibia.i3031".into(),
                    count: 1,
                },
                8800 + n,
                true,
            )
            .await?;
        }
        let entries:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(entries, 20, "full backpack fixture");
    }
    owner.states.loop_ground.invalidate();
    if name.ends_with("-wrong") {
        assert_eq!(use_(owner, admitted, 2, 0).await?, UseDisposition::Rejected);
        assert_eq!(counts(h).await?, (2, 0, 0));
        return Ok(());
    }
    assert_eq!(
        use_(owner, admitted, 2, 0).await?,
        UseDisposition::Committed
    );
    assert_eq!(counts(h).await?, (0, 1, 1));
    if name.ends_with("-full") {
        let gold:i64=sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_container_entries c JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i3031' AND i.lifecycle=1").fetch_one(&h.pool).await?;
        assert_eq!(
            gold, 20,
            "inventory contents survive the ground-output transaction"
        );
    }
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(&callback.storage[1].track), Some(&1));
    let ground:(Vec<u8>,i64)=sqlx::query_as("SELECT g.spatial_position,i.quantity FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i19078' AND i.lifecycle=1")
        .fetch_one(&h.pool).await?;
    let mut expected = 33022_i32.to_be_bytes().to_vec();
    expected.extend_from_slice(&32332_i32.to_be_bytes());
    expected.extend_from_slice(&(-10_i16).to_be_bytes());
    assert_eq!(ground, (expected, 1));
    let carried:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries c JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i19078'").fetch_one(&h.pool).await?;
    assert_eq!(carried, 0, "donor always creates the reward on ground");
    assert_eq!(
        use_(owner, admitted, 2, 0).await?,
        UseDisposition::Committed,
        "same command replays"
    );
    assert_eq!(counts(h).await?, (0, 1, 1));
    assert_eq!(
        owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        1
    );
    assert_eq!(
        use_(owner, admitted, 3, 1).await?,
        UseDisposition::Rejected,
        "new command cannot repeat quest"
    );
    assert_eq!(counts(h).await?, (0, 1, 1));
    Ok(())
}
async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i9659'),(SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i19078'),(SELECT count(*) FROM game_character_quest_receipts)").fetch_one(&h.pool).await?)
}

#[test]
fn brainout_ground_output_schema_rejects_unbound_or_mixed_outputs() -> TestResult {
    let original = callback(NAME)?;
    let value = serde_json::to_value(original)?;
    for case in 0..4 {
        let mut invalid = value.clone();
        match case {
            0 => invalid["effects"]["ground_output"]["count"] = 0.into(),
            1 => invalid["offerings"] = serde_json::json!([]),
            2 => {
                invalid["effects"]["item_reward"] =
                    serde_json::json!({"item":"oteryn:item.tibia.i3031","count":1,"contents":[]})
            }
            _ => invalid["effects"]["ground_output"]["position"]["floor"] = 1.into(),
        }
        assert!(
            quest::world_callbacks::parse(
                &serde_json::json!({"schema":1,"callbacks":[invalid]}).to_string()
            )
            .is_err(),
            "invalid Ground output case {case}"
        );
    }
    Ok(())
}
