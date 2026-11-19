//! Crystal 201189f dreamers_challenge_quest/movement_tower.lua:2-29,
//! data-global/startup/tables/item.lua:4220-4255.
use super::fixture::{Harness, debug, id};
use super::quest_world_callbacks::{input_item, local, scenario, wire, world_cells};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::{
    character_progression::CurrentCharacterGameplayFence,
    quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest},
};
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    world_callbacks::{self, Callback, Offering, Position},
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use std::sync::Arc;

pub(super) const BROTHERHOOD: &str = "knightwatch-brotherhood";
pub(super) const CARLIN: &str = "knightwatch-carlin-rank-one";
pub(super) const KAZOORDOON: &str = "knightwatch-kazoordoon-rank-two";
pub(super) const NIGHTMARE: &str = "knightwatch-nightmare";
pub(super) const RETAINED_UID: &str = "knightwatch-retained-uid";
const RANKS: [&str; 2] = [
    "oteryn:quest-progress/quest/u7_9/nightmare_outfits/outfits",
    "oteryn:quest-progress/quest/u7_9/brotherhood_outfits/outfits",
];
const FACTIONS: [&str; 2] = [
    "oteryn:quest.nightmare_outfits_quest",
    "oteryn:quest.brotherhood_outfits_quest",
];
const SETUP: &str = "oteryn:quest-transition/knightwatch-fixture/rank";
const PEARL: &str = "oteryn:item.tibia.i5021";
// Crystal movement_tower.lua:2-10 supplies the ranks and destinations.
// data-global/startup/tables/item.lua:4220-4255 supplies the retained sources.
const ROUTES: [(u16, Position, Position, i64); 9] = [
    (3048, at(32822, 32344, -6), at(32360, 31782, -7), 1),
    (3049, at(32815, 32344, -6), at(32369, 32241, -7), 1),
    (3050, at(32818, 32347, -6), at(32750, 32344, -14), 1),
    (3051, at(32818, 32347, -5), at(32649, 31925, -11), 2),
    (3052, at(32815, 32344, -5), at(32732, 31634, -7), 2),
    (3053, at(32822, 32344, -5), at(32181, 32436, -7), 2),
    (3054, at(32815, 32344, -4), at(33213, 32454, -1), 4),
    (3055, at(32822, 32344, -4), at(33194, 32853, -8), 4),
    (3056, at(32819, 32347, -4), at(32417, 32139, -15), 4),
];

const fn at(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

fn portal(name: &str) -> TestResult<(u16, Position, Position, i64)> {
    let uid = match name {
        CARLIN => 3048,
        KAZOORDOON => 3051,
        BROTHERHOOD | NIGHTMARE | RETAINED_UID => 3054,
        _ => return Err("Knightwatch test route".into()),
    };
    ROUTES.iter().copied().find(|route| route.0 == uid).ok_or_else(|| "Knightwatch UID".into())
}

pub(super) fn binding(name: &str) -> TestResult<Callback> {
    let (uid, source, exit, rank) = portal(name)?;
    let selector = if matches!(name, BROTHERHOOD | NIGHTMARE) {
        serde_json::json!({"native":{"position":source,"ordinal":1,"appearance":1949}})
    } else {
        serde_json::json!({"unique":uid,"position":source,"appearance":1949})
    };
    Ok(serde_json::from_value(serde_json::json!({
        "key":format!("oteryn:quest-callback/ranked-toll/knightwatch-{uid}"),
        "quest":"oteryn:quest.dreamer_s_challenge_quest","trigger":"step_in",
        "selector":selector,
        "teleport":exit,"requires_world_admission":true,"return_on_failure":true,
        "reversible":false,"passages":[],"offerings":[],"storage":[],
        "item_on_target":{"item":PEARL,"count":1,"on_step":true},
        "effects":{"ranked_toll":{"tracks":RANKS,"minimum":rank},"teleport_effect":11},
        "sources":[
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dreamers_challenge_quest/movement_tower.lua:2-29",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/item.lua:4220-4255"
        ]
    }))?)
}

pub(super) fn world(callback: &Callback, name: &str) -> TestResult<crate::map::boot::BundleWorld> {
    world_cells(callback, portal(name)?.1, 1949, 1, &[], &[], None, &[])
}

pub(super) fn catalogue(
    base: &QuestStateCatalogue,
    callback: &Callback,
    name: &str,
) -> TestResult<Arc<QuestStateCatalogue>> {
    let index = usize::from(matches!(name, BROTHERHOOD | KAZOORDOON));
    let rank = portal(name)?.3;
    let base = base
        .with_fixture_transition(QuestTransition {
            key: SETUP.into(),
            quest: FACTIONS[index].into(),
            effects: vec![QuestEffect {
                track: RANKS[index].into(),
                from: QuestComparison::Eq(0),
                effect: QuestEffectKind::Set(rank),
            }],
            world_change: None,
            completes: false,
            experience: None,
        })
        .map_err(debug)?;
    let base = base
        .with_fixture_transition(world_callbacks::primary_transition(callback)?)
        .map_err(debug)?;
    let document = serde_json::json!({"schema":1,"callbacks":[callback]}).to_string();
    Ok(Arc::new(world_callbacks::with_fixture_world_document(
        &base, &document,
    )?))
}

#[test]
fn knightwatch_step_debits_one_pearl_for_nightmare_rank_only_after_admission() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    tower_scenario(NIGHTMARE)
}

