use super::super::autowalk_fixtures::{read, send};
use super::super::srx3_native_source_cast_fixtures::NativeOwnerClock;
use super::*;
use crate::gameplay_transport::native_cue_offer;
use oteryn_protocol_oteryn::{decode_state_delta, presentation as wire};
use std::num::NonZeroU32;
use tokio::io::DuplexStream;

pub(super) async fn check(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    book: &crate::spell::SpellBook,
) -> TestResult {
    // Start at the admitted dispatch boundary with the qualification world's presentation offer.
    let offered = native_cue_offer::offered(owner.offered_capabilities(), true);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(&offered, &[4, 6, 7, wire::CAPABILITY_PRESENTATION_V1])
            .ok_or("spell, chat and presentation capability selection")?;
    assert_eq!(
        admitted.continuity.selected_capabilities.as_slice(),
        &[4, 6, 7, wire::CAPABILITY_PRESENTATION_V1],
    );
    let index = |words| -> TestResult<NonZeroU32> {
        let key = &book.spoken(words).ok_or("spoken spell")?.spell.key;
        (1..=u32::try_from(book.source_len())?)
            .filter_map(NonZeroU32::new)
            .find(|index| book.indexed(*index).is_some_and(|spell| &spell.key == key))
            .ok_or_else(|| "spell index absent".into())
    };
    let light_healing = index("exura")?;
    let beam = index("exevo vis lux")?;
    let cast = |spell| {
        actor_spell::encode_spell_cast_intent(&actor_spell::SpellCastIntent {
            spell,
            target: actor_spell::SpellTarget::None,
            aim_at_target: false,
        })
    };
    let actor = admitted.runtime_actor.ok_or("caster")?;
    {
        let mut runtime = owner.runtime.lock().await;
        assert_eq!(
            crate::gameplay_transport::actor_spell::qualified_protection_zone(
                &mut runtime,
                owner.movement_cells(),
                actor,
            ),
            Some(true)
        );
    }
    let clock = NativeOwnerClock::new(owner);
    clock.set(1_000_000);
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async { serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await };
    let driving = async {
        loop {
            if decode_wire_envelope(&read(&mut client).await?)
                .map_err(debug)?
                .message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        let spell_command = actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT;
        send(&mut client, 1, spell_command, &cast(beam)).await?;
        let (status, payload, entries) = result(&mut client, 1).await?;
        assert_eq!(status, CommandStatus::Rejected);
        assert_eq!(
            actor_spell::decode_spell_cast_result(&payload).map_err(debug)?,
            actor_spell::SpellCastDisposition::Rejected
        );
        assert!(entries.is_empty());

        let indexed = cast(light_healing);
        send(&mut client, 2, spell_command, &indexed).await?;
        let (status, payload, entries) = result(&mut client, 2).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            actor_spell::decode_spell_cast_result(&payload).map_err(debug)?,
            actor_spell::SpellCastDisposition::Cast
        );
        assert!(entries.is_empty(), "PZ refusal published a cooldown");
        send(&mut client, 2, spell_command, &indexed).await?;
        let (status, _, entries) = result(&mut client, 2).await?;
        assert_eq!(status, CommandStatus::DuplicateReplay);
        assert_cooldowns(entries, light_healing, 0, 1_000);

        clock.set(3_000_000);
        let spoken = chat::encode_chat_intent(&chat::ChatIntent::Say {
            mode: chat::ChatSpeechMode::Say,
            text: "exura".into(),
        })
        .map_err(debug)?;
        send(&mut client, 3, chat::COMMAND_TYPE_CHAT_INTENT, &spoken).await?;
        let (status, payload, entries) = result(&mut client, 3).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            chat::decode_chat_intent_result(&payload)
                .map_err(debug)?
                .disposition,
            chat::ChatDisposition::Ok
        );
        assert!(entries.is_empty(), "indexed replay published a cooldown");
        send(&mut client, 3, chat::COMMAND_TYPE_CHAT_INTENT, &spoken).await?;
        let (status, _, entries) = result(&mut client, 3).await?;
        assert_eq!(status, CommandStatus::DuplicateReplay);
        assert_cooldowns(entries, light_healing, 1, 3_000);
        client.shutdown().await?;
        Ok(())
    };
    let (end, result): (_, TestResult) =
        tokio::time::timeout(Duration::from_secs(45), join_two(serving, driving)).await?;
    result?;
    assert!(
        matches!(
            end,
            crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(_)
        ),
        "{end:?}"
    );
    Ok(())
}

