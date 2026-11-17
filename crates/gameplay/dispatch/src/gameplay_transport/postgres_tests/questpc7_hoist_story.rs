//! Complete chosen hoist knowledge programme and native position-bound transport.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
const TRACK: &str = "oteryn:quest-progress/questpc7/hoist/programme";
const PARENT: &str = "oteryn:quest.the_ice_islands_quest";
#[test]
fn questpc7_hoist_full_knowledge_control_completion_and_native_binding() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc7-hoist-story", || {
        super::quest_world_callbacks::scenario(
            "questpc7-hoist-story",
            Position {
                x: 32154,
                y: 31118,
                floor: -9,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_HOIST_STORY POSTGRES_CASE_COMPLETED duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
async fn progress(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &AdmittedSession,
) -> TestResult<i64> {
    let f = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let state = owner
        .root
        .read_character_quest_state(owner.character, f.character_id)
        .await
        .map_err(debug)?;
    Ok(state.tracks().get(TRACK).copied().unwrap_or(-1))
}
async fn step(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    d: StepDirection,
) -> TestResult {
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    let n = a.continuity.next_command_id;
    let r = super::quest_world_callbacks::wire(
        owner,
        a,
        n,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(d),
    )
    .await?;
    assert_eq!(
        decode_step_result(&r).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}
async fn walk_to(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    x: i32,
    y: i32,
) -> TestResult {
    for _ in 0..32 {
        let p = owner
            .runtime
            .lock()
            .await
            .read_actor_position(a.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position();
        if (p.x, p.y) == (x, y) {
            return Ok(());
        }
        let direction = if p.x < x {
            StepDirection::East
        } else if p.x > x {
            StepDirection::West
        } else if p.y < y {
            StepDirection::South
        } else {
            StepDirection::North
        };
        step(owner, a, direction).await?;
    }
    Err("native walking did not reach its declared target".into())
}
pub(super) fn target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    p: Position,
    appearance: u16,
) -> TestResult<Vec<u8>> {
    let map = owner.bundle_map().ok_or("map")?;
    let tile = crate::map::overlay::TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    };
    let base = map.overlay.base().tile(p.x, p.y, p.floor).ok_or("tile")?;
    let matches: Vec<_> = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, d)| **d == 0)
        .enumerate()
        .filter_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            let entry = map.facts.base_entry(tile, ordinal, *id)?;
            (entry.appearance_id == appearance).then_some(ordinal)
        })
        .collect();
    let [ordinal] = matches.as_slice() else {
        return Err("physical target must be unique".into());
    };
    let key =
        oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, *ordinal).ok_or("key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
    .map_err(debug)
}
async fn use_object(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    p: Position,
    appearance: u16,
) -> TestResult<UseDisposition> {
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    let payload = target(owner, p, appearance)?;
    let n = a.continuity.next_command_id;
    let r = super::quest_world_callbacks::wire(owner, a, n, 2, payload).await?;
    decode_use_result(&r).map_err(debug)
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    a: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_HOIST_STORY POSTGRES_STARTED version={version}");
    assert_eq!(progress(owner, a).await?, -1, "no subject state fixtures");
    let actor = a.runtime_actor.ok_or("actor")?;
    let fence = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            a.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let callbacks = oteryn_gameplay_quests::quest::world_callbacks::load().map_err(debug)?;
    let entry = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/questpc7-hoist-entry")
        .ok_or("missing complete hoist programme entry")?;
    assert_eq!(entry.quest, "oteryn:quest.formogar_mine_hoist");
    for key in [
        "oteryn:quest-progress/crystalserver/quest/u8_0/the_ice_islands/formorgar_mines_hoist_skeleton",
        "oteryn:quest-progress/crystalserver/quest/u8_0/the_ice_islands/formorgar_mines_hoist_chest",
    ] {
        assert_eq!(
            owner
                .quest_catalogue
                .as_ref()
                .ok_or("catalogue")?
                .track(key)
                .ok_or("track")?
                .quest,
            entry.quest,
            "typed ownership, not legacy namespace spelling"
        );
    }
    let mut selected = a.continuity.selected_capabilities.as_slice().to_vec();
    selected.extend([3, 4, 6, 11, 15, 18, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    a.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("capabilities")?;
    step(owner, a, StepDirection::East).await?;
    assert_eq!(
        progress(owner, a).await?,
        -1,
        "parent completion is required"
    );
    super::questpc7_eligibility::complete_parent_key(owner, a, PARENT).await?;
    let prior:String=sqlx::query_scalar("SELECT completed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id(41).as_slice()).bind(PARENT).fetch_one(&h.pool).await?;
    step(owner, a, StepDirection::East).await?;
    assert_eq!(progress(owner, a).await?, 0);
    walk_to(owner, a, 32154, 31117).await?;
    let chest = Position {
        x: 32154,
        y: 31116,
        floor: -9,
    };
    let premium = owner.premium_coordinator;
    owner.premium_coordinator = None;
    let denied = use_object(owner, a, chest, 2472).await?;
    owner.premium_coordinator = premium;
    assert_eq!(denied, UseDisposition::Rejected);
    assert_eq!(progress(owner, a).await?, 0);
    assert_eq!(
        use_object(owner, a, chest, 2472).await?,
        UseDisposition::Committed
    );
    assert_eq!(progress(owner, a).await?, 1);
    walk_to(owner, a, 32155, 31125).await?;
    assert_eq!(
        use_object(
            owner,
            a,
            Position {
                x: 32156,
                y: 31125,
                floor: -9
            },
            5772
        )
        .await?,
        UseDisposition::Committed
    );
    assert_eq!(progress(owner, a).await?, 2);
    let fence = owner
        .current_quest_fence(a.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    for key in [
        "oteryn:quest-progress/crystalserver/quest/u8_0/the_ice_islands/formorgar_mines_hoist_skeleton",
        "oteryn:quest-progress/crystalserver/quest/u8_0/the_ice_islands/formorgar_mines_hoist_chest",
    ] {
        assert_eq!(state.tracks().get(key), Some(&1));
    }

    {
        let actor = a.runtime_actor.ok_or("actor")?;
        let runtime = owner.runtime.lock().await;
        let expected = runtime.read_actor_position(actor).map_err(debug)?;
        assert_eq!(
            (expected.position().x, expected.position().y),
            (32155, 31125)
        );
        let map = owner.bundle_map().ok_or("map")?;
        let closed = crate::map::floor::BundleDoors(map.facts.doors());
        let probe = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            &owner.movement_cells,
            actor,
            a.game_session_id,
            expected,
            crate::foundation::MovementLocalPosition {
                x: 32157,
                y: 31125,
                floor: 10,
            },
            false,
            &closed,
        );
        if let Err(error) = probe {
            return Err(format!("native destination admission: {error:?}").into());
        }
    }
    {
        let actor = a.runtime_actor.ok_or("actor")?;
        let context = {
            let runtime = owner.runtime.lock().await;
            let expected = runtime.read_actor_position(actor).map_err(debug)?;
            crate::durability::quest_world_callback::QuestWorldContext {
                party: Vec::new(),
                map_revision: crate::map::overlay::map_revision(
                    owner.bundle_map().ok_or("map")?.overlay.base(),
                ),
                content_revision: format!(
                    "sha256:{}",
                    runtime
                        .content_pin()
                        .server_artifact_digest()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                ),
                placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                reset_epoch: owner.reset_epoch,
                actor_placement: actor.placement_identity(),
                movement_revision: expected.revision(),
            }
        };
        let permit = owner.spell_lane_permit().await.ok_or("permit")?;
        let allowed = owner
            .root
            .check_character_world_callback(
                owner.character,
                owner.holder,
                fence,
                "oteryn:quest-callback/formogar-hoist-3059",
                owner.quest_catalogue.clone().ok_or("catalogue")?,
                &permit,
                context,
            )
            .await
            .map_err(debug)?;
        assert!(allowed, "fresh durable transport predicate");
    }
    walk_to(owner, a, 32157, 31125).await?;
    let actor = a.runtime_actor.ok_or("actor")?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position()
            .floor,
        10,
        "actual positioned platform binding transports without donor UID"
    );
    let control = Position {
        x: 32156,
        y: 31125,
        floor: -10,
    };
    let payload = target(owner, control, 2773)?;
    tokio::time::sleep(std::time::Duration::from_millis(210)).await;
    let n = a.continuity.next_command_id;
    let reply = super::quest_world_callbacks::wire(owner, a, n, 2, payload.clone()).await?;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(progress(owner, a).await?, 3);
    assert_eq!(
        super::quest_world_callbacks::wire(owner, a, n, 2, payload).await?,
        reply,
        "exact USE replay"
    );
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.formogar_mine_hoist'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    let after:String=sqlx::query_scalar("SELECT completed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id(41).as_slice()).bind(PARENT).fetch_one(&h.pool).await?;
    assert_eq!(prior, after, "parent completion remains read-only");
    Ok(())
}
