//! Crystal 201189f dreamers_challenge_quest/movement_tower.lua:2-29,
//! data-global/startup/tables/item.lua:4220-4255.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{input_item, local, scenario, wire, world_cells};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::{
    character_progression::CurrentCharacterGameplayFence,
    quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest},
};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    world_callbacks::{self, Callback, Offering, Position},
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use std::sync::Arc;

pub(super) const NIGHTMARE: &str = "knightwatch-nightmare";
pub(super) const BROTHERHOOD: &str = "knightwatch-brotherhood";
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
const PORTAL: Position = Position {
    x: 32815,
    y: 32344,
    floor: -4,
};
const ORIGIN: Position = Position { x: 32814, ..PORTAL };
const EXIT: Position = Position {
    x: 33213,
    y: 32454,
    floor: -1,
};

pub(super) fn binding() -> TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/ranked-toll/knightwatch-3054",
        "quest":"oteryn:quest.dreamer_s_challenge_quest","trigger":"step_in",
        "selector":{"native":{"position":PORTAL,"ordinal":1,"appearance":1949}},
        "teleport":EXIT,"requires_world_admission":true,"return_on_failure":true,
        "reversible":false,"passages":[],"offerings":[],"storage":[],
        "item_on_target":{"item":PEARL,"count":1,"on_step":true},
        "effects":{"ranked_toll":{"tracks":RANKS,"minimum":4},"teleport_effect":11},
        "sources":[
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dreamers_challenge_quest/movement_tower.lua:2-29",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/startup/tables/item.lua:4220-4255"
        ]
    }))?)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    world_cells(callback, PORTAL, 1949, 1, &[], &[], None, &[])
}

pub(super) fn catalogue(
    base: &QuestStateCatalogue,
    callback: &Callback,
    name: &str,
) -> TestResult<Arc<QuestStateCatalogue>> {
    let index = usize::from(name == BROTHERHOOD);
    let base = base
        .with_fixture_transition(QuestTransition {
            key: SETUP.into(),
            quest: FACTIONS[index].into(),
            effects: vec![QuestEffect {
                track: RANKS[index].into(),
                from: QuestComparison::Eq(0),
                effect: QuestEffectKind::Set(4),
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
    super::kill_reward_e2e::scenario_thread::run(NIGHTMARE, || scenario(NIGHTMARE, PORTAL, false))
}

#[test]
fn knightwatch_step_debits_one_pearl_for_brotherhood_rank_only_after_admission() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(BROTHERHOOD, || {
        scenario(BROTHERHOOD, PORTAL, false)
    })
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
) -> TestResult {
    input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: ORIGIN,
            item: PEARL.into(),
            count: 1,
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
    for (command, expected, debits) in [(1, ORIGIN, 0), (2, EXIT, 1), (3, ORIGIN, 1)] {
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
        } else if command == 2 {
            let map = owner.bundle_map().ok_or("map")?;
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
