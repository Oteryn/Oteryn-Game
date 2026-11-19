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
    world_callbacks::{self, Callback, Offering, Position, Storage},
};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::{sync::{Arc, atomic::Ordering}, time::Duration};

pub(super) const NAME: &str = "haunted-cellar-disguise";
pub(super) const FINISH: &str = "haunted-cellar-third-challenge";
pub(super) const TIMER: &str = "haunted-cellar-disguise-after-exchange";
const QUEST: &str = "oteryn:quest.the_dream_courts_quest";
const ENTRY: &str = "oteryn:quest-callback/dream-courts-haunted-cellar";
const START: &str = "oteryn:quest-transition/haunted-cellar-fixture/start";
const START_AFTER_CHALLENGES: &str =
    "oteryn:quest-transition/haunted-cellar-fixture/start-after-challenges";
const HAUNTED: &str =
    "oteryn:quest-progress/crystalserver/quest/u12_00/the_dream_courts/haunted_house/questline";
const CELLAR: &str =
    "oteryn:quest-progress/crystalserver/quest/u12_00/the_dream_courts/haunted_house/cellar";
const TEMPLE: &str = "oteryn:quest-progress/quest/u12_00/the_dream_courts/haunted_house/temple";
const TOMB: &str = "oteryn:quest-progress/quest/u12_00/the_dream_courts/haunted_house/tomb";
// Crystal registers an AID. This room binds it to a test floor tile.
const TRIGGER: Position = Position {
    x: 32717,
    y: 32280,
    floor: -8,
};
const EXIT: Position = Position {
    x: 32727,
    ..TRIGGER
};
// This fixture generation admits a portable vial. The production retained input
// is the donor strange empty bucket i29310.
const FIXTURE_CONTAINER: &str = "oteryn:item.tibia.i2874";
const FULL_FIXTURE: &str = "oteryn:item.tibia.i3585";

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

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    world_cells(callback, TRIGGER, 431, 1, &[], &[], None, &[])
}

pub(super) fn catalogue(
    base: &QuestStateCatalogue,
    callback: &Callback,
) -> TestResult<Arc<QuestStateCatalogue>> {
    let mut base = base.clone();
    for (key, progress) in [(START, 0), (START_AFTER_CHALLENGES, 1)] {
        let effects = [(HAUNTED, 1), (TEMPLE, progress), (TOMB, progress)]
            .into_iter()
            .map(|(track, value)| QuestEffect {
                track: track.into(),
                from: QuestComparison::Eq(0),
                effect: QuestEffectKind::Set(value),
            })
            .collect();
        base = base
            .with_fixture_transition(QuestTransition {
                key: key.into(),
                quest: QUEST.into(),
                effects,
                completes: false,
                experience: None,
                world_change: None,
            })
            .map_err(debug)?;
    }
    let callbacks = admission_phases(callback)?;
    for entry in &callbacks {
        base = base
            .with_fixture_transition(world_callbacks::primary_transition(entry)?)
            .map_err(debug)?;
    }
    let document = serde_json::json!({"schema":1,"callbacks":callbacks}).to_string();
    let base = world_callbacks::with_fixture_world_document(&base, &document)?;
    Ok(Arc::new(with_item_self_binding(&base)?))
}

pub(super) fn with_item_self_binding(base: &QuestStateCatalogue) -> TestResult<QuestStateCatalogue> {
    let disguise = serde_json::json!({"schema":1,"tracks":[],"callbacks":[{
        "key":"oteryn:quest-reward/dream-courts/haunted-cellar-disguise",
        "quest":QUEST,"npc":"oteryn:npc.stricken_soul","keywords":["ectoplasm"],"topic":0,"next_topic":0,
        "carried_item":FULL_FIXTURE,"temporary_outfit":{"look_type":19,"duration_ms":120000},
        "repeatable":true,
        "storage":[{"track":HAUNTED,"minimum":1,"maximum":6,"set":1,"preserve":true}],
        "take":[{"item":FULL_FIXTURE,"count":1}],
        "give":{"item":FIXTURE_CONTAINER,"count":1,"contents":[]},"outfit":null,"achievement":null,
        "magic_effects":[{"effect":"CONST_ME_POFF"}],
        "reply":["You pour all of the ectoplasm over yourself. Without the bucket you cannot stabilise it, you need to hurry until it dissolves!"],
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_dream_courts/actions_strange_bucket.lua:53-65"]
    }]}).to_string();
    Ok(oteryn_gameplay_quests::quest::reward_callbacks::with_fixture_item_self_document(
        base, &disguise,
    )?)
}

