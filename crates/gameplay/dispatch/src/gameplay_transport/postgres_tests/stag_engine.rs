//! Cathedral and Court Warlock wire USE against the committed B3 placements.
use super::fixture::{Harness, debug};
use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
use super::quest_world_callbacks::{input_item, local, wire};
use super::{AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult};
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::item_loop_move::{LoopDestination, LoopMoveRequest};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    court_warlock as court, stag_cathedral as cathedral,
    world_callbacks::{Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent, encode_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, encode_step_intent,
};
use std::sync::atomic::Ordering;

pub(super) const CATHEDRAL: &str = "stag-engine-cathedral";
pub(super) const COURT: &str = "order-of-the-stag-court-warlock";

#[test]
fn cathedral_pick_requires_ground_starlight_and_shares_descent_until_reset_postgres() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(CATHEDRAL, || {
        super::quest_world_callbacks::scenario(CATHEDRAL, cathedral::SOURCE, false)
    })
}

#[test]
fn court_warlock_lever_admits_and_ejects_two_stage_eleven_players_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(COURT, || {
        super::quest_world_callbacks::scenario(COURT, court::LEVER, false)
    })
}

pub(super) fn start(name: &str) -> Position {
    if name == CATHEDRAL {
        Position {
            y: 32468,
            ..cathedral::SOURCE
        }
    } else {
        Position {
            x: 33842,
            y: 32146,
            floor: -3,
        }
    }
}

pub(super) fn world(
    name: &str,
    native: &NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    let start = start(name);
    let start = (start.x, start.y);
    let tile = |position, ids| ExpectedTile {
        position,
        ids,
        counted_entry: None,
    };
    let (crops, appearances) = if name == CATHEDRAL {
        (
            vec![
                NativeCrop {
                    bytes: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../content/world/placements/region-z04-x126-y126.b3"
                    )),
                    region: (4, 126, 126),
                    bounds: (32401, 32465, 32403, 32468),
                    start,
                    expected: vec![
                        tile((32402, 32467), vec![516]),
                        tile((32402, 32465), vec![417, 17469, 9101]),
                    ],
                },
                NativeCrop {
                    bytes: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../content/world/placements/region-z05-x126-y126.b3"
                    )),
                    region: (5, 126, 126),
                    bounds: (32401, 32466, 32404, 32471),
                    start: (32402, 32467),
                    expected: vec![
                        tile((32402, 32467), vec![15785, 7114, 29333]),
                        tile((32403, 32470), vec![15785, 7151]),
                    ],
                },
            ],
            &[cathedral::CLOSED][..],
        )
    } else {
        (
            vec![
                NativeCrop {
                    bytes: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../content/world/placements/region-z03-x132-y125.b3"
                    )),
                    region: (3, 132, 125),
                    bounds: (33841, 32141, 33844, 32147),
                    start,
                    expected: vec![
                        tile((33843, 32147), vec![499, 8913]),
                        tile(start, vec![18013]),
                    ],
                },
                NativeCrop {
                    bytes: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../content/world/placements/region-z02-x132-y125.b3"
                    )),
                    region: (2, 132, 125),
                    bounds: (33830, 32133, 33847, 32150),
                    start: (33838, 32141),
                    expected: vec![tile((33838, 32141), vec![499, 4573, 10561, 6379, 7112])],
                },
            ],
            &[][..],
        )
    };
    native_scene_with_appearances(native, crops, &[], appearances)
}

