//! Native offering and pagoda are separate source objects, committed through USE.
//! OTS_HYPOTHESIS_ONLY: Crystal 201189f actions_lions_rock.lua:14-84.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::{
    self,
    world_callbacks::{Callback, Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

const AT: Position = Position {
    x: 33137,
    y: 32291,
    floor: -8,
};
const PAGODA: TilePos = TilePos {
    x: 33134,
    y: 32289,
    floor: -8,
};

#[test]
fn m44_native_lions_first_offering_consumption_progress_and_fenced_pagoda_reset() -> TestResult {
    case("first")
}
#[test]
fn m44_native_lions_later_offering_preserves_existing_progress_and_resets_pagoda() -> TestResult {
    case("later")
}
#[test]
fn m44_native_lions_unknown_commit_reserves_both_objects_until_reconciled() -> TestResult {
    case("first-uncertain")
}
fn case(phase: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("m44-lions", move || {
        super::quest_world_callbacks::scenario(&format!("m44-lions-strength/{phase}"), AT, false)
    })
}
pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    native_scene_with_appearances(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z08-x129-y126.b3"
            )),
            region: (8, 129, 126),
            bounds: (33133, 32288, 33139, 32293),
            start: (33136, 32291),
            expected: vec![
                ExpectedTile {
                    position: (AT.x, AT.y),
                    ids: vec![19271, 21390],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (PAGODA.x, PAGODA.y),
                    ids: vec![499, 2074],
                    counted_entry: None,
                },
            ],
        }],
        &[],
        &[2075],
    )
}