async fn result(
    client: &mut DuplexStream,
    command: u64,
) -> TestResult<(
    CommandStatus,
    Vec<u8>,
    Vec<(u64, u64, wire::ActorCooldowns)>,
)> {
    let mut cooldowns = Vec::new();
    let mut saw_vitals = false;
    loop {
        let bytes = read(client).await?;
        let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
        match envelope.message_type() {
            MessageType::StateDelta => {
                let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
                saw_vitals |= delta.domain_id == actor_spell::STATE_DOMAIN_ACTOR_VITALS;
                if delta.domain_id == wire::STATE_DOMAIN_ACTOR_COOLDOWNS {
                    assert!(!saw_vitals, "the casting client awaits vitals");
                    assert_eq!(delta.delta_type, wire::DELTA_TYPE_ACTOR_COOLDOWNS_V1);
                    cooldowns.push((
                        delta.base_revision,
                        delta.new_revision,
                        wire::decode_actor_cooldowns(delta.payload).map_err(debug)?,
                    ));
                }
            }
            MessageType::CommandResult => {
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, command);
                return Ok((result.status, result.payload.to_vec(), cooldowns));
            }
            _ => (),
        }
    }
}

fn assert_cooldowns(
    cooldowns: Vec<(u64, u64, wire::ActorCooldowns)>,
    spell: NonZeroU32,
    revision: u64,
    now: u64,
) {
    // Crystal 201189f8, data/scripts/spells/healing/light_healing.lua:
    // exura starts 1000 ms for both the spell and its healing group (S9 ordinal 2).
    assert_eq!(
        cooldowns,
        vec![(
            revision,
            revision + 1,
            wire::ActorCooldowns {
                server_now_ms: now,
                rtt_ms: None,
                entries: vec![
                    wire::ActorCooldown {
                        kind: wire::ActorCooldownKind::Spell,
                        id: spell.get(),
                        expires_at_ms: now + 1_000
                    },
                    wire::ActorCooldown {
                        kind: wire::ActorCooldownKind::Group,
                        id: 2,
                        expires_at_ms: now + 1_000
                    },
                ],
            }
        )]
    );
}

