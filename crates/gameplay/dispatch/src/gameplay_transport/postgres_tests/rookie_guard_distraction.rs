//! Crystal 201189f8 data-global/scripts/quests/the_rookie_guard/mission12_into_fortress.lua:251-294.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{input_item, local, world_cells};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::{MapFacts, placement_key};
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Offering, Position};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

pub(super) const NAME: &str = "rookie-guard-distraction";
const GUARD: Position = Position {
    x: 31977,
    y: 32150,
    floor: -7,
};
const EJECT: Position = Position {
    x: 31977,
    y: 32154,
    floor: -7,
};
const QUEST: &str = "oteryn:quest.the_rookie_guard_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission12";
const BONE: &str = "oteryn:item.tibia.i12674";
const APPEARANCE: u16 = 12792;
const GUARD_ORDINAL: u8 = 2;
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission12_into_fortress.lua:251-294";

#[test]
fn fleshy_bone_use_retains_the_item_and_recovers_the_guard_after_two_minutes() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, GUARD, false)
    })
}

fn document() -> serde_json::Value {
    use serde_json::json;
    let first = json!({
        "key":"oteryn:quest-callback/rookie-guard-distraction", "quest":QUEST, "trigger":"use",
        "selector":{"position":GUARD,"appearance":APPEARANCE,
            "native":{"position":GUARD,"ordinal":GUARD_ORDINAL,"appearance":APPEARANCE}},
        "requires_world_admission":true, "passages":[],"offerings":[],
        "reversible":false,"return_on_failure":false,
        "storage":[{"track":TRACK,"minimum":5,"maximum":5,"set":6}], "experience":50,
        "item_on_target":{"item":BONE,"count":1,"reusable":true,"selected_instance":true},
        "effects":{"distraction":{"duration_ms":120000,"radius":2,"eject_to":EJECT,
            "spawns":[{"creature":"oteryn:creature.running_elite_orc_guard","offset_y":1,"count":1},
                      {"creature":"oteryn:creature.wild_dog","offset_y":3,"count":8}]}},
        "sources":[SOURCE]
    });
    let mut retry = first.clone();
    retry["key"] = "oteryn:quest-callback/rookie-guard-distraction-retry".into();
    retry["storage"] = json!([{"track":TRACK,"minimum":6,"maximum":15,"set":6,"at_least":true}]);
    retry["experience"] = serde_json::Value::Null;
    json!({"schema":1,"callbacks":[first,retry]})
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(document()["callbacks"][0].clone())?)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    world_cells(
        callback,
        GUARD,
        APPEARANCE,
        GUARD_ORDINAL,
        &[],
        &[],
        None,
        &[
            EJECT,
            Position {
                y: GUARD.y + 3,
                ..GUARD
            },
        ],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let fixture = QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack {
            key: TRACK.into(),
            quest: QUEST.into(),
            initial: 5,
            min: -1,
            max: 15,
        }],
        vec![],
    )
    .map_err(debug)?;
    let document = document();
    owner.quest_catalogue = Some(std::sync::Arc::new(world_callbacks::fixture_catalogue(
        fixture,
        &document.to_string(),
    )?));
    owner
        .states
        .quest_callbacks
        .fixture
        .set(serde_json::from_value(document["callbacks"].clone())?)
        .map_err(|_| "callback fixture already installed")?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "current quest fence")?
        .ok_or("current quest fence")?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("capabilities")?;
    input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: GUARD,
            item: BONE.into(),
            count: 1,
        },
        12674,
        true,
    )
    .await?;
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native")?;
    let reference = native
        .item_index()
        .and_then(|index| index.definition_ref("Item", BONE, "definition-r1"))
        .ok_or("fleshy bone definition")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .install_companion_policies(native.policies().clone())
            .map_err(debug)?;
    }
    let target = crate::map::overlay::TilePos {
        x: GUARD.x,
        y: GUARD.y,
        floor: GUARD.floor,
    };
    let key = placement_key(target, GUARD_ORDINAL).ok_or("guard placement")?;
    let origin = owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position();
    use_bone(owner, admitted, key, reference).await?;
    {
        let runtime = owner.runtime.lock().await;
        let census = runtime.current_live_creature_census().map_err(debug)?;
        assert_eq!(census.len(), 9);
        assert_eq!(
            census
                .iter()
                .filter(
                    |actor| runtime.read_actor_position(**actor).is_ok_and(|at| {
                        at.position()
                            == local(Position {
                                y: GUARD.y + 3,
                                ..GUARD
                            })
                    })
                )
                .count(),
            8
        );
    }
    let (stage, bones): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT value FROM game_character_quest_tracks WHERE track_key=$1), \
                (SELECT count(*) FROM game_item_instances WHERE definition_production_key=$2 AND lifecycle=1)",
    ).bind(TRACK).bind(BONE).fetch_one(&h.pool).await?;
    assert_eq!((stage, bones), (6, 1));
    let first_experience: i64 = sqlx::query_scalar(
        "SELECT total_experience FROM game_character_progression_state",
    ).fetch_one(&h.pool).await?;
    owner.drain_quest_encounter_deadlines().await;
    assert!(owner.bundle_map().ok_or("map")?.facts.base_hidden(key));
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(121))
        .ok_or("clock")?;
    owner.drain_quest_encounter_deadlines().await;
    assert!(!owner.bundle_map().ok_or("map")?.facts.base_hidden(key));
    let mut runtime = owner.runtime.lock().await;
    assert!(
        runtime
            .current_live_creature_census()
            .map_err(debug)?
            .is_empty()
    );
    let ejected = runtime.read_actor_position(actor).map_err(debug)?;
    assert_eq!(ejected.position(), local(EJECT));
    let map = owner.bundle_map().ok_or("map")?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let step = crate::map::floor::prepare_quest_callback_teleport(
        &runtime, owner.movement_cells(), actor, admitted.game_session_id,
        ejected, origin, false, &doors,
    ).map_err(debug)?;
    runtime.commit_source_step(step).map_err(debug)?;
    drop(runtime);
    use_bone(owner, admitted, key, reference).await?;
    assert!(owner.bundle_map().ok_or("map")?.facts.base_hidden(key));
    assert_eq!(
        owner.runtime.lock().await.current_live_creature_census().map_err(debug)?.len(),
        9,
    );
    let retry_experience: i64 = sqlx::query_scalar(
        "SELECT total_experience FROM game_character_progression_state",
    ).fetch_one(&h.pool).await?;
    assert_eq!(retry_experience, first_experience);
    Ok(())
}

async fn use_bone(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    key: u64,
    reference: std::num::NonZeroU32,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(3).ok_or("handle")?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        })),
        use_with_position: None,
    }).map_err(debug)?;
    let (next, result, _) = super::item_craft::exchange(
        owner, *admitted, admitted.continuity.next_command_id, &payload, Some(reference),
    ).await?;
    *admitted = next;
    assert_eq!(result, UseDisposition::Committed);
    Ok(())
}
