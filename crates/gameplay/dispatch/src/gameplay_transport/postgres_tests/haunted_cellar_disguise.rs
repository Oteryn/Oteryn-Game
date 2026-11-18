//! Crystal 201189f the_dream_courts/movements_slime_looktype.lua:10-28.
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

pub(super) const NAME: &str = "haunted-cellar-disguise";
const QUEST: &str = "oteryn:quest.the_dream_courts_quest";
const ENTRY: &str = "oteryn:quest-callback/dream-courts-haunted-cellar";
const START: &str = "oteryn:quest-transition/haunted-cellar-fixture/start";
const HAUNTED: &str = "oteryn:quest-progress/crystalserver/quest/u12_00/the_dream_courts/haunted_house/questline";
const CELLAR: &str = "oteryn:quest-progress/crystalserver/quest/u12_00/the_dream_courts/haunted_house/cellar";
// Crystal registers an AID. This room binds it to a test floor tile.
const TRIGGER: Position = Position {x:32717,y:32280,floor:-8};
const EXIT: Position = Position {x:32727,..TRIGGER};
// This fixture generation admits a portable vial. The production retained input
// is the donor strange empty bucket i29310.
const FIXTURE_CONTAINER: &str = "oteryn:item.tibia.i2874";

pub(super) fn binding() -> TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key":ENTRY,"quest":QUEST,"trigger":"step_in",
        "selector":{"position":TRIGGER,"action":23102,"appearance":431},
        "requires_world_admission":true,
        "reversible":false,"return_on_failure":false,"passages":[],"offerings":[],
        "teleport":EXIT,"present":[{"item":FIXTURE_CONTAINER,"count":1}],
        "storage":[{"track":HAUNTED,"minimum":1,"maximum":6,"set":1,"preserve":true},
                   {"track":CELLAR,"minimum":0,"maximum":0,"set":1}],
        "effects":{"requires_temporary_look":19,"restore_disguise":true,"teleport_effect":11},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/movements_slime_looktype.lua:10-28"]
    }))?)
}

pub(super) fn world(callback:&Callback) -> TestResult<crate::map::boot::BundleWorld> {
    world_cells(callback,TRIGGER,431,1,&[],&[],None,&[])
}

pub(super) fn catalogue(base:&QuestStateCatalogue, callback:&Callback) -> TestResult<Arc<QuestStateCatalogue>> {
    let base = base.with_fixture_transition(QuestTransition {
        key:START.into(),quest:QUEST.into(),effects:vec![QuestEffect {
            track:HAUNTED.into(),from:QuestComparison::Eq(0),effect:QuestEffectKind::Set(1)
        }],
        completes:false,experience:None,world_change:None,
    }).map_err(debug)?;
    let base = base.with_fixture_transition(world_callbacks::primary_transition(callback)?).map_err(debug)?;
    let document = serde_json::json!({"schema":1,"callbacks":[callback]}).to_string();
    Ok(Arc::new(world_callbacks::with_fixture_world_document(&base,&document)?))
}

#[test]
fn haunted_cellar_step_requires_a_live_slime_look_and_keeps_the_empty_container() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || scenario(NAME,TRIGGER,false))
}

pub(super) async fn exercise(
    owner:&ComposedFreshAdmission<'_, '_, '_>, admitted:&mut AdmittedSession,
    harness:&Harness, fence:CurrentCharacterGameplayFence,
) -> TestResult {
    input_item(owner,fence.item_fence(),&Offering {
        position:Position{x:TRIGGER.x-1,..TRIGGER},item:FIXTURE_CONTAINER.into(),count:1
    },39,true).await?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot.commit_quest_transition(
        owner.root,owner.character,owner.holder,fence,
        QuestTransitionRequest { transition_key:START.into(),cause:QuestCause::Command(
            CommandRef::new(admitted.game_session_id,CommandId::new(2910).map_err(debug)?))},
        owner.quest_catalogue.clone().ok_or("quest catalogue")?,
    ).await.map_err(debug)?;
    assert!(matches!(outcome,QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner.refresh_quest_session(admitted.game_session_id,false).await;

    let actor = admitted.runtime_actor.ok_or("actor")?;
    for (ordinal, direction, expected) in [
        (1,StepDirection::East,TRIGGER),
        (2,StepDirection::West,Position{x:TRIGGER.x-1,..TRIGGER}),
        (3,StepDirection::East,EXIT),
    ] {
        if ordinal == 3 {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let current = states.get(&runtime,actor,admitted.game_session_id).ok_or("spell state")?;
            let disguise = crate::spell::actor_conditions::stage_item_costume(
                current,19,120_000,owner.owner_now().get()/1_000
            ).map_err(debug)?;
            assert!(states.commit(&runtime,actor,admitted.game_session_id,disguise));
        }
        let reply = wire(owner,admitted,ordinal,COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,encode_step_intent(direction)).await?;
        assert_eq!(decode_step_result(&reply).map_err(debug)?, StepDisposition::Moved);
        assert_eq!(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position(), local(expected));
    }
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let effects = states.get(&runtime,actor,admitted.game_session_id).ok_or("spell state")?.owned_conditions();
    assert_eq!(effects.outfit_at(owner.owner_now().get()),None);
    let (progress,retained):(i64,i64) = sqlx::query_as(
        "SELECT (SELECT value FROM game_character_quest_tracks
                 WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),
                (SELECT count(*) FROM game_item_instances WHERE definition_production_key=$3 AND lifecycle=1)"
    ).bind(fence.character_id.as_bytes().as_slice()).bind(CELLAR).bind(FIXTURE_CONTAINER).fetch_one(&harness.pool).await?;
    assert_eq!((progress,retained),(1,1));
    Ok(())
}