/// Only the externally earned starting questline differs between the two cases.
pub(super) fn catalogue(
    original: &quest::QuestStateCatalogue,
    callback: &Callback,
) -> TestResult<quest::QuestStateCatalogue> {
    let tracks = callback
        .storage
        .iter()
        .map(|state| {
            let mut track = original.track(&state.track).cloned().ok_or("lions track")?;
            track.initial = if state.increment {
                3
            } else {
                state.minimum.max(track.min)
            };
            Ok(track)
        })
        .collect::<Result<Vec<_>, &str>>()?;
    quest::QuestStateCatalogue::new(
        original.content_revision(),
        tracks,
        vec![
            original
                .transition(&callback.key)
                .ok_or("lions transition")?
                .clone(),
        ],
    )
    .map_err(debug)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
    uncertain: bool,
) -> TestResult {
    let offering = callback.item_on_target.as_ref().ok_or("offering")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: AT,
            item: offering.item.clone(),
            count: 2,
        },
        70,
        true,
    )
    .await?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1)
        .ok_or("pedestal key")?;
    let pagoda = oteryn_world_bundle::bundle::placement_key(PAGODA.floor, PAGODA.x, PAGODA.y, 1)
        .ok_or("pagoda key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = |revision| {
        oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: revision,
        })
        .map_err(debug)
    };
    let _clock = owner.freeze_owner_clock_for_test();
    let use_payload = payload(0)?;
    let later = callback.key.ends_with("/later");
    if uncertain {
        use crate::durability::quest_state::QuestCause;
        use crate::foundation::{CommandId, CommandRef};
        owner
            .states
            .test_owner_time
            .store(5_000_001, std::sync::atomic::Ordering::Relaxed);
        owner
            .states
            .quest_callbacks
            .interrupt_after_commit
            .store(true, std::sync::atomic::Ordering::Release);
        assert!(
            super::quest_world_callbacks::wire(owner, admitted, 1, 2, use_payload)
                .await
                .is_err()
        );
        assert!(owner.has_pending_quest_callback(admitted.game_session_id));
        assert_eq!(
            (
                map.facts.object_revision(key),
                map.facts.object_revision(pagoda)
            ),
            (0, 0)
        );
        assert_eq!(map.facts.next_movement_decay_time(), None);
        let actor = admitted.runtime_actor.ok_or("lions actor")?;
        let original = QuestCause::Use(CommandRef::new(
            admitted.game_session_id,
            CommandId::new(1).map_err(debug)?,
        ));
        let fresh = QuestCause::Use(CommandRef::new(
            admitted.game_session_id,
            CommandId::new(2).map_err(debug)?,
        ));
        let pending = &owner.states.quest_callbacks;
        assert!(pending.source_target_available(
            actor,
            admitted.game_session_id,
            &original,
            key,
            pagoda
        ));
        assert!(!pending.source_target_available(
            actor,
            admitted.game_session_id,
            &fresh,
            key,
            pagoda
        ));
        assert!(!pending.source_target_available(
            actor,
            admitted.game_session_id,
            &fresh,
            key + 10,
            pagoda
        ));
        assert!(!pending.source_target_available(
            actor,
            admitted.game_session_id,
            &fresh,
            key,
            pagoda + 10
        ));
        assert!(pending.source_target_available(
            actor,
            admitted.game_session_id,
            &fresh,
            key + 10,
            pagoda + 10
        ));
        assert_durable(h, fence, &offering.item, owner.world_id, false).await?;
        owner
            .states
            .test_owner_time
            .store(6_000_001, std::sync::atomic::Ordering::Relaxed);
        assert!(
            owner
                .reconcile_quest_callbacks(admitted.game_session_id)
                .await
        );
        assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
        assert_eq!(
            (
                map.facts.object_revision(key),
                map.facts.object_revision(pagoda)
            ),
            (1, 1)
        );
        assert_eq!(map.facts.next_movement_decay_time(), Some(21_000_000));
        assert!(pending.source_target_available(
            actor,
            admitted.game_session_id,
            &fresh,
            key,
            pagoda
        ));
        assert_durable(h, fence, &offering.item, owner.world_id, false).await?;
        assert_eq!(map.facts.expire_movement_items(21_000_000), 1);
        eprintln!(
            "POSTGRES_CASE_COMPLETED M44 lions_unknown_commit objects_reserved original_cause_reconciled no_double_consumption"
        );
        return Ok(());
    }

    let (result, due) = super::m44_source_lifecycle::commit_through_barrier(
        owner,
        admitted,
        h,
        use_payload.clone(),
        15_000_000,
        if later { 4_444_004 } else { 4_444_003 },
    )
    .await?;
    let appearance = || -> TestResult<u16> {
        let base = map
            .overlay
            .base()
            .tile(PAGODA.x, PAGODA.y, PAGODA.floor)
            .ok_or("pagoda tile")?;
        let id = *base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, d)| **d == 0)
            .nth(1)
            .ok_or("pagoda ordinal")?
            .0;
        Ok(map
            .facts
            .base_entry(PAGODA, 1, id)
            .ok_or("pagoda facts")?
            .appearance_id)
    };
    assert_eq!(appearance()?, 2075);
    assert_eq!(
        (
            map.facts.object_revision(key),
            map.facts.object_revision(pagoda)
        ),
        (1, 1)
    );
    assert_eq!(
        super::quest_world_callbacks::wire(owner, admitted, 1, 2, use_payload).await?,
        result
    );
    assert_eq!(map.facts.next_movement_decay_time(), Some(due));
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert_eq!(appearance()?, 2074);
    assert_eq!(
        (
            map.facts.object_revision(key),
            map.facts.object_revision(pagoda)
        ),
        (1, 2)
    );
    super::m44_source_lifecycle::wait_for_world_action(owner, admitted).await?;
    let refused = super::quest_world_callbacks::wire(owner, admitted, 2, 2, payload(1)?).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&refused).map_err(debug)?,
        UseDisposition::Rejected
    );
    assert_durable(h, fence, &offering.item, owner.world_id, later).await?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED M44 lions_native_pagoda phase={} offering_atomic replay exact_reset once_only",
        if later { "later" } else { "first" }
    );
    Ok(())
}

async fn assert_durable(
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
    offering: &str,
    world: crate::foundation::WorldId,
    later: bool,
) -> TestResult {
    let tracks: Vec<(String, i64)> = sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid ORDER BY track_key LIMIT 3")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_all(&h.pool).await?;
    assert_eq!(tracks.len(), 2);
    assert_eq!(
        tracks
            .iter()
            .find(|(key, _)| key.ends_with("/lions_strength"))
            .map(|(_, value)| *value),
        Some(1)
    );
    assert_eq!(
        tracks
            .iter()
            .find(|(key, _)| key.ends_with("/questline"))
            .map(|(_, value)| *value),
        Some(if later { 4 } else { 2 })
    );
    let counts: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts),(SELECT count(*) FROM game_quest_callback_consumptions)")
        .fetch_one(&h.pool).await?;
    assert_eq!(counts, (1, 1));
    let remaining: i64 = sqlx::query_scalar("SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.definition_production_key=$3")
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(offering).fetch_one(&h.pool).await?;
    assert_eq!(remaining, 1);
    Ok(())
}
