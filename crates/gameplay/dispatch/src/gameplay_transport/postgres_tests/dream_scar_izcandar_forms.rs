//! Crystal 201189f actions_dreamscar_levers.lua:1-15,60-70,96-98 and creaturescripts_izcandar.lua:1-72.
use super::fixture::{debug, id};
use super::quest_world_callbacks::{local, scenario, wire, world_cells};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{
    CharacterId, CharacterLease, ExactActorRef, GameSessionId, OwnerDamageCommand,
};
use crate::map::{overlay::TilePos, view::placement_key};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, UseDisposition, WorldObjectTarget, decode_use_result,
    encode_use_intent,
};
use std::time::Duration;

pub(super) const NAME: &str = "dream-scar-izcandar-forms";
const BASE: &str = "oteryn:creature.izcandar_the_banished";
const WINTER: &str = "oteryn:creature.izcandar_champion_of_winter";
const SUMMER: &str = "oteryn:creature.izcandar_champion_of_summer";
const LEVER: Position = staging(32020);
const ENTRY: Position = room(32207, 32041);
const EAST: Position = room(32208, 32048);
const WEST: Position = room(32207, 32048);
const EXIT: Position = Position {
    x: 32210,
    y: 32035,
    floor: -13,
};

const fn staging(y: u16) -> Position {
    Position {
        x: 32208,
        y,
        floor: -13,
    }
}

const fn room(x: u16, y: u16) -> Position {
    Position { x, y, floor: -14 }
}

pub(super) fn binding() -> TestResult<Callback> {
    let players: Vec<_> = (32021..=32025)
        .map(|y| serde_json::json!({"from":staging(y),"to":ENTRY}))
        .collect();
    Ok(serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/dream-courts/izcandar",
        "quest":"oteryn:quest.the_dream_courts_quest","trigger":"use",
        "selector":{"native":{"position":LEVER,"ordinal":2,"appearance":8911}},
        "reversible":false,"passages":[],"storage":[],"offerings":[],"return_on_failure":false,
        "effects":{
            "encounter":{"minimum_level":250,"players":players,
                "room_min":room(32199,32039),"room_max":room(32229,32055),
                "monsters":[{"position":EAST,"creature":BASE}]},
            "position_forms":{"delay_seconds":[10,20],"exit":EXIT,"time_limit_ms":1800000,
                "regions":[
                    {"min":room(32208,32040),"max":room(32216,32055),"creature":WINTER},
                    {"min":room(32198,32039),"max":room(32207,32055),"creature":SUMMER}
                ]}
        },
        "sources":[
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/actions_dreamscar_levers.lua:1-15",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/creaturescripts_izcandar.lua:1-72"
        ]
    }))?)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    world_cells(callback, LEVER, 8911, 2, &[], &[], None, &[EXIT, WEST])
}