fn target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    pos: Position,
    ordinal: u8,
) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("bundle map")?;
    let key = oteryn_world_bundle::bundle::placement_key(pos.floor, pos.x, pos.y, ordinal)
        .ok_or("placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

fn pace(owner: &mut ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("world USE pacing")?;
    Ok(())
}

async fn use_fixed(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    pos: Position,
    ordinal: u8,
) -> TestResult<UseDisposition> {
    pace(owner)?;
    let payload = encode_use_intent(&target(owner, pos, ordinal)?).map_err(debug)?;
    let reply = wire(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        2,
        payload,
    )
    .await?;
    decode_use_result(&reply).map_err(debug)
}

async fn use_pick(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<UseDisposition> {
    pace(owner)?;
    let pick = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", cathedral::PICK, "definition-r1"))
        .ok_or("pick definition")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(pick),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target(owner, cathedral::SOURCE, 0)?)),
        use_with_position: None,
    })
    .map_err(debug)?;
    let (next, result, _) = super::item_craft::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        None,
    )
    .await?;
    *admitted = next;
    Ok(result)
}

pub(super) async fn exercise_cathedral(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
    actor: ExactActorRef,
) -> TestResult {
    let stand = |item: &str| Offering {
        position: cathedral::STAND,
        item: item.into(),
        count: 1,
    };
    input_item(owner, fence.item_fence(), &stand(cathedral::PICK), 60, true).await?;
    assert_eq!(
        use_fixed(owner, admitted, cathedral::SOURCE, 0).await?,
        UseDisposition::NothingToUse
    );
    assert_eq!(use_pick(owner, admitted).await?, UseDisposition::Rejected);
    input_item(
        owner,
        fence.item_fence(),
        &stand(cathedral::STARLIGHT),
        61,
        true,
    )
    .await?;
    assert_eq!(use_pick(owner, admitted).await?, UseDisposition::Rejected);
    input_item(
        owner,
        fence.item_fence(),
        &stand(cathedral::STARLIGHT),
        62,
        false,
    )
    .await?;
    owner.states.loop_ground.invalidate();
    assert_eq!(use_pick(owner, admitted).await?, UseDisposition::Committed);
    let pos = TilePos {
        x: cathedral::SOURCE.x,
        y: cathedral::SOURCE.y,
        floor: cathedral::SOURCE.floor,
    };
    let map = owner.bundle_map().ok_or("map")?;
    let first_due = map
        .facts
        .next_movement_decay_time()
        .ok_or("opening deadline")?;
    assert_eq!(use_pick(owner, admitted).await?, UseDisposition::Committed);
    let map = owner.bundle_map().ok_or("map")?;
    let due = map
        .facts
        .next_movement_decay_time()
        .ok_or("renewed opening deadline")?;
    assert!(due > first_due);
    assert_eq!(map.facts.expire_movement_items(first_due), 0);
    // The stand gates the pick, but taking the item back must leave the shared opening live.
    let stand_cell = crate::durability::map_item_mint_audit::map_item_spatial_position(
        cathedral::STAND.x,
        cathedral::STAND.y,
        cathedral::STAND.floor,
    );
    let starlight: Vec<u8> = sqlx::query_scalar(
        "SELECT decode(replace(i.item_instance_id::text,'-',''),'hex')
         FROM game_item_instances i JOIN game_item_ground_locations g USING (item_instance_id)
         WHERE i.definition_production_key=$1 AND i.lifecycle=1 AND g.spatial_position=$2",
    )
    .bind(cathedral::STARLIGHT)
    .bind(&stand_cell)
    .fetch_one(&h.pool)
    .await?;
    let starlight = starlight.try_into().map_err(|_| "Frozen Starlight identity")?;
    {
        let native = owner
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or("native content")?;
        let permit = owner.spell_lane_permit().await.ok_or("item lane")?;
        let result = owner
            .root
            .commit_item_loop_move(
                &permit,
                owner.character,
                owner.holder,
                &fence.item_fence(),
                crate::foundation::CommandRef::new(
                    admitted.game_session_id,
                    crate::foundation::CommandId::new(1630).map_err(debug)?,
                ),
                &NativeItemWeights(native),
                LoopMoveRequest {
                    item: starlight,
                    count: None,
                    pickup_cell: Some(stand_cell),
                    destination: LoopDestination::MainBackpack,
                },
            )
            .await?;
        assert!(result.is_ok(), "take back Frozen Starlight: {result:?}");
    }
    owner.states.loop_ground.invalidate();
    assert_eq!(use_pick(owner, admitted).await?, UseDisposition::Rejected);
    assert_eq!(
        owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .next_movement_decay_time(),
        Some(due)
    );
    assert_eq!(
        use_fixed(owner, admitted, cathedral::SOURCE, 0).await?,
        UseDisposition::Committed
    );
    let evidence = super::house_transfer::RecoverySource::start(h).await?;
    let resident_session =
        super::activity_deadline_paging::admit_member(h, &evidence.evidence, 1).await?;
    let arrival = local(cathedral::DESTINATION);
    let resident = {
        let map = owner.bundle_map().ok_or("map")?;
        let mut runtime = owner.runtime.lock().await;
        let expected = runtime.read_actor_position(actor).map_err(debug)?;
        assert_eq!(expected.position(), arrival);
        let doors = crate::map::floor::BundleDoors(&map.facts);
        let return_to_entrance = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            owner.movement_cells(),
            actor,
            admitted.game_session_id,
            expected,
            local(start(CATHEDRAL)),
            false,
            &doors,
        )
        .map_err(debug)?;
        runtime
            .commit_source_step(return_to_entrance)
            .map_err(debug)?;

        // Another admitted player is still on the shared landing when the step arrives.
        let reservation = runtime
            .reserve_fresh_session(resident_session)
            .map_err(debug)?;
        let resident = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(resident, arrival)
            .map_err(debug)?;
        resident
    };
    pace(owner)?;
    wire(
        owner,
        admitted,
        admitted.continuity.next_command_id,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::North),
    )
    .await?;
    {
        let runtime = owner.runtime.lock().await;
        for player in [actor, resident] {
            assert_eq!(
                runtime
                    .read_actor_position(player)
                    .map_err(debug)?
                    .position(),
                arrival
            );
        }
        let warlocks = runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .filter(|actor| {
                runtime.matches_live_creature_identity(*actor, cathedral::WARLOCK.as_bytes())
            })
            .count();
        assert_eq!(warlocks, 1);
    }
    let map = owner.bundle_map().ok_or("map")?;
    assert_eq!(map.facts.expire_movement_items(due), 2);
    assert_eq!(map.facts.quest_portal_destination(pos, due), None);
    let ground = map
        .overlay
        .base()
        .tile(pos.x, pos.y, pos.floor)
        .ok_or("cathedral source")?
        .ids()[0];
    assert_eq!(
        map.facts
            .base_entry(pos, 0, ground)
            .ok_or("restored floor")?
            .appearance_id,
        cathedral::CLOSED
    );
    let quantities: Vec<(i64, bool)> = sqlx::query_as(
        "SELECT i.quantity, coalesce(g.spatial_position=$2,false) AS on_stand
         FROM game_item_instances i
         LEFT JOIN game_item_ground_locations g USING (item_instance_id)
         WHERE i.definition_production_key=$1 AND i.lifecycle=1
         ORDER BY on_stand",
    )
    .bind(cathedral::STARLIGHT)
    .bind(
        crate::durability::map_item_mint_audit::map_item_spatial_position(
            cathedral::STAND.x,
            cathedral::STAND.y,
            cathedral::STAND.floor,
        ),
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(quantities, vec![(1, false), (1, false)]);
    Ok(())
}