pub(super) async fn review_case(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    book: &crate::spell::SpellBook,
    case: u8,
) -> TestResult {
    use crate::gameplay_transport::connection::{ConnectionEnd, ControlLossResult, ResumeAttempt};
    use oteryn_simulation_determinism::SemanticTimeMicros;

    let offered = native_cue_offer::offered(owner.offered_capabilities(), true);
    let supported = [4, 6, 7, wire::CAPABILITY_PRESENTATION_V1];
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(&offered, &supported).ok_or("presentation selected")?;
    let actor = admitted.runtime_actor.ok_or("cooldown recipient")?;
    let session = admitted.game_session_id;
    let key = book.spoken("exura").ok_or("exura")?.spell.key.clone();
    let spell = (1..=u32::try_from(book.source_len())?)
        .filter_map(NonZeroU32::new)
        .find(|i| {
            book.source_indexed(*i)
                .is_some_and(|(spell, _)| spell.key == key)
        })
        .ok_or("exura index")?;
    let clock = NativeOwnerClock::new(owner);
    clock.set(1_000_000);
    if case == COOLDOWN_GIFT {
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let state = states
            .get_mut(&runtime, actor, session)
            .ok_or("recipient state")?;
        state
            .cooldowns
            .spells
            .insert(key.clone(), SemanticTimeMicros::from_micros(91_000_000));
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async move { serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await };
    let test_clock = &clock;
    let driving = async move {
        join_cooldowns(&mut client).await?;
        if case == COOLDOWN_GIFT {
            {
                let runtime = owner.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                // Initial vitals read refreshes the durable Wheel projection.
                // Set the qualified hit fixture after the join has read it.
                let state = states
                    .get_mut(&runtime, actor, session)
                    .ok_or("recipient state")?;
                let facts = crate::spell::cast::CharacterCastFacts {
                    vocation: crate::spell::Vocation::EliteKnight,
                    level: oteryn_gameplay_progression::wheel::WHEEL_LEVEL_OFFSET + 1,
                    ..state.character_facts()
                };
                let mut before = crate::spell::cast::PlayerSpellState::new(facts, 0, 0)
                    .ok_or("Wheel-eligible recipient")?;
                before.cooldowns = state.cooldowns.clone();
                before
                    .apply_owner_promotion_transition(true, true, Some(u64::MAX))
                    .map_err(debug)?;
                let mut effects =
                    oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects::default();
                effects.revelation_stages.insert("gift_of_life".into(), 1);
                before.install_wheel_effects(effects);
                assert!(before.restore_wheel_gift(0, 0));
                before.install_wheel_gift_spellbook(book);
                before.restore_saved_vitals(10, 0);
                *state = before;
                let (next, hit) =
                    crate::spell::actor_conditions::stage_creature_hit(state, 10, 1_000_000)
                        .map_err(debug)?;
                assert!(hit.health_after > 0);
                assert_eq!(next.cooldowns.spells[&key].get(), 31_000_000);
                assert!(states.commit(&runtime, actor, session, next));
            }
            let (base, value) = next_cooldowns(&mut client, "recipient Gift of Life").await?;
            assert_eq!(base, 0);
            assert_eq!(
                value.entries,
                vec![wire::ActorCooldown {
                    kind: wire::ActorCooldownKind::Spell,
                    id: spell.get(),
                    expires_at_ms: 31_000,
                }]
            );
        } else {
            if case == COOLDOWN_RETRY {
                let runtime = owner.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                let state = states
                    .get_mut(&runtime, actor, session)
                    .ok_or("caster state")?;
                state.cooldowns.groups.insert(
                    "invalid_fixture_group".into(),
                    SemanticTimeMicros::from_micros(90_000_000),
                );
            }
            let payload = actor_spell::encode_spell_cast_intent(&actor_spell::SpellCastIntent {
                spell,
                target: actor_spell::SpellTarget::None,
                aim_at_target: false,
            });
            let command = actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT;
            send(&mut client, 1, command, &payload).await?;
            let (status, cast, _) = result(&mut client, 1).await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(
                actor_spell::decode_spell_cast_result(&cast).map_err(debug)?,
                actor_spell::SpellCastDisposition::Cast
            );
            if case == COOLDOWN_RETRY {
                send(&mut client, 1, command, &payload).await?;
                let (status, _, entries) = result(&mut client, 1).await?;
                assert_eq!(status, CommandStatus::DuplicateReplay);
                assert!(entries.is_empty(), "the first publication must fail");
                {
                    let runtime = owner.runtime.lock().await;
                    let mut states = owner.states.spell_states.lock().await;
                    let state = states
                        .get_mut(&runtime, actor, session)
                        .ok_or("caster state")?;
                    state.cooldowns.groups.remove("invalid_fixture_group");
                }
                let (base, value) =
                    next_cooldowns(&mut client, "retry after publication failure").await?;
                assert_eq!(base, 0);
                assert_eq!(value.entries.len(), 2);
            } else {
                let (_, value) = next_cooldowns(&mut client, "committed cast").await?;
                assert_eq!(value.rtt_ms, None);
                if case == COOLDOWN_RTT {
                    loop {
                        let bytes = read(&mut client).await?;
                        let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
                        if envelope.message_type() == MessageType::LivenessProbe {
                            let id =
                                oteryn_protocol_oteryn::decode_liveness_probe(envelope.payload())
                                    .map_err(debug)?;
                            test_clock.set(1_040_000);
                            let ack = oteryn_protocol_oteryn::encode_liveness_ack(1, id, 0)
                                .map_err(debug)?;
                            crate::gameplay_transport::tcp_tls::write_frame(&mut client, &ack)
                                .await?;
                            break;
                        }
                    }
                    let (base, correction) =
                        next_cooldowns(&mut client, "first probe correction").await?;
                    assert_eq!(base, 1);
                    assert_eq!(correction.rtt_ms, Some(40));
                    assert_eq!(correction.entries, value.entries);
                }
            }
        }
        client.shutdown().await?;
        Ok(())
    };
    let (end, result): (_, TestResult) =
        tokio::time::timeout(Duration::from_secs(18), join_two(serving, driving)).await?;
    result?;
    if case == COOLDOWN_REJOIN {
        let ConnectionEnd::AdmittedThenDisconnected(retained) = end else {
            return Err(format!("cast connection end: {end:?}").into());
        };
        assert_eq!(
            owner.lose_control(retained, Duration::ZERO, None).await,
            ControlLossResult::Recorded
        );
        let token = super::super::house_transfer_fixtures::RecoverySource::token(2)?;
        let resumed = owner
            .resume(ResumeAttempt {
                game_session_id: session,
                recovery_material: token.as_bytes(),
                transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[2; 16])
                    .map_err(debug)?,
                last_applied_server_sequence: retained.continuity.server_sequence,
                supported_capabilities: &supported,
            })
            .await
            .map_err(debug)?;
        assert_eq!(resumed.continuity.connection_generation, 2);
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let serving =
            async move { serve_admitted(&mut server, resumed, owner, IDLE_LIVENESS).await };
        let driving = async move {
            let (revision, snapshot) = join_cooldowns(&mut client).await?;
            assert_eq!(snapshot.entries.len(), 2, "rejoin lost committed deadlines");
            assert_eq!(revision, 1, "rejoin lost the delivered domain revision");
            client.shutdown().await?;
            Ok(())
        };
        let (_, result): (_, TestResult) =
            tokio::time::timeout(Duration::from_secs(10), join_two(serving, driving)).await?;
        result?;
    }
    Ok(())
}