/// The existing disjoint STEP phases express Crystal movements_slime_looktype.lua:17-18.
fn admission_phases(callback: &Callback) -> TestResult<[Callback; 3]> {
    let flag = |track: &str, value| -> Result<Storage, serde_json::Error> {
        serde_json::from_value(serde_json::json!({
            "track":track,"minimum":value,"maximum":value,"set":value,"preserve":true,
        }))
    };
    let mut temple_pending = callback.clone();
    temple_pending.key.push_str("-temple-pending");
    temple_pending.storage.push(flag(TEMPLE, 0)?);
    let mut tomb_pending = callback.clone();
    tomb_pending.key.push_str("-tomb-pending");
    tomb_pending
        .storage
        .extend([flag(TEMPLE, 1)?, flag(TOMB, 0)?]);
    let mut final_challenge = callback.clone();
    final_challenge.key.push_str("-last-challenge");
    let stage = final_challenge
        .storage
        .iter_mut()
        .find(|entry| entry.track == HAUNTED)
        .ok_or("haunted track")?;
    stage.set = 2;
    stage.preserve = false;
    final_challenge
        .storage
        .extend([flag(TEMPLE, 1)?, flag(TOMB, 1)?]);
    Ok([temple_pending, tomb_pending, final_challenge])
}

#[test]
fn haunted_cellar_self_use_applies_a_timed_slime_look_and_the_step_keeps_the_empty_container()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || scenario(NAME, TRIGGER, false))
}

#[test]
fn haunted_cellar_advances_the_haunted_house_journal_when_it_is_the_third_challenge() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(FINISH, || scenario(FINISH, TRIGGER, false))
}

#[test]
fn haunted_cellar_disguise_lasts_two_minutes_after_the_bucket_exchange_commits() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(TIMER, || scenario(TIMER, TRIGGER, false))
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
    name: &str,
) -> TestResult {
    let finishes = name == FINISH;
    input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: TRIGGER.x - 1,
                ..TRIGGER
            },
            item: FULL_FIXTURE.into(),
            count: 1,
        },
        39,
        true,
    )
    .await?;
    let full = carried_bucket(harness, fence, FULL_FIXTURE).await?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: if finishes {
                    START_AFTER_CHALLENGES
                } else {
                    START
                }
                .into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(2910).map_err(debug)?,
                )),
            },
            owner.quest_catalogue.clone().ok_or("quest catalogue")?,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;

    let actor = admitted.runtime_actor.ok_or("actor")?;
    if name == TIMER {
        apply_self_outfit(owner, admitted, actor, Some(harness)).await?;
        assert_eq!(carried_bucket(harness, fence, FIXTURE_CONTAINER).await?, full);
        return Ok(());
    }
    for (direction, expected) in [
        (StepDirection::East, TRIGGER),
        (
            StepDirection::West,
            Position {
                x: TRIGGER.x - 1,
                ..TRIGGER
            },
        ),
        (StepDirection::East, EXIT),
    ] {
        if expected == EXIT {
            apply_self_outfit(owner, admitted, actor, None).await?;
            // Crystal actions_strange_bucket.lua:66 transforms the selected
            // bucket. Keep its item identity and exact backpack placement.
            assert_eq!(carried_bucket(harness, fence, FIXTURE_CONTAINER).await?, full);
        }
        let ordinal = admitted.continuity.next_command_id;
        let reply = wire(
            owner,
            admitted,
            ordinal,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(direction),
        )
        .await?;
        assert_eq!(
            decode_step_result(&reply).map_err(debug)?,
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
    }
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let effects = states
        .get(&runtime, actor, admitted.game_session_id)
        .ok_or("spell state")?
        .owned_conditions();
    assert_eq!(effects.outfit_at(owner.owner_now().get()), None);
    let (progress,retained,journal):(i64,i64,i64) = sqlx::query_as(
        "SELECT (SELECT value FROM game_character_quest_tracks
                 WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),
                (SELECT count(*) FROM game_item_instances WHERE definition_production_key=$3 AND lifecycle=1),
                (SELECT value FROM game_character_quest_tracks
                 WHERE character_id=encode($1,'hex')::uuid AND track_key=$4)"
    ).bind(fence.character_id.as_bytes().as_slice()).bind(CELLAR).bind(FIXTURE_CONTAINER).bind(HAUNTED).fetch_one(&harness.pool).await?;
    assert_eq!(
        (progress, retained, journal),
        (1, 1, if finishes { 2 } else { 1 })
    );
    Ok(())
}

