//! Crystal 201189f the_dream_courts/actions_sacrophagus_use.lua:16-62 and items.xml:55554-55559.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{scenario, wire, world_cells};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::{
    character_progression::CurrentCharacterGameplayFence,
    quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest},
};
use crate::foundation::{CommandId, CommandRef};
use crate::map::{overlay::TilePos, view::placement_key};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    world_callbacks::{self, Callback, Position},
};
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use std::sync::Arc;

pub(super) const PRIZE: &str = "haunted-tomb-prize";
pub(super) const MUMMY_CASE: &str = "haunted-tomb-mummy";
const QUEST: &str = "oteryn:quest.the_dream_courts_quest";
const STARTED: &str = "oteryn:quest-progress/crystalserver/quest/u12_00/the_dream_courts/haunted_house/questline";
const TOMB: &str = "oteryn:quest-progress/quest/u12_00/the_dream_courts/haunted_house/tomb";
const KEY: &str = "oteryn:quest-callback/haunted-tomb/33081-32355";
const BEGIN: &str = "oteryn:quest-transition/haunted-tomb-fixture/start";
const MUMMY: &str = "oteryn:creature.mummy";
const SARCOPHAGUS: Position = Position {x:33081,y:32355,floor:-8};
// The existing test generation can mint this native fixture Item. Production
// authoring binds the donor's i29351 temporal key.
const FIXTURE_ITEM: &str = "oteryn:item.tibia.i3585";

pub(super) fn binding() -> TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key":KEY,"quest":QUEST,"trigger":"use",
        "selector":{"native":{"position":SARCOPHAGUS,"ordinal":2,"appearance":29349}},
        "return_on_failure":false,"reversible":false,"passages":[],"offerings":[],
        "requires_world_admission":true,
        "storage":[
            {"track":STARTED,"minimum":1,"maximum":1,"set":1,"preserve":true},
            {"track":TOMB,"minimum":0,"maximum":0,"set":0,"preserve":true}
        ],
        "failure_message":"The opening of the last sarcophagus still haunts you. Wait some time for your spirits to return.",
        "effects":{"roll_reward":{"sides":10,"minimum_success":7,"failure_creature":MUMMY,
            "failure_effect":3,"failure_text":"The curse of disturbing this fragile, ancient peace is your price to pay!",
            "rearm_ms":30000,"source_action":23104},
            "item_reward":{"item":FIXTURE_ITEM,"count":1,"contents":[]}},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/actions_sacrophagus_use.lua:1-62"]
    }))?)
}

pub(super) fn world(callback:&Callback) -> TestResult<crate::map::boot::BundleWorld> {
    world_cells(callback, SARCOPHAGUS, 29349, 2, &[], &[], None, &[])
}

pub(super) fn catalogue(base:&QuestStateCatalogue, callback:&Callback) -> TestResult<Arc<QuestStateCatalogue>> {
    let base = base.with_fixture_transition(QuestTransition {
        key:BEGIN.into(),quest:QUEST.into(),effects:vec![QuestEffect {
            track:STARTED.into(),from:QuestComparison::Eq(0),effect:QuestEffectKind::Set(1)
        }],
        completes:false,experience:None,world_change:None,
    }).map_err(debug)?;
    let base = base.with_fixture_transition(world_callbacks::primary_transition(callback)?).map_err(debug)?;
    let document = serde_json::json!({"schema":1,"callbacks":[callback]}).to_string();
    Ok(Arc::new(world_callbacks::with_fixture_world_document(&base,&document)?))
}

#[test]
fn haunted_tomb_success_roll_mints_only_one_reward_before_the_source_rearms() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(PRIZE, || scenario(PRIZE,SARCOPHAGUS,false))
}

#[test]
fn haunted_tomb_failed_roll_spawns_a_mummy_without_minting_the_reward() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(MUMMY_CASE, || scenario(MUMMY_CASE,SARCOPHAGUS,false))
}

pub(super) async fn exercise(
    owner:&ComposedFreshAdmission<'_, '_, '_>, admitted:&mut AdmittedSession,
    harness:&Harness, fence:CurrentCharacterGameplayFence, callback:&Callback, won:bool,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let source = placement_key(TilePos {x:SARCOPHAGUS.x,y:SARCOPHAGUS.y,floor:SARCOPHAGUS.floor},2)
        .ok_or("sarcophagus")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&source.to_be_bytes());
    let use_payload = |expected_revision| encode_use_intent(&WorldObjectTarget {
        placement:placement.clone(),expected_revision
    }).map_err(debug);
    let reply = wire(owner,admitted,1,COMMAND_TYPE_USE_INTENT,use_payload(0)?).await?;
    assert_ne!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Committed);

    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot.commit_quest_transition(
        owner.root,owner.character,owner.holder,fence,
        QuestTransitionRequest { transition_key:BEGIN.into(), cause:QuestCause::Command(
            CommandRef::new(admitted.game_session_id,CommandId::new(2903).map_err(debug)?))},
        owner.quest_catalogue.clone().ok_or("catalogue")?,
    ).await.map_err(debug)?;
    assert!(matches!(outcome,QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner.refresh_quest_session(admitted.game_session_id,false).await;

    let actor = admitted.runtime_actor.ok_or("actor")?;
    let revision = owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.revision();
    let roll = callback.effects.roll_reward.as_ref().ok_or("roll")?;
    let command = (2..=64).find(|ordinal| roll.succeeds(
        KEY,admitted.game_session_id.as_bytes(),*ordinal,&actor.placement_identity(),revision
    )==won).ok_or("fixture roll occurrence")?;
    for ordinal in 2..command {
        let reply = wire(owner,admitted,ordinal,COMMAND_TYPE_USE_INTENT,use_payload(1)?).await?;
        assert_ne!(decode_use_result(&reply).map_err(debug)?,UseDisposition::Committed);
    }
    let before = owner.owner_now().get();
    let reply = wire(owner,admitted,command,COMMAND_TYPE_USE_INTENT,use_payload(0)?).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?,UseDisposition::Committed);
    assert_eq!(super::engworldchg_journals::live_creatures(owner,MUMMY).await?.len(),usize::from(!won));
    let due = map.facts.next_movement_decay_time().ok_or("rearm deadline")?;
    assert!(due>=before+30_000_000);
    assert!(!map.facts.quest_callback_action_ready(source,23104));
    let revision = map.facts.object_revision(source);
    let reply = wire(owner,admitted,command+1,COMMAND_TYPE_USE_INTENT,use_payload(revision)?).await?;
    assert_ne!(decode_use_result(&reply).map_err(debug)?,UseDisposition::Committed);
    let rewards:i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1")
        .bind(FIXTURE_ITEM).fetch_one(&harness.pool).await?;
    assert_eq!(rewards,i64::from(won));
    assert_eq!(map.facts.expire_movement_items(due-1),0);
    assert_eq!(map.facts.expire_movement_items(due),1);
    assert!(map.facts.quest_callback_action_ready(source,23104));
    Ok(())
}
