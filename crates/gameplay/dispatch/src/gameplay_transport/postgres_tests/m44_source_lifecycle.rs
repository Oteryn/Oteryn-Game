//! M44: native source USE, durable commit latency and revision-fenced reset.
//! OTS_HYPOTHESIS_ONLY: Crystal 201189f871d23597b82caf88fd611454a2ddefb8,
//! data-global/scripts/quests/an_uneasy_alliance/actions_crystal_ball.lua:1-24.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    self,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

const AT: Position = Position {
    x: 33155,
    y: 31297,
    floor: -3,
};
const NAME: &str = "uneasy-alliance-crystal";

#[test]
fn m44_native_source_reset_starts_after_commit_and_fences_newer_revisions() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("m44-source-lifecycle", || {
        super::quest_world_callbacks::scenario(NAME, AT, false)
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
                "/../../../content/world/placements/region-z03-x129-y122.b3"
            )),
            region: (3, 129, 122),
            bounds: (33153, 31295, 33157, 31299),
            start: (33155, 31298),
            expected: vec![ExpectedTile {
                position: (33155, 31297),
                ids: vec![9929, 10052, 10186, 1897],
                counted_entry: None,
            }],
        }],
        &[],
        &[10187],
    )
}

/// Arrange only the NPC prerequisite; keep the production transition unchanged.
pub(super) fn catalogue(
    original: &quest::QuestStateCatalogue,
    callback: &Callback,
) -> TestResult<quest::QuestStateCatalogue> {
    let tracks = callback
        .storage
        .iter()
        .map(|state| {
            let mut track = original
                .track(&state.track)
                .cloned()
                .ok_or("source track")?;
            track.initial = 3;
            Ok(track)
        })
        .collect::<Result<Vec<_>, &str>>()?;
    quest::QuestStateCatalogue::new(
        original.content_revision(),
        tracks,
        vec![
            original
                .transition(&callback.key)
                .ok_or("source transition")?
                .clone(),
        ],
    )
    .map_err(debug)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let key =
        oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 2).ok_or("source key")?;
    let map = owner.bundle_map().ok_or("map")?;
    let payload = |revision| -> TestResult<Vec<u8>> {
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement,
            expected_revision: revision,
        })
        .map_err(debug)
    };
    // A real PostgreSQL lock barrier, released only once the writer reaches it.
    // No sleep or injected dispatch result substitutes for the durable path.
    let mut blocker = h.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(4444001)")
        .execute(&mut *blocker)
        .await?;
    sqlx::query("CREATE FUNCTION m44_commit_barrier() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_advisory_xact_lock(4444001); RETURN NEW; END $$")
        .execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER m44_commit_barrier BEFORE INSERT ON game_character_quest_receipts FOR EACH ROW EXECUTE FUNCTION m44_commit_barrier()")
        .execute(&h.pool).await?;
    let _clock = owner.freeze_owner_clock_for_test();
    owner
        .states
        .test_owner_time
        .store(5_000_001, std::sync::atomic::Ordering::Relaxed);
    let release = async {
        let limit = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND objid=4444001 AND NOT granted)")
                .fetch_one(&h.pool).await?;
            if waiting {
                break;
            }
            if std::time::Instant::now() >= limit {
                return Err("source never reached commit barrier".into());
            }
            tokio::task::yield_now().await;
        }
        owner
            .states
            .test_owner_time
            .store(6_000_001, std::sync::atomic::Ordering::Relaxed);
        let released_at = owner.owner_now().get();
        blocker.rollback().await?;
        Ok::<_, Box<dyn std::error::Error>>(released_at)
    };
    let use_payload = payload(0)?;
    let (result, released_at) = tokio::join!(
        super::quest_world_callbacks::wire(owner, admitted, 1, 2, use_payload.clone()),
        release
    );
    let released_at = released_at?;
    let result = result?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
        UseDisposition::Committed
    );
    let due = map
        .facts
        .next_movement_decay_time()
        .ok_or("source reset timer")?;
    assert!(
        due >= released_at + 10_000_000,
        "source must remain transformed for ten seconds after durable commit, due={due}, release={released_at}"
    );
    assert_eq!(map.facts.object_revision(key), 1);
    let replay = super::quest_world_callbacks::wire(owner, admitted, 1, 2, use_payload).await?;
    assert_eq!(result, replay);
    assert_eq!(map.facts.next_movement_decay_time(), Some(due));
    owner
        .states
        .test_owner_time
        .store(7_000_001, std::sync::atomic::Ordering::Relaxed);
    let busy = super::quest_world_callbacks::wire(owner, admitted, 2, 2, payload(1)?).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&busy).map_err(debug)?,
        UseDisposition::NothingToUse
    );
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert_eq!(map.facts.object_revision(key), 2);
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipts, 1);
    let pos = crate::map::overlay::TilePos {
        x: AT.x,
        y: AT.y,
        floor: AT.floor,
    };
    let base = map
        .overlay
        .base()
        .tile(AT.x, AT.y, AT.floor)
        .ok_or("source tile")?;
    let id = *base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(2)
        .ok_or("source ordinal")?
        .0;
    let restored = map.facts.base_entry(pos, 2, id).ok_or("restored source")?;
    assert_eq!(restored.appearance_id, 10186);
    let active = map
        .facts
        .quest_callback_replacement(key, 10187)
        .ok_or("active source")?;
    let timer = crate::map::facts::QuestCallbackProjection::TimedTransform {
        due: due + 10_000_000,
        restore: restored,
    };
    assert!(
        map.facts
            .apply_quest_callback(key, 2, active, &[], &[], false, Some(&timer))
    );
    assert!(
        map.facts
            .apply_quest_callback(key, 3, active, &[], &[], false, None)
    );
    assert_eq!(map.facts.expire_movement_items(due + 10_000_000), 0);
    assert_eq!(map.facts.object_revision(key), 4);
    assert_eq!(
        map.facts
            .base_entry(pos, 2, id)
            .ok_or("newer source")?
            .appearance_id,
        10187
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED M44 native_use commit_barrier replay busy expiry stale_timer"
    );
    Ok(())
}