pub(super) async fn carried_bucket(
    harness: &Harness,
    fence: CurrentCharacterGameplayFence,
    definition: &str,
) -> TestResult<(String, serde_json::Value)> {
    Ok(sqlx::query_as(
        "SELECT i.item_instance_id::text,c.location
         FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c
         JOIN game_item_instances i ON i.item_instance_id=c.item
         WHERE i.definition_production_key=$3 AND i.lifecycle=1 AND i.quantity=1",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
    .bind(definition)
    .fetch_one(&harness.pool)
    .await?)
}

pub(super) async fn apply_self_outfit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    actor: crate::foundation::ExactActorRef,
    delay_exchange: Option<&Harness>,
) -> TestResult {
    use crate::gameplay_transport::connection::FreshAdmissionAuthority;
    let target = {
        let runtime = owner.runtime.lock().await;
        let players = runtime.visible_entities();
        let player = players
            .players
            .iter()
            .find(|player| player.actor == actor)
            .ok_or("visible self")?;
        EntityRef {
            identity: actor.placement_identity(),
            generation: player.generation,
        }
    };
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("USE capabilities")?;
    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|native| native.item_index())
        .ok_or("item index")?
        .definition_ref("Item", FULL_FIXTURE, "definition-r1")
        .ok_or("full container definition")?;
    for (selected, should_commit) in [
        (
            EntityRef {
                generation: target.generation + 1,
                ..target
            },
            false,
        ),
        (target, true),
    ] {
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::MIN),
            use_with: Some(selected),
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)?;
        let exchange = super::rust_lottery::exchange_latest_source(
            owner,
            *admitted,
            admitted.continuity.next_command_id,
            &payload,
            Some(reference),
        );
        let (next, result, _) = if let Some(harness) = delay_exchange.filter(|_| should_commit) {
            let mut blocker = harness.pool.begin().await?;
            // SHARE allows the replay probe to read the proof table. The live
            // exchange holds the runtime turn when its first INSERT waits here.
            sqlx::query("LOCK TABLE game_quest_callback_consumptions IN SHARE MODE")
                .execute(&mut *blocker)
                .await?;
            let writer_start = owner.owner_now().get() / 1_000 * 1_000;
            owner.states.test_owner_time.store(writer_start + 1, Ordering::Relaxed);
            let advance = async {
                loop {
                    let waiting: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM pg_locks \
                          WHERE database=(SELECT oid FROM pg_database WHERE datname=current_database()) \
                            AND relation='game_quest_callback_consumptions'::regclass \
                            AND mode='RowExclusiveLock' AND NOT granted)",
                    )
                    .fetch_one(&harness.pool)
                    .await?;
                    if waiting {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
                owner.states.test_owner_time.store(writer_start + 1_000_001, Ordering::Relaxed);
                blocker.commit().await
            };
            tokio::pin!(exchange);
            tokio::select! {
                biased;
                result = tokio::time::timeout(Duration::from_secs(5), advance) => result??,
                _ = &mut exchange => return Err("bucket exchange completed before its proof INSERT waited".into()),
            }
            exchange.await?
        } else {
            exchange.await?
        };
        *admitted = next;
        assert_eq!(result == UseDisposition::Committed, should_commit);
    }
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let outfit = states
        .get(&runtime, actor, admitted.game_session_id)
        .ok_or("spell state")?
        .owned_conditions();
    let now = owner.owner_now().get();
    assert_eq!(outfit.outfit_at(now), Some(19));
    if delay_exchange.is_some() {
        // Crystal actions_strange_bucket.lua:66-67 starts the outfit after
        // the bucket changes. Even the final microsecond must be available.
        assert_eq!(outfit.outfit_at(now + 119_999_999), Some(19));
    }
    assert_eq!(outfit.outfit_at(now + 120_000_000), None);
    Ok(())
}
