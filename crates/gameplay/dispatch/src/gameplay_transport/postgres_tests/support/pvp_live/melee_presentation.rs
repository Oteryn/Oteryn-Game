use super::*;
use crate::gameplay_transport::connection::{AdmittedSession, fresh_capability_selection};
use crate::spell::combat_batch::{ManaShieldState, OwnerCombatChange};
use oteryn_protocol_oteryn::{decode_state_delta, presentation};

pub(super) async fn drive(
    attacker_client: &mut DuplexStream,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    attacker: AdmittedSession,
    mut victim: AdmittedSession,
    payload: &[u8],
    selected: bool,
) -> TestResult {
    let source = attacker.runtime_actor.ok_or("attacker")?;
    let target = victim.runtime_actor.ok_or("victim")?;
    let fence = current_fence(owner, victim.game_session_id).await?.item_fence();
    victim.first_entry = FirstEntryOutcome::Positioned;
    victim.item_fence = Some(fence);
    victim.continuity.connection_generation = fence.connection_generation.get();
    victim.controller = None;
    victim.continuity.selected_capabilities = fresh_capability_selection(
        owner,
        if selected {
            &[6, 17, 21, presentation::CAPABILITY_PRESENTATION_V1]
        } else {
            &[6, 17, 21]
        },
    )
    .await
    .ok_or("victim capability selection")?;
    assert_eq!(
        victim
            .continuity
            .selected_capabilities
            .contains(presentation::CAPABILITY_PRESENTATION_V1),
        selected
    );

    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = serve_admitted(&mut server, victim, owner, IDLE_LIVENESS);
    let driving = async move {
        let stop = attack::encode_attack_target_intent(None).map_err(debug)?;
        let kind = attack::COMMAND_TYPE_ATTACK_TARGET_INTENT;
        // A command round trip proves the victim's snapshot and initial presentation cursor
        // are established before the attacker starts swinging.
        command(&mut client, 1, kind, &stop).await?;
        let before = {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states
                .get_mut(&runtime, target, victim.game_session_id)
                .ok_or("victim state")?;
            let before = state.vitals().health;
            // Absorb one point so a proposed swing magnitude cannot pass the HP-loss assertion.
            state.restore_saved_vitals(before, 1);
            let now_ms = owner.owner_now().get() / 1_000;
            crate::spell::actor_conditions::apply_combat_change(
                state,
                &OwnerCombatChange::ManaShield(ManaShieldState {
                    capacity: 1,
                    expires_ms: now_ms + 60_000,
                }),
                now_ms,
            )
            .map_err(debug)?;
            assert_eq!(state.vitals().mana, 1);
            before
        };
        let (sound_cursor, _) = owner
            .observe_sounds(source, attacker.game_session_id, u64::MAX)
            .await
            .ok_or("initial attacker sound cursor")?;
        let result = command(attacker_client, 3, kind, payload).await?;
        assert_eq!(
            attack::decode_attack_intent_result(&result).map_err(debug)?,
            attack::AttackIntentDisposition::Ok
        );
        tokio::time::timeout(Duration::from_secs(6), async {
            while health(owner, target).await? == before {
                wait_live(attacker_client, Duration::from_millis(25)).await?;
            }
            Ok::<_, Box<dyn std::error::Error>>(())
        })
        .await??;
        command(attacker_client, 4, kind, &stop).await?;
        let loss = u64::from(before - health(owner, target).await?);
        assert!(loss > 0);

        let (_, events) = owner
            .observe_sounds(source, attacker.game_session_id, sound_cursor)
            .await
            .ok_or("attacker sounds after the swing")?;
        assert!(events.iter().any(|event| matches!(
            event.kind,
            presentation::PresentationEventKind::Sound {
                cue_id: 1,
                source: presentation::PresentationSource::Own,
                ..
            }
        )));
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert!(states.has_training_to_flush(&runtime, source, attacker.game_session_id));
            assert_eq!(
                states
                    .get(&runtime, target, victim.game_session_id)
                    .ok_or("victim state after the swing")?
                    .vitals()
                    .mana,
                0
            );
        }

        let entity = |actor: crate::foundation::ExactActorRef| EntityRef {
            identity: actor.placement_identity(),
            generation: actor.actor_local_generation(),
        };
        let mut received = 0;
        // The first command's outbound refresh runs before the second command result.
        for id in 2..=3 {
            command_observing(&mut client, id, kind, &stop, |env| {
                if env.message_type() != MessageType::StateDelta {
                    return Ok(());
                }
                let delta = decode_state_delta(env.payload()).map_err(debug)?;
                if delta.domain_id != presentation::STATE_DOMAIN_WORLD_PRESENTATION {
                    return Ok(());
                }
                assert!(selected, "victim without presentation received its delta");
                if delta.delta_type != presentation::DELTA_TYPE_WORLD_PRESENTATION_BATCH_V1 {
                    return Ok(());
                }
                for event in presentation::decode_presentation_batch(delta.payload).map_err(debug)? {
                    if let presentation::PresentationEventKind::ValueText {
                        kind: presentation::PresentationValueKind::DamageReceived,
                        target: victim_ref,
                        attacker: attacker_ref,
                        primary,
                        ..
                    } = event.kind
                    {
                        assert_eq!(attacker_ref, Some(entity(source)));
                        assert_eq!(victim_ref, entity(target));
                        received += primary.value;
                    }
                }
                Ok(())
            })
            .await?;
        }
        assert_eq!(received, if selected { loss } else { 0 });
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (_, result) = join_two(serving, driving).await;
    result
}