async fn join_cooldowns(client: &mut DuplexStream) -> TestResult<(u64, wire::ActorCooldowns)> {
    let mut snapshot = None;
    loop {
        let bytes = read(client).await?;
        let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
        match envelope.message_type() {
            MessageType::SnapshotChunk => {
                for row in oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())
                    .map_err(debug)?
                    .1
                {
                    if row.domain_id == wire::STATE_DOMAIN_ACTOR_COOLDOWNS {
                        snapshot = Some((
                            row.revision,
                            wire::decode_actor_cooldowns(row.payload).map_err(debug)?,
                        ));
                    }
                }
            }
            MessageType::SnapshotCommit => {
                return snapshot.ok_or_else(|| "no cooldown snapshot".into());
            }
            _ => {}
        }
    }
}

async fn next_cooldowns(
    client: &mut DuplexStream,
    context: &str,
) -> TestResult<(u64, wire::ActorCooldowns)> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let bytes = read(client).await?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            if envelope.message_type() == MessageType::StateDelta {
                let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
                if delta.domain_id == wire::STATE_DOMAIN_ACTOR_COOLDOWNS {
                    return Ok((
                        delta.base_revision,
                        wire::decode_actor_cooldowns(delta.payload).map_err(debug)?,
                    ));
                }
            }
        }
    })
    .await
    .map_err(|_| format!("{context}: no domain 20 delta"))?
}
