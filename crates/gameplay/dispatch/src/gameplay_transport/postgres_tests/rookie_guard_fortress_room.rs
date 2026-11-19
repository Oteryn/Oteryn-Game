//! Crystal 201189f8 data-global/scripts/quests/the_rookie_guard/mission12_into_fortress.lua:513-647.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{local, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Position};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack};
use oteryn_protocol_oteryn::attack::{
    AttackIntentDisposition, encode_attack_intent_result, encode_attack_target_intent,
};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, encode_step_intent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

pub(super) const NAME: &str = "rookie-guard-fortress-room";
// Crystal 201189f8 data-global/startup/tables/item.lua:4877-4891.
const PORTAL: u16 = 1959;
const ENTRY: Position = Position {
    x: 31948,
    y: 32174,
    floor: -10,
};
const LANDING: Position = Position {
    x: 31944,
    y: 32174,
    floor: -10,
};
const TIMEOUT: Position = Position {
    x: 31980,
    y: 32173,
    floor: -10,
};
const EXIT_TILE: Position = Position {
    x: 31945,
    y: 32174,
    floor: -10,
};
const EXIT_LANDING: Position = Position {
    x: 31954,
    ..ENTRY
};
const TREASURE_TILE: Position = Position {
    x: 31929,
    y: 32171,
    floor: -10,
};
const TREASURE_APPROACH: Position = Position {
    y: 32172,
    ..TREASURE_TILE
};
const TREASURE: Position = Position {
    x: 31932,
    y: 32171,
    floor: -11,
};
const BOSS: Position = Position { x: 31930, y: 32170, floor: -10 };
const BOSS_APPROACH: Position = Position { x: BOSS.x + 1, ..BOSS };
const QUEST: &str = "oteryn:quest.the_rookie_guard_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission12";
const ENTER: &str = "oteryn:quest-callback/rookie-guard-fortress-room";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_rookie_guard/mission12_into_fortress.lua:513-647";
const PORTALS_SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/item.lua:4877-4891";

#[test]
fn fortress_room_rejects_a_second_player_and_retires_exact_boss_on_exit_or_timeout() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, ENTRY, false)
    })
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks::world_cells(
        callback,
        ENTRY,
        PORTAL,
        1,
        &[
            (EXIT_TILE, PORTAL, oteryn_world_bundle::sector::Attrs {
                unique: Some(40071),
                ..Default::default()
            }),
            (TREASURE_TILE, PORTAL, oteryn_world_bundle::sector::Attrs {
                unique: Some(40072),
                ..Default::default()
            }),
        ],
        &[],
        None,
        &[TREASURE, TREASURE_APPROACH],
    )
}

