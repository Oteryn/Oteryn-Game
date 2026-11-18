//! Crystal 201189f the_dream_courts/actions_strange_bucket.lua:16-58.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{input_item, scenario, world_cells};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::{
    character_progression::CurrentCharacterGameplayFence,
    quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest},
};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::{overlay::TilePos, view::placement_key};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    world_callbacks::{self, Callback, Offering, Position, RollToolStage},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use std::sync::Arc;

pub(super) const FILLED: &str = "haunted-cellar-ectoplasm-filled";
pub(super) const SPENT: &str = "haunted-cellar-ectoplasm-spent";
const QUEST: &str = "oteryn:quest.the_dream_courts_quest";
const STARTED: &str =
    "oteryn:quest-progress/crystalserver/quest/u12_00/the_dream_courts/haunted_house/questline";
const BEGIN: &str = "oteryn:quest-transition/haunted-cellar-fixture/start";
const USE: &str = "oteryn:quest-callback/haunted-cellar-egg/32722-32231-z9";
// Crystal world.otbm, first top-level egg in the Haunted Cellar approach.
const EGG: Position = Position {
    x: 32722,
    y: 32231,
    floor: -9,
};
// The fixture uses content-admitted portable forms. Production uses
// i29310, i29307, i29308 and i29309 in the same order.
const EMPTY: &str = "oteryn:item.tibia.i2874";
const PARTIAL: &str = "oteryn:item.tibia.i3585";

pub(super) fn binding() -> TestResult<Callback> {
    let stage = RollToolStage {
        source: EMPTY.into(),
        output: PARTIAL.into(),
        text: "There is not enough ectoplasm left to fill it in the bucket.".into(),
    };
    Ok(serde_json::from_value(serde_json::json!({
        "key":USE,"quest":QUEST,"trigger":"use",
        "selector":{"native":{"position":EGG,"ordinal":1,"appearance":29305}},
        "reversible":false,"passages":[],"offerings":[],"return_on_failure":false,
        "requires_world_admission":true,
        "item_on_target":{
            "item":EMPTY,"alternatives":[PARTIAL],"count":1,
            "reusable":true,"selected_instance":true
        },
        "storage":[{"track":STARTED,"minimum":1,"maximum":6,"set":1,"preserve":true}],
        "effects":{"transform":233,"roll_tool":{"outcomes":11,"minimum_success":5,
            "success_effect":21,"failure_effect":3,"stages":[stage]}},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/actions_strange_bucket.lua:16-58"]
    }))?)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    world_cells(callback, EGG, 29305, 1, &[], &[233], None, &[])
}

pub(super) fn catalogue(
    base: &QuestStateCatalogue,
    callback: &Callback,
) -> TestResult<Arc<QuestStateCatalogue>> {
    let base = base
        .with_fixture_transition(QuestTransition {
            key: BEGIN.into(),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track: STARTED.into(),
                from: QuestComparison::Eq(0),
                effect: QuestEffectKind::Set(1),
            }],
            completes: false,
            experience: None,
            world_change: None,
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
fn haunted_cellar_success_roll_changes_the_exact_carried_bucket_and_spends_the_egg() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(FILLED, || scenario(FILLED, EGG, false))
}

#[test]
fn haunted_cellar_failure_roll_keeps_the_exact_carried_bucket_and_spends_the_egg() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(SPENT, || scenario(SPENT, EGG, false))
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
    won: bool,
) -> TestResult {
    input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: EGG,
            item: EMPTY.into(),
            count: 1,
        },
        39,
        true,
    )
    .await?;
    let original: String = sqlx::query_scalar(
        "SELECT item_instance_id::text FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1",
    ).bind(EMPTY).fetch_one(&harness.pool).await?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: BEGIN.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(2920).map_err(debug)?,
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
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("USE capabilities")?;
    let map = owner.bundle_map().ok_or("map")?;
    let tile = TilePos {
        x: EGG.x,
        y: EGG.y,
        floor: EGG.floor,
    };
    let source = placement_key(tile, 1).ok_or("egg placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&source.to_be_bytes());
    let definition = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .ok_or("item index")?
        .definition_ref("Item", EMPTY, "definition-r1")
        .ok_or("empty bucket definition")?;
    let payload = |expected_revision| {
        encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::MIN),
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement: placement.clone(),
                expected_revision,
            })),
            use_with: None,
            use_with_position: None,
        })
        .map_err(debug)
    };
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let revision = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .revision();
    let first = admitted.continuity.next_command_id;
    let roll = callback.effects.roll_tool.as_ref().ok_or("roll tool")?;
    let command = (first..first + 64)
        .find(|ordinal| {
            roll.succeeds(
                USE,
                admitted.game_session_id.as_bytes(),
                *ordinal,
                &actor.placement_identity(),
                revision,
            ) == won
        })
        .ok_or("fixture roll occurrence")?;
    for ordinal in first..=command {
        let expected_revision = u64::from(ordinal != command);
        let (next, result, _) = super::rust_lottery::exchange_latest_source(
            owner,
            *admitted,
            ordinal,
            &payload(expected_revision)?,
            Some(definition),
        )
        .await?;
        *admitted = next;
        assert_eq!(result == UseDisposition::Committed, ordinal == command);
    }
    let map_tile = map
        .overlay
        .base()
        .tile(tile.x, tile.y, tile.floor)
        .ok_or("egg cell")?;
    let base_id = map_tile
        .ids()
        .iter()
        .zip(map_tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(1)
        .map(|(id, _)| *id)
        .ok_or("egg source")?;
    assert_eq!(
        map.facts
            .base_entry(tile, 1, base_id)
            .ok_or("egg view")?
            .appearance_id,
        233
    );
    let current: String = sqlx::query_scalar(
        "SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=$1::uuid AND lifecycle=1",
    ).bind(original).fetch_one(&harness.pool).await?;
    assert_eq!(current, if won { PARTIAL } else { EMPTY });
    let proofs: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key=$1),
                (SELECT count(*) FROM game_quest_world_tool_reads WHERE transition_key=$1)",
    )
    .bind(USE)
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(proofs, (i64::from(won), i64::from(!won)));
    Ok(())
}
