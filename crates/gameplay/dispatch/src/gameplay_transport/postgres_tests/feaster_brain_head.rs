//! Crystal 201189f data-global/scripts/quests/feaster_of_souls/actions_portal_brain_head.lua:1-126.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks::{local, scenario, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::GameSessionId;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use std::time::Duration;

pub(super) const NAME: &str = "feaster-brain-head";
pub(super) const FULL: &str = "feaster-brain-head-full";
pub(super) const CLOSED: &str = "feaster-brain-head-closed";
const ADMISSION_WINDOW_MS: u32 = 30_000;
const BOSS: &str = "oteryn:creature.brain_head";
const ADD: &str = "oteryn:creature.cerebellum";
const DOOR: Position = at(31937, 32325);
const ALIAS: Position = at(31937, 32326);
const ENTRY: Position = at(31963, 32324);
pub(super) const EXIT: Position = at(31971, 32325);

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -10 }
}

pub(super) fn binding() -> TestResult<Callback> {
    let monsters: Vec<_> = [(BOSS, at(31954, 32325))]
        .into_iter()
        .chain(
            [
                (31953, 32324),
                (31955, 32324),
                (31953, 32326),
                (31955, 32326),
                (31960, 32320),
                (31960, 32330),
                (31947, 32320),
                (31947, 32330),
            ]
            .map(|(x, y)| (ADD, at(x, y))),
        )
        .map(|(creature, position)| serde_json::json!({"creature": creature,"position":position}))
        .collect();
    Ok(serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/solo-room/brain-head",
        "quest":"oteryn:quest.feaster_of_souls_quest",
        "trigger":"step_in",
        "selector":{"position":DOOR,"position_aliases":[ALIAS],"ordinal":0},
        "requires_world_admission":true,"return_on_failure":true,"failure_teleport":EXIT,
        "success_message":"You've entered the Brain Head's lair.",
        "reversible":false,"passages":[],"offerings":[],"storage":[],
        "effects":{"encounter":{
            "minimum_level":250,"players":[{"from":DOOR,"to":ENTRY}],
            "monsters":monsters,
            "room_min":at(31941,32313),"room_max":at(31966,32337),
            "solo_expiry":{"exit":EXIT,"time_limit_ms":300000,"cooldown_seconds":36000,
                "shared":{"capacity":5,"join_window_ms":ADMISSION_WINDOW_MS}}
        }},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/feaster_of_souls/actions_portal_brain_head.lua:1-126"]
    }))?)
}

#[test]
fn brain_head_admits_two_players_through_separate_portals_then_ejects_both_at_the_deadline()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || scenario(NAME, ALIAS, false))
}

#[test]
fn brain_head_full_room_returns_step_to_occupied_exit_without_booking_cooldown() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(FULL, || scenario(FULL, ALIAS, false))
}