fn document() -> serde_json::Value {
    use serde_json::json;
    json!({"schema":1,"callbacks":[
        {"key":ENTER, "quest":QUEST, "trigger":"step_in",
         "selector":{"position":ENTRY,"appearance":PORTAL,"unique":40070},"requires_world_admission":true,
         "passages":[],"offerings":[],"reversible":false,"return_on_failure":true,
         "storage":[{"track":TRACK,"minimum":11,"maximum":15,"set":12}],
         "success_message":"You're entering Kraknaknork's throne room. You have 5 minutes to kill him!",
         "failure_message":"A player is already inside the boss room.",
         "effects":{
             "magic":{"position":ENTRY,"effect":11},
             "encounter":{"minimum_level":1,"players":[{"from":ENTRY,"to":LANDING}],
                 "monsters":[{"creature":"oteryn:creature.kraknaknork",
                              "position":BOSS}],
                 "room_min":{"x":31929,"y":32165,"floor":-10},
                 "room_max":{"x":31945,"y":32175,"floor":-10}},
             "encounter_timeout":{"duration_ms":300000,"landing":TIMEOUT,"heal":true,
                 "message":"With his last energy, Kraknaknork pushes you out of his throne room. Hurry back and defeat him before he regains his power."}},
         "sources":[SOURCE,PORTALS_SOURCE]},
        {"key":"oteryn:quest-callback/rookie-guard-fortress-room-exit","quest":QUEST,
         "trigger":"step_in","selector":{"position":EXIT_TILE,"appearance":PORTAL,"unique":40071},
         "storage":[],"passages":[],"offerings":[],"reversible":false,"return_on_failure":false,
         "teleport":EXIT_LANDING,"success_message":"You backed out of the fight. You may try again at any time.",
         "effects":{"encounter_exit":ENTER,"magic":{"position":EXIT_TILE,"effect":11},"teleport_effect":11},
         "sources":[SOURCE,PORTALS_SOURCE]},
        {"key":"oteryn:quest-callback/rookie-guard-fortress-treasure","quest":QUEST,
         "trigger":"step_in","selector":{"position":TREASURE_TILE,"appearance":PORTAL,"unique":40072},
         "requires_world_admission":true, "storage":[{"track":TRACK,"minimum":12,"maximum":12,"set":13}],
         "passages":[],"offerings":[],"reversible":false,"return_on_failure":true,
         "teleport":TREASURE,"failure_message":"You may not use this teleporter yet.",
         "effects":{"magic":{"position":TREASURE_TILE,"effect":11},"teleport_effect":11,
             "clear_area":{"minimum":{"x":31929,"y":32165,"floor":-10},
              "maximum":{"x":31945,"y":32175,"floor":-10},"creatures":["oteryn:creature.kraknaknork"]}},
         "sources":[SOURCE,PORTALS_SOURCE]}
    ]})
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(document()["callbacks"][0].clone())?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let fixture = QuestStateCatalogue::new(
        "content-1",
        vec![QuestTrack {
            key: TRACK.into(),
            quest: QUEST.into(),
            initial: 11,
            min: -1,
            max: 15,
        }],
        vec![],
    )
    .map_err(debug)?;
    let document = document();
    owner.quest_catalogue = Some(std::sync::Arc::new(world_callbacks::fixture_catalogue(
        fixture,
        &document.to_string(),
    )?));
    owner
        .states
        .quest_callbacks
        .fixture
        .set(serde_json::from_value(document["callbacks"].clone())?)
        .map_err(|_| "callback fixture already installed")?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;

    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native gameplay")?;
    let player = admitted.runtime_actor.ok_or("actor")?;
    let player_session = admitted.game_session_id;
    let origin = {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .install_companion_policies(native.policies().clone())
            .map_err(debug)?;
        runtime
            .read_actor_position(player)
            .map_err(debug)?
            .position()
    };
    let peer_approach = Position {
        x: ENTRY.x + 1,
        ..ENTRY
    };
    let peer_outside = local(peer_approach);
    let mut second = super::scenarios::peer_at(h, owner, *admitted, peer_outside).await?;
    let peer = second.runtime_actor.ok_or("second actor")?;
    let peer_session = second.game_session_id;
    owner.admit_quest_session(&second).await;

    let east = encode_step_intent(StepDirection::East);
    let west = encode_step_intent(StepDirection::West);
    wire(owner, admitted, 1, 1, east.clone()).await?;
    let effects = owner.bundle_map().ok_or("map")?.facts.movement_effects_since(0);
    for expected in [ENTRY, LANDING] {
        assert!(
            effects.iter().any(|(_, position, effect)| {
                *effect == 11 && (position.x, position.y, position.floor)
                    == (expected.x, expected.y, expected.floor)
            }),
            "the committed encounter signals both the portal and the room"
        );
    }
    advance_clock(owner, 2)?;
    wire(owner, &mut second, 1, 1, west.clone()).await?;
    {
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(player)
                .map_err(debug)?
                .position(),
            local(LANDING)
        );
        assert_eq!(
            runtime.read_actor_position(peer).map_err(debug)?.position(),
            peer_outside,
            "the second player's StepIn is refused while the room is occupied"
        );
        assert_eq!(
            runtime.current_live_creature_census().map_err(debug)?.len(),
            1
        );
    }
    relocate(owner, player, player_session, TREASURE_APPROACH).await?;
    advance_clock(owner, 2)?;
    let north = encode_step_intent(StepDirection::North);
    wire(owner, admitted, 2, 1, north.clone()).await?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(player)
            .map_err(debug)?
            .position(),
        local(TREASURE_APPROACH),
        "the live boss blocks the treasure teleporter"
    );
    relocate(owner, player, player_session, LANDING).await?;
    advance_clock(owner, 2)?;
    wire(owner, admitted, 3, 1, east.clone()).await?;
    assert_eq!(boss_retirement_cues(owner)?, 1);
    {
        let mut runtime = owner.runtime.lock().await;
        let current = runtime.read_actor_position(player).map_err(debug)?;
        assert_eq!(current.position(), local(EXIT_LANDING));
        assert_eq!(runtime.read_actor_position(peer).map_err(debug)?.position(), peer_outside);
        assert!(
            runtime
                .current_live_creature_census()
                .map_err(debug)?
                .is_empty()
        );
        let map = owner.bundle_map().ok_or("map")?;
        let doors = crate::map::floor::BundleDoors(map.facts.doors());
        let proof = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            owner.movement_cells(),
            player,
            player_session,
            current,
            origin,
            false,
            &doors,
        )
        .map_err(debug)?;
        runtime.commit_source_step(proof).map_err(debug)?;
    }
    advance_clock(owner, 2)?;
    wire(owner, &mut second, 2, 1, west.clone()).await?;
    {
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime.read_actor_position(peer).map_err(debug)?.position(),
            local(LANDING),
            "the second player's StepIn succeeds after the first player exits"
        );
        assert_eq!(runtime.current_live_creature_census().map_err(debug)?.len(), 1);
    }
    advance_clock(owner, 2)?;
    wire(owner, &mut second, 3, 1, east.clone()).await?;
    assert_eq!(boss_retirement_cues(owner)?, 2);
    {
        let runtime = owner.runtime.lock().await;
        assert_eq!(runtime.read_actor_position(peer).map_err(debug)?.position(), local(EXIT_LANDING));
        assert_eq!(runtime.read_actor_position(player).map_err(debug)?.position(), origin);
        assert!(runtime.current_live_creature_census().map_err(debug)?.is_empty());
    }
    relocate(owner, peer, peer_session, peer_approach).await?;
    advance_clock(owner, 2)?;
    wire(owner, admitted, 4, 1, east.clone()).await?;
    let (base_health, injured_health) = {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let state = states.get_mut(&runtime, player, player_session).ok_or("combat player")?;
        let base = state.base_max_health();
        state.apply_wheel_maximum_bonus(20, 0).map_err(debug)?;
        state.health = base.checked_sub(10).ok_or("combat base health")?;
        (base, state.health)
    };
    // Crystal mission12_into_fortress.lua:575 leaves the fight active for five minutes.
    advance_clock(owner, 290)?;
    owner.drain_quest_encounter_deadlines().await;
    {
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime.read_actor_position(player).map_err(debug)?.position(),
            local(LANDING),
            "the timeout keeps the admitted player in the room before the deadline"
        );
        assert_eq!(runtime.current_live_creature_census().map_err(debug)?.len(), 1);
        let states = owner.states.spell_states.lock().await;
        let state = states.get(&runtime, player, player_session).ok_or("combat player")?;
        assert_eq!(state.health, injured_health);
    }
    advance_clock(owner, 11)?;
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(boss_retirement_cues(owner)?, 3);
    let mut runtime = owner.runtime.lock().await;
    let timed_out = runtime.read_actor_position(player).map_err(debug)?;
    assert_eq!(timed_out.position(), local(TIMEOUT));
    {
        let states = owner.states.spell_states.lock().await;
        let state = states.get(&runtime, player, player_session).ok_or("combat player")?;
        assert_eq!(state.facts.max_health, base_health + 20);
        assert_eq!(state.health, base_health);
    }
    assert_eq!(
        runtime.read_actor_position(peer).map_err(debug)?.position(),
        peer_outside
    );
    assert!(
        runtime
            .current_live_creature_census()
            .map_err(debug)?
            .is_empty()
    );
    let map = owner.bundle_map().ok_or("map")?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        owner.movement_cells(),
        player,
        player_session,
        timed_out,
        origin,
        false,
        &doors,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    drop(runtime);
    advance_clock(owner, 2)?;
    wire(owner, admitted, 5, 1, east).await?;
    let fence = owner.current_quest_fence(player_session).await
        .map_err(|()| "combat character fence")?.ok_or("combat character fence")?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT(character_id) DO NOTHING")
        .bind(fence.character_id.as_bytes().as_slice())
        .execute(&h.pool).await?;
    defeat_boss(owner, admitted, player).await?;
    // Crystal mission12_into_fortress.lua:559-562 cancels the timer on death.
    advance_clock(owner, 301)?;
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(
        owner.runtime.lock().await.read_actor_position(player).map_err(debug)?.position(),
        local(TREASURE_APPROACH),
        "defeat cancels automatic ejection while the player is still in the boss room"
    );
    wire(owner, admitted, 7, 1, north).await?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(player)
            .map_err(debug)?
            .position(),
        local(TREASURE),
        "defeat unlocks the treasure"
    );
    let state: i64 = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE track_key=$1 AND character_id=encode($2,'hex')::uuid",
    )
            .bind(TRACK)
            .bind(fence.character_id.as_bytes().as_slice())
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(state, 13);
    Ok(())
}

