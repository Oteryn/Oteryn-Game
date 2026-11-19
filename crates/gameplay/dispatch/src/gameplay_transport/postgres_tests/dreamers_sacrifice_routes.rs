//! Crystal 201189f movement_sacrifice_teleport.lua:8-29 and startup/tables/item.lua:3243-3262.
use super::fixture::debug;
use super::quest_world_callbacks::{local, scenario, wire, world_with_cells};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::{
    character_progression::CurrentCharacterGameplayFence,
    quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest},
};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    world_callbacks::{self, Callback, Position},
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use std::sync::Arc;

pub(super) const BROTHERHOOD: &str = "dreamers-sacrifice-brotherhood";
pub(super) const NIGHTMARE: &str = "dreamers-sacrifice-nightmare";
const SET_RANK: &str = "oteryn:quest-transition/dreamer-sacrifice-fixture/rank";

const fn at(x: u16, y: u16) -> Position {
    Position { x, y, floor: -14 }
}

#[derive(Clone, Copy)]
struct Route {
    action: u16,
    source: Position,
    fallback: Position,
    qualified: Position,
    track: &'static str,
    faction: &'static str,
}

const BROTHERHOOD_ROUTE: Route = Route {
    action: 50150,
    source: at(32788, 32227),
    fallback: at(32790, 32227),
    qualified: at(32784, 32226),
    track: "oteryn:quest-progress/quest/u7_9/brotherhood_outfits/outfits",
    faction: "oteryn:quest.brotherhood_outfits_quest",
};
const NIGHTMARE_ROUTE: Route = Route {
    action: 50149,
    source: at(32840, 32226),
    fallback: at(32844, 32228),
    qualified: at(32835, 32225),
    track: "oteryn:quest-progress/quest/u7_9/nightmare_outfits/outfits",
    faction: "oteryn:quest.nightmare_outfits_quest",
};

fn route(name: &str) -> Route {
    if name == BROTHERHOOD {
        BROTHERHOOD_ROUTE
    } else {
        NIGHTMARE_ROUTE
    }
}

fn bindings(name: &str) -> TestResult<[Callback; 2]> {
    let route = route(name);
    let make = |phase, minimum, maximum, destination| {
        // The rank belongs to the faction quest. The read-only world owner
        // checks the track under that quest's existing Character fence.
        serde_json::from_value(serde_json::json!({
            "key":format!("oteryn:quest-callback/dreamers-sacrifice-{}-{phase}", route.action),
            "quest":route.faction,
            "trigger":"step_in",
            "selector":{"position":route.source,"action":route.action,"appearance":1949},
            "reversible":false,"passages":[],"offerings":[],
            "storage":[{"track":route.track,"minimum":minimum,"maximum":maximum,
                "set":minimum,"read_only":true}],
            "teleport":destination,"return_on_failure":true,"effects":{"teleport_effect":3},
            "sources":[
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dreamers_challenge_quest/movement_sacrifice_teleport.lua:8-29",
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/item.lua:3243-3262"
            ]
        }))
    };
    Ok([
        make("fallback", -1, 1, route.fallback)?,
        make("qualified", 2, 4, route.qualified)?,
    ])
}

pub(super) fn binding(name: &str) -> TestResult<Callback> {
    let [fallback, _] = bindings(name)?;
    Ok(fallback)
}

pub(super) fn world(callback: &Callback, name: &str) -> TestResult<crate::map::boot::BundleWorld> {
    let route = route(name);
    world_with_cells(callback, route.source, &[route.qualified])
}

pub(super) fn catalogue(
    base: &QuestStateCatalogue,
    name: &str,
) -> TestResult<Arc<QuestStateCatalogue>> {
    let route = route(name);
    let base = base
        .with_fixture_transition(QuestTransition {
            key: SET_RANK.into(),
            quest: route.faction.into(),
            effects: vec![QuestEffect {
                track: route.track.into(),
                from: QuestComparison::Between(-1, 1),
                effect: QuestEffectKind::Set(2),
            }],
            world_change: None,
            completes: false,
            experience: None,
        })
        .map_err(debug)?;
    let document = serde_json::json!({"schema":1,"callbacks":bindings(name)?}).to_string();
    Ok(Arc::new(world_callbacks::with_fixture_world_document(
        &base, &document,
    )?))
}

#[test]
fn dreamers_brotherhood_sacrifice_portal_selects_both_read_only_rank_routes() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(BROTHERHOOD, || {
        scenario(BROTHERHOOD, BROTHERHOOD_ROUTE.source, false)
    })
}

#[test]
fn dreamers_nightmare_sacrifice_portal_selects_both_read_only_rank_routes() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NIGHTMARE, || {
        scenario(NIGHTMARE, NIGHTMARE_ROUTE.source, false)
    })
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    fence: CurrentCharacterGameplayFence,
    name: &str,
) -> TestResult {
    let route = route(name);
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let map = owner.bundle_map().ok_or("sacrifice map")?;
    for (command, destination) in [(1, route.fallback), (2, route.qualified)] {
        let reply = wire(
            owner,
            admitted,
            command,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(StepDirection::East),
        )
        .await?;
        assert_eq!(
            decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        let observed = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?;
        assert_eq!(observed.position(), local(destination));
        if command == 1 {
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let outcome = slot
                .commit_quest_transition(
                    owner.root,
                    owner.character,
                    owner.holder,
                    fence,
                    QuestTransitionRequest {
                        transition_key: SET_RANK.into(),
                        cause: QuestCause::Command(CommandRef::new(
                            admitted.game_session_id,
                            CommandId::new(2900).map_err(debug)?,
                        )),
                    },
                    owner.quest_catalogue.clone().ok_or("quest catalogue")?,
                )
                .await
                .map_err(debug)?;
            assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
            drop(slot);

            let mut runtime = owner.runtime.lock().await;
            let proof = crate::map::floor::prepare_quest_callback_teleport(
                &runtime,
                owner.movement_cells(),
                actor,
                admitted.game_session_id,
                observed,
                local(Position {
                    x: route.source.x - 1,
                    ..route.source
                }),
                false,
                &crate::map::floor::BundleDoors(&map.facts),
            )
            .map_err(debug)?;
            runtime.commit_source_step(proof).map_err(debug)?;
        }
    }
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(route.track), Some(&2));
    Ok(())
}
