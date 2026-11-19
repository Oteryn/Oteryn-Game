//! Crystal 201189f data-global/scripts/quests/threatened_dreams_quest/movement_kroazur_room.lua:3-78.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{local, scenario, stateless_scenario, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::{
    character_progression::CurrentCharacterGameplayFence,
    quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest},
};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    world_callbacks::{self, Callback, Position},
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use std::{sync::Arc, time::Duration};

pub(super) const NAME: &str = "kroazur-solo-room";
pub(super) const MANUAL_EXIT: &str = "kroazur-solo-room-manual-exit";
const KEY: &str = "oteryn:quest-callback/threatened-dreams-kroazur-entry";
const QUEST: &str = "oteryn:quest.threatened_dreams_quest";
const ACCESS: &str =
    "oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/kroazur_access";
const KROAZUR: &str = "oteryn:creature.kroazur";
// Crystal movement_kroazur_room.lua:69.
const ADMISSION_TEXT: &str = "You have ten minutes to kill and loot this boss, else you will lose that chance and will be kicked out.";
// Crystal data-global/startup/tables/tile.lua:993-999 maps AID 25029 here.
const TRIGGER: Position = Position {
    x: 33619,
    y: 32304,
    floor: -9,
};
const ORIGIN: Position = Position {
    x: 33618,
    ..TRIGGER
};
const LANDING: Position = at(33591, 32315);
const EXIT_PORTAL: Position = at(33591, 32321);
pub(super) const EXIT: Position = Position {
    x: 33619,
    y: 32306,
    floor: -9,
};

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -10 }
}

pub(super) fn binding() -> TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key":KEY, "quest":QUEST, "trigger":"step_in",
        "selector":{"position":TRIGGER,"action":25029,"appearance":10840,
            "native":{"position":TRIGGER,"ordinal":1,"appearance":10840}},
        "requires_world_admission":true,
        "return_on_failure":true,"reversible":false,"passages":[],"offerings":[],
        "success_message":ADMISSION_TEXT,
        "storage":[{"track":ACCESS,"minimum":1,"maximum":1,"set":1}],
        "effects":{"magic":{"position":TRIGGER,"effect":11},"encounter":{
            "minimum_level":0,
            "players":[{"from":TRIGGER,"to":LANDING}],
            "monsters":[{"position":at(33591,32305),"creature":KROAZUR}],
            "room_min":at(33571,32285),"room_max":at(33611,32325),
            "solo_expiry":{"exit":EXIT,"time_limit_ms":600000,"cooldown_seconds":7200}
        }},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/threatened_dreams_quest/movement_kroazur_room.lua:3-78"]
    }))?)
}

pub(super) fn exit_binding() -> TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/threatened-dreams-kroazur-exit",
        "quest":QUEST,"trigger":"step_in",
        "selector":{"position":EXIT_PORTAL,"action":25029,"appearance":10840,
            "native":{"position":EXIT_PORTAL,"ordinal":1,"appearance":10840}},
        "teleport":EXIT,"return_on_failure":false,"reversible":false,
        "passages":[],"offerings":[],"storage":[],
        "effects":{"magic":{"position":EXIT_PORTAL,"effect":11},"teleport_effect":11},
        "sources":[
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/threatened_dreams_quest/movement_kroazur_room.lua:30-35",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/tile.lua:993-999"
        ]
    }))?)
}

pub(super) fn catalogue(base: &QuestStateCatalogue) -> TestResult<Arc<QuestStateCatalogue>> {
    let callback = binding()?;
    let document = serde_json::json!({"schema":1,"callbacks":[callback]}).to_string();
    let base = base
        .with_fixture_transition(world_callbacks::primary_transition(&callback)?)
        .map_err(debug)?;
    Ok(Arc::new(world_callbacks::with_fixture_world_document(
        &base, &document,
    )?))
}

#[test]
fn kroazur_solo_step_sets_personal_cooldown_and_ejects_only_at_the_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || scenario(NAME, TRIGGER, false))
}

#[test]
fn kroazur_manual_exit_step_teleports_without_an_admission_receipt() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(MANUAL_EXIT, || {
        stateless_scenario(MANUAL_EXIT, EXIT_PORTAL, EXIT)
    })
}

