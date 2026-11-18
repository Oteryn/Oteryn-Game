//! Crystal 201189f creaturescripts_faceless_bane.lua:11-28 and movements_faceless_tiles.lua:25-40.
use super::fixture::{debug, id};
use super::quest_world_callbacks::{local, scenario, wire, world_cells};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{
    CharacterId, CharacterLease, ExactActorRef, GameSessionId, OwnerDamageCommand,
};
use crate::map::{overlay::TilePos, view::placement_key};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    world_callbacks::{self, Callback, Position},
};
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, UseDisposition, WorldObjectTarget, decode_use_result,
    encode_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use std::{sync::Arc, time::Duration};

pub(super) const NAME: &str = "faceless-bane-restores";
const KEY: &str = "oteryn:quest-callback/dream-courts/faceless-bane";
const QUEST: &str = "oteryn:quest.the_dream_courts_quest";
const BOSS: &str = "oteryn:creature.faceless_bane";
const SUMMONS: [&str; 3] = [
    "oteryn:creature.burster_spectre",
    "oteryn:creature.gazer_spectre",
    "oteryn:creature.ripper_spectre",
];
const LEVER: Position = at(33637, 32562);
const ENTRY: Position = at(33617, 32567);
const RELEASE: Position = at(33617, 32566);
const RELEASE_CENTER: Position = at(33617, 32563);
const OUTSIDE_RELEASE: Position = at(33606, 32563);
const RELEASE_BOUNDARY: Position = at(33607, 32573);
const EXIT: Position = Position {
    x: 33618,
    y: 32523,
    floor: -15,
};

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -13 }
}

pub(super) fn binding() -> TestResult<Callback> {
    let players: Vec<_> = (33638..=33642)
        .map(|x| serde_json::json!({"from":at(x,32562),"to":ENTRY}))
        .collect();
    Ok(serde_json::from_value(serde_json::json!({
        "key":KEY,"quest":QUEST,"trigger":"use",
        "selector":{"native":{"position":LEVER,"ordinal":2,"appearance":9110}},
        "reversible":false,"passages":[],"storage":[],"offerings":[],"return_on_failure":false,
        "effects":{
            "encounter":{"minimum_level":250,"players":players,
                "room_min":at(33607,32553),"room_max":at(33627,32570),
                "monsters":[{"position":at(33617,32561),"creature":BOSS}]},
            "threshold_rearm":{"health_percent":20,"restores":5,"summons":SUMMONS,
                "release_center":RELEASE_CENTER,"release_radius":10,
                "exit":EXIT,"time_limit_ms":900000}
        },
        "sources":[
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/bosses_levers/faceless_bane.lua:1-24",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/creaturescripts_faceless_bane.lua:11-28",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/movements_faceless_tiles.lua:11-17"
        ]
    }))?)
}

fn release_binding() -> TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/dream-courts/faceless-bane-release","quest":QUEST,"trigger":"step_in",
        "selector":{"native":{"position":RELEASE,"ordinal":1,"appearance":431}},
        "reversible":false,"passages":[],"storage":[],"offerings":[],"return_on_failure":false,
        "effects":{"release_encounter":KEY},
        "sources":[
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/movements_faceless_tiles.lua:25-40"
        ]
    }))?)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    world_cells(
        callback,
        LEVER,
        9110,
        2,
        &[(RELEASE, 431)],
        &[],
        None,
        &[EXIT, OUTSIDE_RELEASE, RELEASE_BOUNDARY],
    )
}

pub(super) fn catalogue(
    base: &QuestStateCatalogue,
    callback: &Callback,
) -> TestResult<Arc<QuestStateCatalogue>> {
    // The synthetic tile represents the shared switch lane's completed STEP.
    let document =
        serde_json::json!({"schema":1,"callbacks":[callback,&release_binding()?]}).to_string();
    Ok(Arc::new(world_callbacks::with_fixture_world_document(
        base, &document,
    )?))
}

#[test]
fn faceless_bane_think_restores_health_and_protects_until_the_room_switch_step_then_ejects_two_players()
-> TestResult {
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
            local(at(33638, 32562)),
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
            .initialize_pinned_test_position(second, local(at(33639, 32562)))
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
    let [boss] = live(owner, BOSS)
        .await?
        .try_into()
        .map_err(|_| "admitted boss")?;
    {
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
        let hp = runtime.companion_snapshot(boss).map_err(debug)?.health;
        assert_eq!(hp, 35000);
        damage(&mut runtime, boss, b"before-threshold", 28000).map_err(debug)?;
        assert_eq!(runtime.live_creature_health(boss), Some(7000));
        owner.tick_quest_encounter_thresholds(&mut runtime);
        assert_eq!(runtime.live_creature_health(boss), Some(35000));
        assert!(damage(&mut runtime, boss, b"while-immune", 1).is_err());
        owner.tick_quest_encounter_thresholds(&mut runtime);
    }
    for creature in SUMMONS {
        assert_eq!(live(owner, creature).await?.len(), 1);
    }
    {
        let mut runtime = owner.runtime.lock().await;
        let before = runtime.read_actor_position(boss).map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(boss, before, local(OUTSIDE_RELEASE))
            .map_err(debug)?;
    }
    step(owner, admitted, StepDirection::North).await?;
    {
        let mut runtime = owner.runtime.lock().await;
        assert!(damage(&mut runtime, boss, b"outside-release-square", 1).is_err());
        let before = runtime.read_actor_position(boss).map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(boss, before, local(RELEASE_BOUNDARY))
            .map_err(debug)?;
    }
    step(owner, admitted, StepDirection::South).await?;
    step(owner, admitted, StepDirection::North).await?;
    {
        let mut runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(first)
                .map_err(debug)?
                .position(),
            local(RELEASE)
        );
        damage(&mut runtime, boss, b"after-release", 1).map_err(debug)?;
        assert_eq!(runtime.live_creature_health(boss), Some(34999));
    }
    let due = owner
        .states
        .quest_callbacks
        .next_encounter_due_us()
        .ok_or("encounter deadline")?;
    let elapsed = due.saturating_sub(owner.owner_now().get());
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_micros(elapsed))
        .ok_or("clock")?;
    owner.drain_quest_encounter_deadlines().await;
    for actor in [first, second] {
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            local(EXIT)
        );
    }
    assert!(live(owner, BOSS).await?.is_empty());
    Ok(())
}

async fn step(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    direction: StepDirection,
) -> TestResult {
    let command = admitted.continuity.next_command_id;
    let result = wire(
        owner,
        admitted,
        command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(direction),
    )
    .await?;
    assert_eq!(
        decode_step_result(&result).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}

fn damage(
    runtime: &mut crate::foundation::ChannelRuntimeV1,
    boss: ExactActorRef,
    occurrence: &[u8],
    amount: i64,
) -> Result<oteryn_gameplay_entities::OwnerDamageResult, crate::foundation::CarrierError> {
    runtime.borrow_exact_actor_commit_at_us(1).commit_damage(
        boss,
        OwnerDamageCommand {
            target: BOSS.as_bytes(),
            occurrence,
            binding: occurrence,
            damage: amount,
        },
    )
}
