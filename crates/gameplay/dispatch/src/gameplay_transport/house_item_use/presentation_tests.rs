use super::*;
use crate::foundation::{
    CharacterId, CharacterLease, ConnectionGeneration, GameSessionId, LocalPosition,
    ScopeOwnershipGeneration,
};
use oteryn_gameplay_entities::carrier::house_custody::HouseActorState;
use std::future::Future;
use std::task::Poll;

#[test]
fn w18_house_committed_cue_waits_without_holding_runtime_and_publishes_once()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .map_err(|error| format!("{error:?}"))?
        .block_on(async {
            let (mut runtime, actor, _, source) =
                crate::gameplay_transport::spell_presentations::tests::runtime();
            let position = runtime
                .read_actor_position(actor)
                .map_err(|error| format!("{error:?}"))?
                .position();
            let mut bytes = *source.as_bytes();
            bytes[15] = 89;
            let character = CharacterId::decode(&bytes).map_err(|error| format!("{error:?}"))?;
            runtime
                .bind_attacker_lease(
                    actor,
                    source,
                    CharacterLease::new(character, 1).map_err(|error| format!("{error:?}"))?,
                )
                .map_err(|error| format!("{error:?}"))?;
            bytes[15] = 90;
            let session = GameSessionId::decode(&bytes).map_err(|error| format!("{error:?}"))?;
            let house = HouseId::new(
                runtime.binding().world_id(),
                "oteryn:content.house.presentation_fixture",
            )
            .ok_or("fixture House key is valid")?;
            let scope = house
                .runtime_scope()
                .map_err(|error| format!("{error:?}"))?;
            let lease = CharacterLease::new(character, 2).map_err(|error| format!("{error:?}"))?;
            let generation =
                ScopeOwnershipGeneration::new(1).map_err(|error| format!("{error:?}"))?;
            let at = LocalPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            };
            let actor = runtime
                .restore_transfer_actor(
                    source,
                    session,
                    lease,
                    at,
                    Some(HouseActorState {
                        house: house.house_key().into(),
                        scope,
                        generation,
                        position: at,
                        lease,
                        origin_position: at,
                    }),
                )
                .map_err(|error| format!("{error:?}"))?;
            let fence = CurrentCharacterItemFence {
                character_id: crate::domain::CharacterId::from_bytes(*character.as_bytes())
                    .map_err(|error| format!("{error:?}"))?,
                game_session_id: session,
                connection_generation: ConnectionGeneration::new(1)
                    .map_err(|error| format!("{error:?}"))?,
                character_lease_generation: 2,
                runtime_scope: scope,
                scope_ownership_generation: generation,
            };
            let runtime = Mutex::new(runtime);
            let states = Mutex::new(ChannelSpellStates::default());
            let chat = ChatRuntime::default();
            let effects = serde_json::json!({"ground":{"x":at.x,"y":at.y,"floor":at.floor},
            "cues":[{"effect_id":175}]});
            let held = states.lock().await;
            let mut publishing = std::pin::pin!(publish(
                &runtime,
                &states,
                &chat,
                Publication {
                    actor,
                    fence,
                    house: &house,
                    effects: &effects
                }
            ));
            std::future::poll_fn(|cx| {
                assert!(publishing.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;
            assert!(
                runtime.try_lock().is_ok(),
                "a contending publication must release Runtime"
            );
            drop(held);
            assert!(
                tokio::time::timeout(std::time::Duration::from_secs(1), publishing)
                    .await
                    .map_err(|error| format!("{error:?}"))?
            );
            let revision = {
                let runtime = runtime.lock().await;
                let states = states.try_lock().map_err(|error| format!("{error:?}"))?;
                let (revision, cues) = states
                    .sounds
                    .observe(&runtime, actor, session, 0, true)
                    .ok_or("current House observes committed sound")?;
                assert_eq!(
                    cues.len(),
                    1,
                    "retry must publish this committed cue exactly once"
                );
                assert!(matches!(
                    cues[0].kind,
                    oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect {
                        effect_id: 175,
                        ..
                    }
                ));
                assert!(
                    states
                        .sounds
                        .observe(&runtime, actor, session, revision, true)
                        .ok_or("cursor suppresses already observed sound")?
                        .1
                        .is_empty()
                );
                revision
            };
            let retired = CurrentCharacterItemFence {
                game_session_id: source,
                ..fence
            };
            assert!(
                !publish(
                    &runtime,
                    &states,
                    &chat,
                    Publication {
                        actor,
                        fence: retired,
                        house: &house,
                        effects: &effects
                    }
                )
                .await
            );
            let runtime = runtime.lock().await;
            let states = states.try_lock().map_err(|error| format!("{error:?}"))?;
            assert!(
                states
                    .sounds
                    .observe(&runtime, actor, session, revision, true)
                    .ok_or("retired publication emits no further sound")?
                    .1
                    .is_empty(),
                "session must be current under the jointly acquired guards"
            );
            Ok::<(), Box<dyn std::error::Error>>(())
        })
}
