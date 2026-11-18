//! Crystal 201189f data-global/scripts/quests/the_dream_courts/movements_spider_summon.lua:16-31.
use super::TestResult;
use super::fixture::debug;
use super::quest_world_callbacks::{scenario, wire};
use crate::durability::{
    character_progression::CurrentCharacterGameplayFence,
    quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest},
};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::{AdmittedSession, ComposedFreshAdmission};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, world_callbacks::{self, Callback, Position}};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result, encode_step_intent,
};
use std::sync::Arc;

pub(super) const NAME: &str = "lucifuga-bait-spawn";
const QUEST: &str = "oteryn:quest.the_dream_courts_quest";
const BAIT: &str = "oteryn:quest-progress/crystalserver/quest/u12_00/the_dream_courts/unsafe_release/has_bait";
const BAIT_TRANSITION: &str = "oteryn:quest-transition/crystalserver/quest/u12_00/the_dream_courts/unsafe_release/has_bait/npc_2";
const SPIDER: &str = "oteryn:creature.lucifuga_aranea";
const TRIGGER: Position = Position { x: 33672, y: 32228, floor: -7 };

pub(super) fn binding() -> TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key":"oteryn:quest-callback/dream-courts/lucifuga-bait",
        "quest":QUEST,
        "trigger":"step_in",
        "selector":{"position":TRIGGER,"action":23120,"appearance":100},
        "requires_world_admission":true,
        "reversible":false,"passages":[],"offerings":[],
        "storage":[{"track":BAIT,"minimum":1,"maximum":1,"set":1,"read_only":true}],
        "effects":{"step_spawn":{"creature":SPIDER,"rearm_step_ms":60000,"rearm_steps":10}},
        "return_on_failure":false,
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/movements_spider_summon.lua:16-31"]
    }))?)
}

pub(super) fn catalogue(base: &QuestStateCatalogue) -> TestResult<Arc<QuestStateCatalogue>> {
    let document = serde_json::json!({"schema":1,"callbacks":[binding()?]}).to_string();
    Ok(Arc::new(world_callbacks::with_fixture_world_document(base, &document)?))
}

#[test]
fn lucifuga_requires_bait_and_rearms_its_exact_tile_after_the_donor_window() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || scenario(NAME, TRIGGER, false))
}

async fn step(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    direction: StepDirection,
) -> TestResult {
    let reply = wire(owner, admitted, command, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, encode_step_intent(direction)).await?;
    assert_eq!(decode_step_result(&reply).map_err(debug)?, StepDisposition::Moved);
    Ok(())
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    fence: CurrentCharacterGameplayFence,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let key = crate::map::view::placement_key(crate::map::overlay::TilePos {
        x: TRIGGER.x, y: TRIGGER.y, floor: TRIGGER.floor,
    }, 1).ok_or("source tile")?;
    let creatures = super::engworldchg_journals::live_creatures;
    step(owner, admitted, 1, StepDirection::East).await?;
    assert!(creatures(owner, SPIDER).await?.is_empty());
    assert_eq!(map.facts.action_at(key), Some(23120));
    step(owner, admitted, 2, StepDirection::West).await?;

    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let request = QuestTransitionRequest {
        transition_key: BAIT_TRANSITION.into(),
        cause: QuestCause::Command(CommandRef::new(admitted.game_session_id, CommandId::new(2900).map_err(debug)?)),
    };
    let admitted_bait = slot.commit_quest_transition(
        owner.root, owner.character, owner.holder, fence, request,
        owner.quest_catalogue.clone().ok_or("quest catalogue")?,
    ).await.map_err(debug)?;
    assert!(matches!(admitted_bait, QuestTransitionOutcome::Committed(_)));
    drop(slot);

    let before = owner.owner_now().get();
    step(owner, admitted, 3, StepDirection::East).await?;
    let [spider] = creatures(owner, SPIDER).await?.try_into().map_err(|_| "exactly one spider")?;
    assert_eq!(map.facts.action_at(key), None);
    let due = map.facts.next_movement_decay_time().ok_or("rearm deadline")?;
    assert!(due >= before + 60_000_000);
    assert!(due <= owner.owner_now().get() + 600_000_000);
    owner.runtime.lock().await.remove_encounter_actor(spider).map_err(debug)?;
    step(owner, admitted, 4, StepDirection::West).await?;
    step(owner, admitted, 5, StepDirection::East).await?;
    assert!(creatures(owner, SPIDER).await?.is_empty());

    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert_eq!(map.facts.action_at(key), Some(23120));
    step(owner, admitted, 6, StepDirection::West).await?;
    step(owner, admitted, 7, StepDirection::East).await?;
    assert_eq!(creatures(owner, SPIDER).await?.len(), 1);
    Ok(())
}
