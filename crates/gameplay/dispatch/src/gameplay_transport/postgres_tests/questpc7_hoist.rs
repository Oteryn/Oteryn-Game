//! Hoist STEP transport checks both instruction flags on every trip.
use super::fixture::{Harness, debug};
use super::{TestResult, quest_world_callbacks};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::connection::AdmittedSession;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use quest_world_callbacks::{local, wire};

#[test]
fn questpc7_formorgar_hoist_read_only_step() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("questpc7-hoist", || {
        for (name, floor) in [
            ("formogar-hoist-3059", -9),
            ("formogar-hoist-3060", -10),
            ("formogar-hoist-3061", -11),
        ] {
            let start = std::time::Instant::now();
            quest_world_callbacks::scenario(
                name,
                Position {
                    x: 32157,
                    y: 31125,
                    floor,
                },
                false,
            )?;
            eprintln!(
                "QUESTPC7_HOIST POSTGRES_CASE_COMPLETED {name} duration={:?}",
                start.elapsed()
            );
        }
        Ok(())
    })?;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    callback: &Callback,
    trigger: Position,
) -> TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::foundation::{CharacterId, CharacterLease, CommandId, CommandRef};
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
        QuestTransition,
    };
    use oteryn_protocol_oteryn::world_spatial::{
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, encode_step_intent,
    };
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            CharacterLease::new(
                CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let tracks = callback
        .storage
        .iter()
        .map(|storage| QuestTrack {
            key: storage.track.clone(),
            quest: callback.quest.clone(),
            initial: -1,
            min: -1,
            max: 2,
        })
        .collect();
    let combinations = [
        (-1, -1),
        (1, -1),
        (-1, 1),
        (1, 1),
        (2, 1),
        (1, 1),
        (1, 1),
        (1, 1),
    ];
    let mut transitions: Vec<_> = combinations
        .iter()
        .enumerate()
        .map(|(n, &(skeleton, chest))| QuestTransition {
            key: format!("oteryn:quest-transition/fixture/hoist-{n}"),
            quest: callback.quest.clone(),
            effects: callback
                .storage
                .iter()
                .zip([skeleton, chest])
                .map(|(storage, value)| QuestEffect {
                    track: storage.track.clone(),
                    from: QuestComparison::Between(-1, 2),
                    effect: QuestEffectKind::Set(value),
                })
                .collect(),
            completes: false,
            experience: None,
        world_change: None,
        })
        .collect();
    if let Some(transition) = owner
        .quest_catalogue
        .as_ref()
        .and_then(|c| c.transition(&callback.key))
    {
        transitions.push(transition.clone());
    }
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new("content-1", tracks, transitions).map_err(debug)?,
    );
    owner.quest_catalogue = Some(catalogue.clone());
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Hoist", owner.owner_now());
    for (n, &(skeleton, chest)) in combinations.iter().enumerate() {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let seed = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: format!("oteryn:quest-transition/fixture/hoist-{n}"),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(900 + u64::try_from(n)?).map_err(debug)?,
                    )),
                },
                catalogue.clone(),
            )
            .await
            .map_err(debug)?;
        assert!(
            matches!(seed, QuestTransitionOutcome::Committed(_)),
            "{seed:?}"
        );
        drop(slot);
        let origin = local(Position {
            x: trigger.x - 1,
            ..trigger
        });
        {
            let mut runtime = owner.runtime.lock().await;
            if let Some(remote) = &callback.remote_teleport {
                use crate::map::view::placement_key;
                let map = owner.bundle_map().ok_or("map")?;
                let p = crate::map::overlay::TilePos {
                    x: remote.position.x,
                    y: remote.position.y,
                    floor: remote.position.floor,
                };
                let base = map
                    .overlay
                    .base()
                    .tile(p.x, p.y, p.floor)
                    .ok_or("remote tile")?;
                let remote_key = placement_key(p, 1).ok_or("remote key")?;
                let mut after = map
                    .facts
                    .base_entry(p, 1, base.ids()[1])
                    .ok_or("remote facts")?;
                after.appearance_id = if n == 5 { 2773 } else { remote.appearance };
                assert!(map.facts.apply_quest_callback(
remote_key,
map.facts.object_revision(remote_key),
after,
&[remote_key],
&[],
n != 7,
None,
));
            }
            let expected = runtime.read_actor_position(actor).map_err(debug)?;
            if expected.position() != origin {
                let map = owner.bundle_map().ok_or("map")?;
                let closed = crate::map::floor::BundleDoors(map.facts.doors());
                let proof = crate::map::floor::prepare_quest_callback_teleport(
                    &runtime,
                    &owner.movement_cells,
                    actor,
                    admitted.game_session_id,
                    expected,
                    origin,
                    false,
                    &closed,
                )
                .map_err(debug)?;
                runtime.commit_source_step(proof).map_err(debug)?;
            }
        }
        let before = owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?;
        let counts: (i64, i64, String) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_character_quest_receipts), \
             (SELECT count(*) FROM game_character_quest_states), \
             (SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid)")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        wire(
            owner,
            admitted,
            1 + u64::try_from(n)?,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(StepDirection::East),
        )
        .await?;
        let expected = if skeleton == 1 && chest == 1 {
            local(
                if let Some(remote) = callback
                    .remote_teleport
                    .as_ref()
                    .filter(|_| n != 5 && n != 7)
                {
                    remote.destination
                } else {
                    callback.teleport.ok_or("teleport")?
                },
            )
        } else {
            origin
        };
        let feedback = owner.states.chat.drain(admitted.game_session_id);
        if skeleton != 1 || chest != 1 {
            let message = callback.failure_message.as_ref().ok_or("failure message")?;
            assert!(
                feedback.iter().any(|line| matches!(line,
                oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == message)),
                "refused hoist must explain the instruction gate"
            );
        }
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            expected,
            "both flags must equal one: {skeleton}/{chest}"
        );
        let after = owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(before.tracks(), after.tracks());
        let after_counts: (i64, i64, String) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_character_quest_receipts), \
             (SELECT count(*) FROM game_character_quest_states), \
             (SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid)")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            after_counts, counts,
            "STEP must not write Quest state, receipts or Character revision"
        );
        assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
        let key = crate::map::view::placement_key(
            crate::map::overlay::TilePos {
                x: trigger.x,
                y: trigger.y,
                floor: trigger.floor,
            },
            1,
        )
        .ok_or("hoist key")?;
        assert_eq!(
            owner.bundle_map().ok_or("map")?.facts.object_revision(key),
            0,
            "read-only STEP changed the map object"
        );
    }
    Ok(())
}