#[test]
fn knightwatch_step_debits_one_pearl_for_brotherhood_rank_only_after_admission() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    tower_scenario(BROTHERHOOD)
}

#[test]
fn knightwatch_retained_portal_uid_debits_once_for_an_eligible_faction() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    tower_scenario(RETAINED_UID)
}

#[test]
fn knightwatch_carlin_takes_one_pearl_on_each_paid_rank_one_step() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    tower_scenario(CARLIN)
}

#[test]
fn knightwatch_kazoordoon_charges_a_rank_two_brotherhood_member_only_when_the_landing_is_free()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    tower_scenario(KAZOORDOON)
}

fn tower_scenario(name: &'static str) -> TestResult {
    let (_, source, _, _) = portal(name)?;
    super::kill_reward_e2e::scenario_thread::run(name, move || scenario(name, source, false))
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
    name: &str,
) -> TestResult {
    let (_, source, exit, _) = portal(name)?;
    let origin = at(source.x - 1, source.y, source.floor);
    let two_pearls = name == CARLIN;
    input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: origin,
            item: PEARL.into(),
            count: if two_pearls { 2 } else { 1 },
        },
        11,
        true,
    )
    .await?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let spent = || {
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key=$1",
        )
        .bind(&callback.key)
        .fetch_one(&harness.pool)
    };
    let steps = if two_pearls {
        &[(1, origin, 0), (2, exit, 1), (3, exit, 2), (4, origin, 2)][..]
    } else if name == KAZOORDOON {
        &[(1, origin, 0), (2, origin, 0), (3, exit, 1), (4, origin, 1)][..]
    } else {
        &[(1, origin, 0), (2, exit, 1), (3, origin, 1)][..]
    };
    let mut blocker = None;
    for &(command, expected, debits) in steps {
        let result = wire(
            owner,
            admitted,
            command,
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
            local(expected)
        );
        assert_eq!(spent().await?, debits);
        if command == 1 {
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let outcome = slot
                .commit_quest_transition(
                    owner.root,
                    owner.character,
                    owner.holder,
                    fence,
                    QuestTransitionRequest {
                        transition_key: SETUP.into(),
                        cause: QuestCause::Command(CommandRef::new(
                            admitted.game_session_id,
                            CommandId::new(3200).map_err(debug)?,
                        )),
                    },
                    owner.quest_catalogue.clone().ok_or("catalogue")?,
                )
                .await
                .map_err(debug)?;
            assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
            drop(slot);
            owner
                .refresh_quest_session(admitted.game_session_id, false)
                .await;
            if name == KAZOORDOON {
                let mut runtime = owner.runtime.lock().await;
                let session = GameSessionId::decode(&id(110)).map_err(debug)?;
                let ticket = runtime.reserve_fresh_session(session).map_err(debug)?;
                let occupying = runtime.commit_fresh_session(ticket).map_err(debug)?;
                runtime.initialize_pinned_test_position(occupying, local(exit)).map_err(debug)?;
                blocker = Some(occupying);
            }
        } else if let Some(occupying) = blocker.take() {
            owner.runtime.lock().await.remove_test_actor(occupying).map_err(debug)?;
        } else if expected == exit {
            let map = owner.bundle_map().ok_or("map")?;
            let mut runtime = owner.runtime.lock().await;
            let proof = crate::map::floor::prepare_quest_callback_teleport(
                &runtime,
                owner.movement_cells(),
                actor,
                admitted.game_session_id,
                runtime.read_actor_position(actor).map_err(debug)?,
                local(origin),
                false,
                &crate::map::floor::BundleDoors(&map.facts),
            )
            .map_err(debug)?;
            runtime.commit_source_step(proof).map_err(debug)?;
        }
    }
    let qty: i64 = sqlx::query_scalar(
        "SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances
         WHERE definition_production_key=$1 AND lifecycle=1",
    )
    .bind(PEARL)
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(qty, 0);
    Ok(())
}