#[test]
fn izcandar_changes_forms_at_the_scheduler_deadline_and_ejects_both_players() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || scenario(NAME, LEVER, false))
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let first = admitted.runtime_actor.ok_or("first actor")?;
    let second_session = GameSessionId::decode(&id(110)).map_err(debug)?;
    let second = {
        let mut runtime = owner.runtime.lock().await;
        let map = owner.bundle_map().ok_or("map")?;
        let expected = runtime.read_actor_position(first).map_err(debug)?;
        let move_to_staging = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            owner.movement_cells(),
            first,
            admitted.game_session_id,
            expected,
            local(staging(32021)),
            false,
            &crate::map::floor::BundleDoors(&map.facts),
        )
        .map_err(debug)?;
        runtime.commit_source_step(move_to_staging).map_err(debug)?;
        let lease = runtime
            .reserve_fresh_session(second_session)
            .map_err(debug)?;
        let second = runtime.commit_fresh_session(lease).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(second, local(staging(32022)))
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                second,
                second_session,
                CharacterLease::new(CharacterId::decode(&id(101)).map_err(debug)?, 2)
                    .map_err(debug)?,
            )
            .map_err(debug)?;
        let mut states = owner.states.spell_states.lock().await;
        let facts = states
            .get(&runtime, first, admitted.game_session_id)
            .ok_or("first player")?
            .character_facts();
        states
            .initialize(
                &runtime,
                second,
                second_session,
                facts,
                (0, 0),
                owner.owner_now(),
            )
            .ok_or("second player")?;
        second
    };
    let map = owner.bundle_map().ok_or("map")?;
    let key = placement_key(
        TilePos {
            x: LEVER.x,
            y: LEVER.y,
            floor: LEVER.floor,
        },
        2,
    )
    .ok_or("lever")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    assert_eq!(
        decode_use_result(&wire(owner, admitted, 1, COMMAND_TYPE_USE_INTENT, payload).await?)
            .map_err(debug)?,
        UseDisposition::Committed
    );
    let live = super::engworldchg_journals::live_creatures;
    let [initial] = live(owner, BASE)
        .await?
        .try_into()
        .map_err(|_| "base form")?;
    let expiry = due(owner)?;
    let (health, started) = {
        let mut runtime = owner.runtime.lock().await;
        for player in [first, second] {
            assert_eq!(
                runtime
                    .read_actor_position(player)
                    .map_err(debug)?
                    .position(),
                local(ENTRY)
            );
        }
        damage(&mut runtime, initial).map_err(debug)?;
        let health = runtime
            .live_creature_health(initial)
            .ok_or("injured form")?;
        let states = owner.states.spell_states.lock().await;
        let started = owner.owner_now().get();
        owner.observe_quest_encounter_forms(&mut runtime, &states);
        (health, started)
    };
    let winter_due = due(owner)?;
    assert!((10_000_000..20_100_000).contains(&(winter_due - started)));
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(live(owner, BASE).await?, [initial]);
    assert!(live(owner, WINTER).await?.is_empty());

    advance(owner, winter_due).await?;
    let [winter] = live(owner, WINTER)
        .await?
        .try_into()
        .map_err(|_| "winter form")?;
    assert!(live(owner, BASE).await?.is_empty());
    {
        let mut runtime = owner.runtime.lock().await;
        assert!(runtime.companion_snapshot(initial).is_err());
        let before = runtime.read_actor_position(winter).map_err(debug)?;
        assert_eq!(before.position(), local(EAST));
        assert_eq!(runtime.live_creature_health(winter), Some(health));
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(winter, before, local(WEST))
            .map_err(debug)?;
        let states = owner.states.spell_states.lock().await;
        owner.observe_quest_encounter_forms(&mut runtime, &states);
    }
    let [interim] = live(owner, BASE)
        .await?
        .try_into()
        .map_err(|_| "interim base form")?;
    assert_ne!(initial, interim);
    {
        let runtime = owner.runtime.lock().await;
        assert!(runtime.companion_snapshot(winter).is_err());
        assert_eq!(runtime.live_creature_health(interim), Some(health));
    }
    advance(owner, due(owner)?).await?;
    let [summer] = live(owner, SUMMER)
        .await?
        .try_into()
        .map_err(|_| "summer form")?;
    {
        let runtime = owner.runtime.lock().await;
        assert!(runtime.companion_snapshot(interim).is_err());
        assert_eq!(
            runtime
                .read_actor_position(summer)
                .map_err(debug)?
                .position(),
            local(WEST)
        );
        assert_eq!(runtime.live_creature_health(summer), Some(health));
    }
    assert_eq!(due(owner)?, expiry);

    advance(owner, expiry).await?;
    assert!(live(owner, SUMMER).await?.is_empty());
    let runtime = owner.runtime.lock().await;
    for player in [first, second] {
        assert_eq!(
            runtime
                .read_actor_position(player)
                .map_err(debug)?
                .position(),
            local(EXIT)
        );
    }
    Ok(())
}

fn due(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<u64> {
    owner
        .states
        .quest_callbacks
        .next_encounter_due_us()
        .ok_or_else(|| "encounter deadline".into())
}

async fn advance(owner: &mut ComposedFreshAdmission<'_, '_, '_>, deadline: u64) -> TestResult {
    let elapsed = deadline.saturating_sub(owner.owner_now().get());
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_micros(elapsed))
        .ok_or("clock")?;
    owner.drain_quest_encounter_deadlines().await;
    Ok(())
}

fn damage(
    runtime: &mut crate::foundation::ChannelRuntimeV1,
    boss: ExactActorRef,
) -> Result<oteryn_gameplay_entities::OwnerDamageResult, crate::foundation::CarrierError> {
    runtime.borrow_exact_actor_commit_at_us(1).commit_damage(
        boss,
        OwnerDamageCommand {
            target: BASE.as_bytes(),
            occurrence: b"before-form",
            binding: b"before-form",
            damage: 101,
        },
    )
}