async fn step(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    expected: Position,
) -> TestResult {
    let reply = wire(
        owner,
        admitted,
        id,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_location(owner, admitted, expected).await
}

async fn assert_location(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    expected: Position,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actor")?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(expected)
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
) -> TestResult {
    {
        let map = owner.bundle_map().ok_or("Kroazur map")?;
        let before = map
            .facts
            .movement_effects_since(0)
            .last()
            .map_or(0, |cue| cue.0);
        step(owner, admitted, 1, ORIGIN).await?;
        // Crystal movement_kroazur_room.lua:37-41 flashes the portal first
        // and the safe return square second when the access track is unset.
        assert_eq!(
            map.facts
                .movement_effects_since(before)
                .into_iter()
                .map(|(_, at, effect)| (at.x, at.y, at.floor, effect))
                .collect::<Vec<_>>(),
            [TRIGGER, ORIGIN].map(|p| (p.x, p.y, p.floor, 11))
        );
    }
    assert!(
        super::engworldchg_journals::live_creatures(owner, KROAZUR)
            .await?
            .is_empty()
    );

    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let started = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: "oteryn:quest-transition/threatened-dreams/maelyrra-start".into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(2900).map_err(debug)?,
                )),
            },
            owner.quest_catalogue.clone().ok_or("quest catalogue")?,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(started, QuestTransitionOutcome::Committed(_)));
    drop(slot);

    let measure = || {
        sqlx::query_scalar::<_, i64>(
            "SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint",
        )
        .fetch_one(&h.pool)
    };
    let before = measure().await?;
    let map = owner.bundle_map().ok_or("Kroazur map")?;
    let effect_cursor = map.facts.movement_effects_since(0).last().map_or(0, |cue| cue.0);
    step(owner, admitted, 2, LANDING).await?;
    let effects = map.facts.movement_effects_since(effect_cursor);
    for position in [TRIGGER, LANDING] {
        assert_eq!(effects.iter().filter(|(_, at, effect)| {
            (at.x, at.y, at.floor, *effect) == (position.x, position.y, position.floor, 11)
        }).count(), 1);
    }
    let messages = owner.states.chat.drain(admitted.game_session_id);
    assert_eq!(messages.iter().filter(|line| {
        matches!(line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == ADMISSION_TEXT)
    }).count(), 1);
    assert_eq!(
        super::engworldchg_journals::live_creatures(owner, KROAZUR)
            .await?
            .len(),
        1
    );
    let cooldown: i64 = sqlx::query_scalar(
        "SELECT available_at FROM game_character_boss_cooldowns WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2"
    ).bind(fence.character_id.as_bytes().as_slice()).bind(KROAZUR).fetch_one(&h.pool).await?;
    assert!((before + 7_200_000..=measure().await? + 7_200_000).contains(&cooldown));

    owner.drain_quest_encounter_deadlines().await;
    assert_location(owner, admitted, LANDING).await?;
    let now = owner.owner_now().get();
    let due = owner
        .states
        .quest_callbacks
        .next_encounter_due_us()
        .ok_or("room deadline")?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_micros(due.saturating_sub(now)))
        .ok_or("clock")?;
    owner.drain_quest_encounter_deadlines().await;
    assert_location(owner, admitted, EXIT).await?;
    assert!(
        super::engworldchg_journals::live_creatures(owner, KROAZUR)
            .await?
            .is_empty()
    );

    let actor = admitted.runtime_actor.ok_or("actor")?;
    let map = owner.bundle_map().ok_or("map")?;
    {
        let mut runtime = owner.runtime.lock().await;
        let proof = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            owner.movement_cells(),
            actor,
            admitted.game_session_id,
            runtime.read_actor_position(actor).map_err(debug)?,
            local(ORIGIN),
            false,
            &crate::map::floor::BundleDoors(&map.facts),
        )
        .map_err(debug)?;
        runtime.commit_source_step(proof).map_err(debug)?;
    }
    let effect_cursor = map.facts.movement_effects_since(0).last().map_or(0, |cue| cue.0);
    step(owner, admitted, 3, ORIGIN).await?;
    // Crystal movement_kroazur_room.lua:45-49 uses the same ordered pair
    // of teleport cues while the personal cooldown is active.
    assert_eq!(
        map.facts
            .movement_effects_since(effect_cursor)
            .into_iter()
            .map(|(_, at, effect)| (at.x, at.y, at.floor, effect))
            .collect::<Vec<_>>(),
        [TRIGGER, ORIGIN].map(|p| (p.x, p.y, p.floor, 11))
    );
    assert!(!owner.states.chat.drain(admitted.game_session_id).iter().any(|line| {
        matches!(line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == ADMISSION_TEXT)
    }));
    assert!(
        super::engworldchg_journals::live_creatures(owner, KROAZUR)
            .await?
            .is_empty()
    );
    Ok(())
}
