//! The authored anniversary entrance and free return through ordinary network USE.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::*;

pub(super) const NAME: &str = "qengine-rewards-m03-use";
pub(super) const ENTRY: Position = Position {
    x: 32430,
    y: 32166,
    floor: -8,
};
const QUEST: &str = "oteryn:quest.25_years_of_tibia_quest";

#[test]
fn m03_time_machine_native_use_live_guards_replay_and_free_return_postgres() -> TestResult {
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, ENTRY, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
pub(super) fn callback() -> TestResult<Callback> {
    let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    oteryn_gameplay_quests::quest::world_callbacks::load_for(loaded.catalogue())?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/m03/25-years/time-machine")
        .ok_or_else(|| "authored anniversary callback".into())
}
pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
    native_scene(
        native,
        vec![
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z08-x126-y125.b3"
                )),
                region: (8, 126, 125),
                bounds: (32429, 32166, 32431, 32168),
                start: (32429, 32167),
                expected: vec![
                    ExpectedTile {
                        position: (32430, 32166),
                        ids: vec![355, 25098],
                        counted_entry: None,
                    },
                    ExpectedTile {
                        position: (32430, 32167),
                        ids: vec![354, 25100],
                        counted_entry: None,
                    },
                ],
            },
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z08-x130-y121.b3"
                )),
                region: (8, 130, 121),
                bounds: (33453, 31029, 33454, 31031),
                start: (33453, 31030),
                expected: vec![
                    ExpectedTile {
                        position: (33453, 31029),
                        ids: vec![15532, 25098],
                        counted_entry: None,
                    },
                    ExpectedTile {
                        position: (33453, 31030),
                        ids: vec![15534, 15620, 25100],
                        counted_entry: None,
                    },
                ],
            },
        ],
        &[],
    )
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("M03_TIME_MACHINE_USE_POSTGRES {version}");
    owner.admit_premium(id(40), admitted.game_session_id);
    assert!(
        owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current()
    );
    assert_eq!(footprint(h).await?, (0, 0));
    assert_scripted_scope(owner, admitted)?;
    facts(h, true, 5).await?;
    assert_eq!(
        use_machine(owner, admitted, 1, ENTRY).await?.0,
        UseDisposition::Rejected,
        "low level"
    );
    super::qengine_outfit_stories::earn_level(owner, admitted, authority, 20).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    facts(h, false, 5).await?;
    assert_eq!(
        use_machine(owner, admitted, 2, ENTRY).await?.0,
        UseDisposition::Rejected,
        "promotion missing with all regular blessings"
    );
    facts(h, true, 4).await?;
    assert_eq!(
        use_machine(owner, admitted, 3, ENTRY).await?.0,
        UseDisposition::Rejected,
        "four regular blessings are insufficient"
    );
    facts(h, true, 5).await?;
    let premium = owner.premium_coordinator.take();
    let denied = use_machine(owner, admitted, 4, ENTRY).await;
    owner.premium_coordinator = premium;
    assert_eq!(
        denied?.0,
        UseDisposition::Rejected,
        "Premium remains a live condition"
    );
    assert_eq!(position(owner, admitted).await?, (32429, 32167, 8));
    assert_eq!(footprint(h).await?, (0, 0));
    assert!(
        owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current()
    );
    let current = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "current entrance fence")?
        .ok_or("current entrance fence")?;
    assert_eq!(
        admitted.item_fence,
        Some(current.item_fence()),
        "the real USE carries the current item fence"
    );
    let accepted = use_machine(owner, admitted, 5, ENTRY).await?;
    assert_eq!(accepted.0, UseDisposition::Committed);
    assert_eq!(position(owner, admitted).await?, (33453, 31030, 8));
    assert_eq!(footprint(h).await?, (1, 1));
    // The successful callback advances the source object's revision. Replay the
    // original bytes, including its old expected revision, rather than reclicking.
    let replay = super::quest_world_callbacks::wire(
        owner,
        admitted,
        5,
        COMMAND_TYPE_USE_INTENT,
        accepted.2.clone(),
    )
    .await?;
    assert_eq!(
        replay, accepted.1,
        "literal USE replay must not repeat the teleport or progress"
    );
    assert_eq!(position(owner, admitted).await?, (33453, 31030, 8));
    assert_eq!(footprint(h).await?, (1, 1));
    facts(h, false, 0).await?;
    let premium = owner.premium_coordinator.take();
    let returned = use_machine(
        owner,
        admitted,
        6,
        Position {
            x: 33453,
            y: 31029,
            floor: -8,
        },
    )
    .await;
    owner.premium_coordinator = premium;
    assert_eq!(
        returned?.0,
        UseDisposition::Committed,
        "return needs neither Premium, promotion nor blessings"
    );
    assert_eq!(position(owner, admitted).await?, (32430, 32167, 8));
    assert_eq!(footprint(h).await?, (1, 2));
    let (value,completed):(i64,i64)=sqlx::query_as("SELECT (SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key='oteryn:quest-progress/chosen-source/25_years_of_tibia_quest/s1'),(SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes)").bind(QUEST).fetch_one(&h.pool).await?;
    assert_eq!(
        (value, completed),
        (1, 0),
        "return preserves entry progress without completing the quest"
    );
    Ok(())
}
// Only pre-existing eligibility is fixture data. Restore every ownership trigger
// before invoking dispatch; purchases themselves are outside the admission mechanic.
async fn facts(h: &Harness, promotion: bool, count: usize) -> TestResult {
    let catalogue =
        oteryn_gameplay_content::content::blessing_catalogue::BlessingCatalogue::load()?;
    let (_, regular) = catalogue
        .henricus_quote(20, &[])
        .ok_or("regular blessing catalogue")?;
    assert_eq!(regular.len(), 5);
    assert!(count <= regular.len());
    let mut tx = h.pool.begin().await?;
    sqlx::raw_sql("ALTER TABLE game_character_promotions DISABLE TRIGGER ALL; ALTER TABLE game_character_blessings DISABLE TRIGGER ALL").execute(&mut *tx).await?;
    sqlx::query("DELETE FROM game_character_promotions WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await?;
    if promotion {
        sqlx::query("INSERT INTO game_character_promotions(character_id,purchase_occurrence_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid)").bind(id(41).as_slice()).bind(id(91).as_slice()).execute(&mut *tx).await?;
    }
    for blessing in regular.iter().take(count) {
        sqlx::query("INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,$2,'m03-use-fixture')").bind(id(41).as_slice()).bind(&blessing.storage_key).execute(&mut *tx).await?;
    }
    sqlx::raw_sql("ALTER TABLE game_character_promotions ENABLE TRIGGER ALL; ALTER TABLE game_character_blessings ENABLE TRIGGER ALL").execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
async fn footprint(h: &Harness) -> Result<(i64, i64), sqlx::Error> {
    sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_tracks WHERE quest_key=$1),(SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1)").bind(QUEST).fetch_one(&h.pool).await
}
async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<(i32, i32, i16)> {
    let runtime = owner.runtime.lock().await;
    let p = runtime
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    Ok((p.x, p.y, p.floor))
}
async fn use_machine(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    at: Position,
) -> TestResult<(UseDisposition, Vec<u8>, Vec<u8>)> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("machine map")?;
    let key = crate::map::view::placement_key(
        crate::map::overlay::TilePos {
            x: at.x,
            y: at.y,
            floor: at.floor,
        },
        1,
    )
    .ok_or("machine key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    let input = encode_use_intent(&target).map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_USE_INTENT,
        input.clone(),
    )
    .await?;
    Ok((decode_use_result(&reply).map_err(debug)?, reply, input))
}