#[test]
fn brain_head_join_window_refuses_a_late_second_player_until_the_room_expires() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(CLOSED, || scenario(CLOSED, ALIAS, false))
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
    name: &str,
) -> TestResult {
    let full = name == FULL;
    let late = name == CLOSED;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    // A full zone refuses entry even if all occupants are spectators.
    if full {
        let mut runtime = owner.runtime.lock().await;
        let cells = [
            ENTRY,
            at(31962, 32323),
            at(31962, 32324),
            at(31962, 32325),
            at(31963, 32325),
            EXIT,
        ];
        for (index, cell) in cells.into_iter().enumerate() {
            let session = GameSessionId::decode(&id(112 + u8::try_from(index)?)).map_err(debug)?;
            let lease = runtime.reserve_fresh_session(session).map_err(debug)?;
            let spectator = runtime.commit_fresh_session(lease).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(spectator, local(cell))
                .map_err(debug)?;
        }
    }
    let map = owner.bundle_map().ok_or("room map")?;
    let cue_cursor = map
        .facts
        .movement_effects_since(0)
        .last()
        .map_or(0, |cue| cue.0);
    let result = wire(
        owner,
        admitted,
        1,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        decode_step_result(&result).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(if full { EXIT } else { ENTRY }),
    );
    let live = super::engworldchg_journals::live_creatures;
    let remaining: Option<i64> = sqlx::query_scalar(
        "SELECT available_at - floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint
         FROM game_character_boss_cooldowns
         WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(BOSS)
    .fetch_optional(&harness.pool)
    .await?;
    if full {
        let cues = map.facts.movement_effects_since(cue_cursor);
        assert!(cues.iter().any(|(_, at, effect)| {
            (at.x, at.y, at.floor, *effect) == (EXIT.x, EXIT.y, EXIT.floor, 11)
        }));
        assert!(remaining.is_none());
        assert!(live(owner, BOSS).await?.is_empty());
        assert!(live(owner, ADD).await?.is_empty());
        assert!(
            owner
                .states
                .quest_callbacks
                .next_encounter_due_us()
                .is_none()
        );
        return Ok(());
    }
    assert_eq!(live(owner, BOSS).await?.len(), 1);
    assert_eq!(live(owner, ADD).await?.len(), 8);
    assert!((35_990_000..=36_000_000).contains(&remaining.ok_or("booked cooldown")?));
    // Crystal registers every approach tile for the same visit. A second
    // independently fenced Character enters by the primary position.
    let mut second = second_player(owner, admitted, harness).await?;
    let second_actor = second.runtime_actor.ok_or("second player")?;
    if late {
        // Crystal actions_portal_brain_head.lua:81-91 seals admission after
        // 30 seconds while the original cohort continues for another 270.
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(Duration::from_millis(u64::from(ADMISSION_WINDOW_MS)))
            .ok_or("join-window clock")?;
        owner.drain_quest_encounter_deadlines().await;
    }
    let reply = wire(
        owner,
        &mut second,
        1,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(second_actor)
            .map_err(debug)?
            .position(),
        local(if late { EXIT } else { ENTRY })
    );
    assert_eq!(live(owner, BOSS).await?.len(), 1);
    assert_eq!(live(owner, ADD).await?.len(), 8);
    let cooled_players: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_boss_cooldowns WHERE boss_key=$1")
            .bind(BOSS)
            .fetch_one(&harness.pool)
            .await?;
    assert_eq!(cooled_players, if late { 1 } else { 2 });
    let due = owner
        .states
        .quest_callbacks
        .next_encounter_due_us()
        .ok_or("room deadline")?;
    let advance = Duration::from_micros(due.saturating_sub(owner.owner_now().get()));
    owner.clock_origin = owner.clock_origin.checked_sub(advance).ok_or("clock")?;
    owner.drain_quest_encounter_deadlines().await;
    {
        let runtime = owner.runtime.lock().await;
        for player in [actor, second_actor] {
            assert_eq!(
                runtime
                    .read_actor_position(player)
                    .map_err(debug)?
                    .position(),
                local(EXIT)
            );
        }
    }
    assert!(live(owner, BOSS).await?.is_empty());
    assert!(live(owner, ADD).await?.is_empty());
    Ok(())
}

async fn second_player(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    first: &AdmittedSession,
    harness: &Harness,
) -> TestResult<AdmittedSession> {
    use crate::world_party_runtime_postgres_cases::{
        FreshPlayerOwners, committed_fresh_player_with_operation,
    };

    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&harness.pool)
            .await?;
    let (current, operation) = committed_fresh_player_with_operation(
        &FreshPlayerOwners {
            root: harness.root.clone(),
            node: harness.node.clone(),
            recovery: &harness.recovery,
        },
        61,
        63,
        71,
        now,
    )
    .await?;
    let first_actor = first.runtime_actor.ok_or("first actor")?;
    let session = current.current_game_session_id();
    let (actor, digest) = {
        let mut runtime = owner.runtime.lock().await;
        let reserved = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reserved).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, current.current_character_lease())
            .map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                actor,
                local(Position {
                    x: DOOR.x - 1,
                    ..DOOR
                }),
            )
            .map_err(debug)?;
        let mut states = owner.states.spell_states.lock().await;
        let facts = states
            .get(&runtime, first_actor, first.game_session_id)
            .ok_or("first player's level-250 facts")?
            .character_facts();
        states
            .initialize(&runtime, actor, session, facts, (0, 0), owner.owner_now())
            .ok_or("second player's live facts")?;
        (actor, runtime.content_pin().server_artifact_digest())
    };
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "second player fence read")?
        .ok_or("second player fence")?
        .item_fence();
    harness
        .root
        .initialize_admission_equipment(owner.character, owner.holder, &fence, digest, &operation)
        .await
        .map_err(debug)?;

    use crate::gameplay_transport::connection::SessionContinuity;

    let mut admitted = AdmittedSession {
        game_session_id: session,
        runtime_actor: Some(actor),
        controller: None,
        item_fence: Some(fence),
        continuity: SessionContinuity {
            connection_generation: current.current_connection_generation().get(),
            selected_capabilities: first.continuity.selected_capabilities,
            ..SessionContinuity::FRESH
        },
        ..*first
    };
    owner.admit_progression_session(&mut admitted).await;
    owner.admit_quest_session(&admitted).await;
    Ok(admitted)
}
