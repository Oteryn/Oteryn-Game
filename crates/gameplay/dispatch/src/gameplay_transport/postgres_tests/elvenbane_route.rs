//! Earn Elvenbane exploration through literal source-map protocol movement.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use oteryn_gameplay_quests::quest::kill_objectives as rules;

#[test]
fn elvenbane_source_basement_and_poison_route() -> TestResult {
    super::annihilator_route::elvenbane_source_exploration()
}

pub(super) async fn exploration(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
    pool: &sqlx::PgPool,
) -> TestResult {
    let started = std::time::Instant::now();
    let version: i32 = sqlx::query_scalar("SELECT current_setting('server_version_num')::int")
        .fetch_one(pool)
        .await?;
    eprintln!("ELVENBANE_EXPLORATION POSTGRES_CASE_OBSERVED server_version_num={version}");
    owner
        .refresh_quest_session(member.game_session_id, true)
        .await;
    let fence = owner
        .current_quest_fence(member.game_session_id)
        .await
        .map_err(|_| "quest fence")?
        .ok_or("session")?;
    // Match production admission before the fixture's per-command reconnects.
    // A carried command cannot reconcile retained work without this current fence.
    member.item_fence = Some(fence.item_fence());
    super::desert_earned_route::step(
        owner,
        member,
        oteryn_protocol_oteryn::world_spatial::StepDirection::West,
    )
    .await?;
    let position = owner
        .runtime
        .lock()
        .await
        .read_actor_position(member.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    assert_eq!(
        position,
        crate::foundation::MovementLocalPosition {
            x: 32598,
            y: 31653,
            floor: 8
        },
        "the actual source hole and stairs must admit the basement without relocation"
    );
    let phase: Option<i64> = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(fence.character_id.as_bytes().as_slice()).bind(rules::PHASE).fetch_optional(pool).await?;
    assert_eq!(
        phase,
        Some(1),
        "source basement exploration must earn phase1"
    );
    let defender_phase = &oteryn_gameplay_quests::quest::native_kill_goals::load()
        .map_err(debug)?
        .iter()
        .find(|goal| goal.key.ends_with("/elvenbane-dragon"))
        .ok_or("native defender phase")?
        .phase;
    let activated: Option<i64> = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(fence.character_id.as_bytes().as_slice()).bind(defender_phase).fetch_optional(pool).await?;
    assert_eq!(
        activated,
        Some(0),
        "actual source basement entry must activate the exact native defender cohort"
    );
    let one_visit_transaction: bool = sqlx::query_scalar("SELECT count(*)=2 AND count(DISTINCT committed_character_revision)=1 AND count(DISTINCT xmin::text)=1 FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[])")
        .bind(fence.character_id.as_bytes().as_slice()).bind(vec![rules::PHASE, defender_phase.as_str()])
        .fetch_one(pool).await?;
    assert!(
        one_visit_transaction,
        "basement exploration and defender activation must be one atomic visit"
    );
    use oteryn_gameplay_quests::quest::{elvenbane, world_callbacks::Position};
    use oteryn_protocol_oteryn::world_spatial::StepDirection::North;
    // The original basement stairs return outdoors; every subsequent flat edge
    // is searched against source collision and sent through protocol STEP.
    super::desert_earned_route::step(owner, member, North).await?;
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
    let ladder = {
        let map = owner.bundle_map().ok_or("map")?;
        let key = crate::map::view::placement_key(
            crate::map::overlay::TilePos {
                x: 32598,
                y: 31651,
                floor: -7,
            },
            1,
        )
        .ok_or("source ladder")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        }
    };
    let mut stale = ladder.clone();
    stale.expected_revision += 1;
    assert_eq!(
        super::annihilator_route::use_target(owner, member, &stale).await?,
        UseDisposition::StaleState,
        "stale ladder revision cannot relocate"
    );
    let command = member.continuity.next_command_id;
    assert_eq!(
        super::annihilator_route::use_target(owner, member, &ladder).await?,
        UseDisposition::Committed,
        "source ladder USE must ascend through the movement owner"
    );
    let landed = owner
        .runtime
        .lock()
        .await
        .read_actor_position(member.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    let reply = super::quest_world_callbacks::wire(
        owner,
        member,
        command,
        2,
        oteryn_protocol_oteryn::world_object::encode_use_intent(&ladder).map_err(debug)?,
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed,
        "exact ladder command replay keeps its original outcome"
    );
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(member.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    assert_eq!(at, landed, "replay must not move or ascend a second time");
    assert_eq!(at.floor, 6, "native ladder must ascend to the wall walk");
    // The outer wall walk is disconnected from the poison room. Descend the
    // original trapdoor, cross the courtyard and use the central tower stairs.
    super::desert_earned_route::step(owner, member, North).await?;
    super::desert_earned_route::walk_to(
        owner,
        member,
        Position {
            x: 32593,
            y: 31644,
            floor: -7,
        },
    )
    .await?;
    super::desert_earned_route::step(owner, member, North).await?;
    super::desert_earned_route::walk_to(
        owner,
        member,
        Position {
            x: 32589,
            y: 31644,
            floor: -6,
        },
    )
    .await?;
    let poison: Option<i64> = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(fence.character_id.as_bytes().as_slice()).bind(elvenbane::POISON).fetch_optional(pool).await?;
    assert_eq!(
        poison,
        Some(1),
        "source poison-field traversal must earn its own prerequisite"
    );
    super::desert_earned_route::walk_to(
        owner,
        member,
        Position {
            x: 32588,
            y: 31644,
            floor: -6,
        },
    )
    .await?;
    super::desert_earned_route::step(owner, member, North).await?;
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(member.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    assert_eq!(
        at,
        crate::foundation::MovementLocalPosition {
            x: 32588,
            y: 31642,
            floor: 5
        },
        "native source stairs must reach Dragon floor without relocation"
    );
    let phase: Option<i64> = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
        .bind(fence.character_id.as_bytes().as_slice()).bind(rules::PHASE).fetch_optional(pool).await?;
    assert_eq!(
        phase,
        Some(2),
        "only earned basement and poison crossing admit the Dragon stage"
    );
    eprintln!(
        "ELVENBANE_EXPLORATION POSTGRES_CASE_COMPLETED basement=earned poison=earned dragon_floor=earned elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}