fn boss_retirement_cues(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<usize> {
    let map = owner.bundle_map().ok_or("map")?;
    Ok(map
        .facts
        .movement_effects_since(0)
        .iter()
        .filter(|(_, at, effect)| {
            *effect == 3 && (at.x, at.y, at.floor) == (BOSS.x, BOSS.y, BOSS.floor)
        })
        .count())
}

async fn defeat_boss(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    player: ExactActorRef,
) -> TestResult {
    let session = admitted.game_session_id;
    let (boss, target) = {
        let runtime = owner.runtime.lock().await;
        let visible = runtime.visible_entities();
        let [boss] = visible.creatures.as_slice() else {
            return Err("one live boss expected".into());
        };
        assert_eq!(runtime.read_actor_position(boss.actor).map_err(debug)?.position(), local(BOSS));
        let mut states = owner.states.spell_states.lock().await;
        // Bound the case with the same high-level attacker fixture as the Asura room.
        states.get_mut(&runtime, player, session).ok_or("combat player")?.facts.level = 10_000;
        (boss.actor, EntityRef {
            identity: boss.actor.placement_identity(),
            generation: boss.generation,
        })
    };
    relocate(owner, player, session, BOSS_APPROACH).await?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::PRODUCTION_OFFERED_CAPABILITIES,
            &[6, 17],
        ).ok_or("combat capabilities")?;
    let payload = encode_attack_target_intent(Some(&target)).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    assert_eq!(
        wire(owner, admitted, command, 11, payload).await?,
        encode_attack_intent_result(AttackIntentDisposition::Ok),
    );
    let mut defeated = false;
    for _ in 0..64 {
        owner.refresh_attack_weapon(player, session).await;
        owner.drain_auto_attacks().await;
        owner.drain_kill_rewards(session).await;
        if !owner.runtime.lock().await.contains_live_creature(boss) {
            defeated = true;
            break;
        }
        advance_clock(owner, 2)?;
    }
    assert!(defeated, "ordinary combat must defeat the room's exact boss");
    relocate(owner, player, session, TREASURE_APPROACH).await
}

async fn relocate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    player: ExactActorRef,
    session: GameSessionId,
    destination: Position,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    let current = runtime.read_actor_position(player).map_err(debug)?;
    let map = owner.bundle_map().ok_or("map")?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime, owner.movement_cells(), player, session, current, local(destination), false, &doors,
    ).map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}

fn advance_clock(owner: &mut ComposedFreshAdmission<'_, '_, '_>, seconds: u64) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(seconds))
        .ok_or("clock underflow")?;
    Ok(())
}