pub(super) async fn exercise_court(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
    deps: &Dependencies,
) -> TestResult {
    let evidence = super::house_transfer::RecoverySource::start(h).await?;
    let second_session =
        super::activity_deadline_paging::admit_member(h, &evidence.evidence, 1).await?;
    let (second, _) =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(second_session)
            .await?;
    let second_lease = second.current_character_lease();
    let second_start = Position {
        y: 32142,
        ..start(COURT)
    };
    let second_actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime
            .reserve_fresh_session(second_session)
            .map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, second_session, second_lease)
            .map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, local(second_start))
            .map_err(debug)?;
        actor
    };
    deps.initialize_player(owner, second_actor, second_session)
        .await;
    let seed = |character: [u8; 16], stage: i64| async move {
        sqlx::query(
            "INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
             VALUES(encode($1,'hex')::uuid,$2,$3,$4,2)
             ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value",
        ).bind(character.as_slice()).bind(court::TRACK).bind(court::QUEST).bind(stage)
            .execute(&h.pool).await
    };
    let first_character = *fence.character_id.as_bytes();
    let second_character = *second_lease.character_id().as_bytes();
    seed(first_character, 10).await?;
    seed(second_character, 11).await?;
    super::maintain_ready(&h.root).await?;
    assert_eq!(
        use_fixed(owner, admitted, court::LEVER, 1).await?,
        UseDisposition::Rejected
    );
    seed(first_character, 11).await?;
    seed(second_character, 10).await?;
    super::maintain_ready(&h.root).await?;
    assert_eq!(
        use_fixed(owner, admitted, court::LEVER, 1).await?,
        UseDisposition::Rejected
    );
    {
        let runtime = owner.runtime.lock().await;
        for (actor, at) in [(deps.actor, start(COURT)), (second_actor, second_start)] {
            assert_eq!(
                runtime
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position(),
                local(at)
            );
        }
    }
    seed(second_character, 11).await?;
    super::maintain_ready(&h.root).await?;
    assert_eq!(
        use_fixed(owner, admitted, court::LEVER, 1).await?,
        UseDisposition::Committed
    );
    let due = {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let phases = &states.encounter_phases;
        let due = phases.court_warlock_due_us().ok_or("first wave deadline")?;
        for actor in [deps.actor, second_actor] {
            assert_eq!(
                runtime
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position(),
                local(court::CENTER)
            );
        }
        due
    };
    owner.states.test_owner_time.store(due, Ordering::Relaxed);
    owner.drain_quest_encounter_deadlines().await;
    {
        let runtime = owner.runtime.lock().await;
        assert!(
            runtime
                .current_live_creature_census()
                .map_err(debug)?
                .is_empty()
        );
    }
    owner.states.test_owner_time.store(due + 1, Ordering::Relaxed);
    owner.drain_quest_encounter_deadlines().await;
    let exit_at = {
        let runtime = owner.runtime.lock().await;
        let creatures = runtime.current_live_creature_census().map_err(debug)?;
        assert_eq!(creatures.len(), court::RAUBRITTERS_PER_WAVE);
        assert!(creatures.iter().all(|actor| {
            court::RAUBRITTERS
                .iter()
                .any(|kind| runtime.matches_live_creature_identity(*actor, kind.as_bytes()))
        }));
        owner
            .states
            .spell_states
            .lock()
            .await
            .encounter_phases
            .court_warlock_due_us()
            .ok_or("encounter timeout")?
    };
    assert_eq!(exit_at - due, court::TIMEOUT_US - court::INITIAL_WAVE_US);
    owner
        .states
        .test_owner_time
        .store(exit_at + 1, Ordering::Relaxed);
    owner.drain_quest_encounter_deadlines().await;
    {
        let runtime = owner.runtime.lock().await;
        for actor in [deps.actor, second_actor] {
            assert_eq!(
                runtime
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position(),
                local(court::EXIT)
            );
        }
        assert!(
            runtime
                .current_live_creature_census()
                .map_err(debug)?
                .is_empty()
        );
        assert_eq!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .encounter_phases
                .court_warlock_due_us(),
            None
        );
    }
    let (stages, receipts): (Vec<i64>, i64) = sqlx::query_as(
        "SELECT ARRAY(SELECT value FROM game_character_quest_tracks WHERE track_key=$1 ORDER BY character_id),
                (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$2)",
    ).bind(court::TRACK).bind(court::KEY).fetch_one(&h.pool).await?;
    assert_eq!((stages, receipts), (vec![11, 11], 0));
    Ok(())
}