fn assert_scripted_scope(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    let map = owner.bundle_map().ok_or("machine map")?;
    let at = crate::map::overlay::TilePos {
        x: ENTRY.x,
        y: ENTRY.y,
        floor: ENTRY.floor,
    };
    let base = map
        .overlay
        .base()
        .tile(at.x, at.y, at.floor)
        .ok_or("machine base")?;
    let source = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(1)
        .ok_or("machine ordinal")?
        .0;
    assert!(
        map.facts.base_entry(at, 1, *source).is_some(),
        "retained machine has live map facts"
    );
    let closed = crate::map::floor::BundleDoors(map.facts.doors());
    let runtime = owner.runtime.try_lock().map_err(debug)?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    let pad = crate::foundation::MovementLocalPosition {
        x: 32430,
        y: 32167,
        floor: 8,
    };
    assert!(
        crate::map::floor::prepare_quest_callback_teleport_facing(
            &runtime,
            owner.movement_cells(),
            actor,
            admitted.game_session_id,
            expected,
            pad,
            false,
            &closed,
            None
        )
        .is_err(),
        "ordinary teleport still refuses the blocked script pad"
    );
    assert!(
        crate::map::floor::prepare_scripted_quest_teleport_facing(
            &runtime,
            owner.movement_cells(),
            actor,
            admitted.game_session_id,
            expected,
            pad,
            false,
            &closed,
            None,
            true
        )
        .is_ok(),
        "source script may use its present pad"
    );
    let absent = crate::foundation::MovementLocalPosition {
        x: 0,
        y: 0,
        floor: 8,
    };
    assert!(
        crate::map::floor::prepare_scripted_quest_teleport_facing(
            &runtime,
            owner.movement_cells(),
            actor,
            admitted.game_session_id,
            expected,
            absent,
            false,
            &closed,
            None,
            true
        )
        .is_err(),
        "a script cannot create an absent landing"
    );
    let foreign = crate::foundation::GameSessionId::decode(&id(99)).map_err(debug)?;
    assert!(
        crate::map::floor::prepare_scripted_quest_teleport_facing(
            &runtime,
            owner.movement_cells(),
            actor,
            foreign,
            expected,
            pad,
            false,
            &closed,
            None,
            true
        )
        .is_err(),
        "script policy does not bypass session fencing"
    );
    Ok(())
}
