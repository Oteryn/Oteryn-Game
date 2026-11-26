use super::super::autowalk_fixtures::read;
use super::super::srx3_native_source_cast_fixtures::NativeOwnerClock;
use super::*;
use crate::gameplay_transport::connection::{ConnectionEnd, ControlLossResult, ResumeAttempt};
use oteryn_protocol_oteryn::{decode_snapshot_chunk, decode_state_delta, presentation as wire};
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::num::NonZeroU32;
use tokio::io::DuplexStream;

const SUPPORTED: [u32; 4] = [4, 6, 7, wire::CAPABILITY_PRESENTATION_V1];

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    book: &crate::spell::SpellBook,
    case: u8,
) -> TestResult {
    let offered = crate::gameplay_transport::native_cue_offer::offered(
        owner.offered_capabilities(),
        true,
    );
    // Omit item views in the holder-contention case so the test isolates the join's
    // vitals and cooldown read from independently required inventory snapshots.
    let supported = if case == 104 {
        &[6, wire::CAPABILITY_PRESENTATION_V1][..]
    } else {
        &SUPPORTED[..]
    };
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(&offered, supported).ok_or("presentation selected")?;
    let actor = admitted.runtime_actor.ok_or("caster")?;
    let session = admitted.game_session_id;
    let key = &book.spoken("exura").ok_or("exura")?.spell.key;
    let spell = (1..=u32::try_from(book.source_len())?)
        .filter_map(NonZeroU32::new)
        .find(|i| book.source_indexed(*i).is_some_and(|(spell, _)| &spell.key == key))
        .ok_or("exura index")?;
    let clock = NativeOwnerClock::new(owner);
    clock.set(1_000_000);

    if case == 105 {
        // Seed the persistent condition's owner value before admission.
        let runtime = owner.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        let state = states.get_mut(&runtime, actor, session).ok_or("caster state")?;
        state.cooldowns.spells.insert(key.clone(), SemanticTimeMicros::from_micros(u64::MAX));
        state.cooldowns.groups.insert("healing".into(), SemanticTimeMicros::from_micros(3_000_000));
    }
    if case == 103 {
        return gift(owner, admitted, book, spell).await;
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async { serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await };
    let driving = async {
        let (_, snapshot) = join_cooldowns(&mut client).await?;
        let cast = actor_spell::encode_spell_cast_intent(&actor_spell::SpellCastIntent {
            spell,
            target: actor_spell::SpellTarget::None,
            aim_at_target: false,
        });
        let command = actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT;
        if case != 105 {
            assert!(snapshot.entries.is_empty());
            send(&mut client, 1, 1, command, &cast).await?;
        }
        let barrier = if case == 105 { 1 } else { 2 };
        if case != 101 {
            send(&mut client, 1, barrier, 65_535, &[]).await?;
        }
        let mut deltas = Vec::new();
        let mut saw_result = false;
        loop {
            let frame = read(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::StateDelta {
                let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
                if delta.domain_id == wire::STATE_DOMAIN_ACTOR_COOLDOWNS {
                    assert_eq!(delta.delta_type, wire::DELTA_TYPE_ACTOR_COOLDOWNS_V1);
                    assert!(!saw_result, "cooldown must arrive with the original result");
                    deltas.push(wire::decode_actor_cooldowns(delta.payload).map_err(debug)?);
                }
            }
            if case == 101 && envelope.message_type() == MessageType::LivenessProbe {
                let probe = oteryn_protocol_oteryn::decode_liveness_probe(envelope.payload()).map_err(debug)?;
                clock.set(1_250_000);
                let ack = oteryn_protocol_oteryn::encode_liveness_ack(1, probe, 0).map_err(debug)?;
                crate::gameplay_transport::tcp_tls::write_frame(&mut client, &ack).await?;
                send(&mut client, 1, barrier, 65_535, &[]).await?;
            }
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                if result.command_id == barrier {
                    break;
                }
                assert_eq!(result.command_id, 1);
                assert_eq!(result.status, CommandStatus::Accepted);
                assert_eq!(actor_spell::decode_spell_cast_result(result.payload).map_err(debug)?,
                    actor_spell::SpellCastDisposition::Cast);
                saw_result = true;
            }
        }
        if case == 105 {
            assert_eq!(snapshot.entries, expected(spell, wire::MAX_CLOCK_MS, 3_000));
            assert!(deltas.is_empty());
        } else {
            assert!(saw_result);
            if !matches!(case, 104 | 106) {
                assert_eq!(deltas.len(), 1, "one delta before the later command result");
                assert_eq!(deltas[0].rtt_ms, None);
                assert_eq!(deltas[0].entries, expected(spell, 1_000, 1_000));
                assert_eq!(deltas[0].server_now_ms, 0);
            }
        }
        client.shutdown().await?;
        Ok(())
    };
    let (end, result): (_, TestResult) =
        tokio::time::timeout(Duration::from_secs(45), join_two(serving, driving)).await?;
    result?;
    let ConnectionEnd::AdmittedThenDisconnected(retained) = end else {
        return Err(format!("cast connection ended: {end:?}").into());
    };
    if !matches!(case, 104 | 106) {
        return Ok(());
    }

    assert_eq!(owner.lose_control(retained, Duration::ZERO, None).await, ControlLossResult::Recorded);
    let token = super::super::house_transfer_fixtures::RecoverySource::token(2)?;
    let resumed = owner.resume(ResumeAttempt {
        game_session_id: session,
        recovery_material: token.as_bytes(),
        transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[2; 16]).map_err(debug)?,
        last_applied_server_sequence: retained.continuity.server_sequence,
        supported_capabilities: supported,
    }).await.map_err(debug)?;
    assert_eq!(resumed.continuity.connection_generation, 2);
    clock.set(1_250_000);
    let expected_entries = expected(spell, 2_000, 2_000);
    let (snapshot, mut continued) = rejoin(owner, resumed, case == 104, true).await?;
    assert_eq!(snapshot.entries, expected_entries);
    assert_eq!(snapshot.server_now_ms, 1_250);
    assert_eq!(snapshot.rtt_ms, None);
    if case == 106 {
        let fence = continued.item_fence.as_mut().ok_or("recovered fence")?;
        fence.character_lease_generation = fence
            .character_lease_generation
            .checked_sub(1)
            .ok_or("stale lease generation")?;
        let (stale, _) = rejoin(owner, continued, false, false).await?;
        assert!(stale.entries.is_empty(), "a stale lease read committed cooldowns");
    }
    Ok(())
}


async fn gift(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    book: &crate::spell::SpellBook,
    spell: NonZeroU32,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("recipient")?;
    let session = admitted.game_session_id;
    let key = &book.source_indexed(spell).ok_or("spell")?.0.key;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS);
    let driving = async {
        join_cooldowns(&mut client).await?;
        let (health_revision, rescued_health) = {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states.get_mut(&runtime, actor, session).ok_or("recipient state")?;
            let facts = crate::spell::cast::CharacterCastFacts {
                vocation: crate::spell::Vocation::EliteKnight,
                level: oteryn_gameplay_progression::wheel::WHEEL_LEVEL_OFFSET + 1,
                ..state.character_facts()
            };
            let mut before = crate::spell::cast::PlayerSpellState::new(facts, 0, 0)
                .ok_or("Wheel-eligible recipient")?;
            before.apply_owner_promotion_transition(true, true, Some(u64::MAX)).map_err(debug)?;
            let mut effects = oteryn_gameplay_progression::wheel_effects::WheelRuntimeEffects::default();
            effects.revelation_stages.insert("gift_of_life".into(), 1);
            before.install_wheel_effects(effects);
            assert!(before.restore_wheel_gift(0, 0));
            before.install_wheel_gift_spellbook(book);
            before.restore_saved_vitals(10, 0);
            before.cooldowns.spells.insert(key.clone(), SemanticTimeMicros::from_micros(91_000_000));
            *state = before;
            let hit = states.apply_creature_damage_native(
                &runtime, actor, session, 10, crate::foundation::owner_timer::SemanticTimeMicros::from_micros(1_000_000),
            ).ok_or("Gift of Life hit")?;
            assert!(hit.damage.health_after > 0);
            let after = states.get(&runtime, actor, session).ok_or("rescued actor")?;
            assert_eq!(after.cooldowns.spells[key].get(), 31_000_000);
            (hit.vitals_revision, hit.damage.health_after)
        };
        let mut cooldowns = Vec::new();
        let mut health_seen = false;
        let mut cooldown_sequence = None;
        let mut health_sequence = None;
        loop {
            let frame = read(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            match envelope.message_type() {
                MessageType::StateDelta => {
                    let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
                    if delta.domain_id == wire::STATE_DOMAIN_ACTOR_COOLDOWNS {
                        assert!(!health_seen, "rescued vitals arrived before their cooldown");
                        cooldown_sequence = Some(envelope.server_sequence());
                        cooldowns.extend(wire::decode_actor_cooldowns(delta.payload).map_err(debug)?.entries);
                    }
                    if delta.domain_id == actor_spell::STATE_DOMAIN_ACTOR_VITALS
                        && delta.new_revision >= health_revision && !health_seen
                    {
                        let value = actor_spell::decode_actor_vitals(delta.payload).map_err(debug)?;
                        assert_eq!(value.health, rescued_health);
                        health_sequence = Some(envelope.server_sequence());
                        health_seen = true;
                        send(&mut client, 1, 1, 65_535, &[]).await?;
                    }
                }
                MessageType::CommandResult => break,
                _ => {}
            }
        }
        assert!(health_seen);
        assert_eq!(cooldowns, vec![wire::ActorCooldown {
            kind: wire::ActorCooldownKind::Spell, id: spell.get(), expires_at_ms: 30_000,
        }]);
        assert_eq!(
            cooldown_sequence.and_then(|seq| seq.checked_add(1)),
            health_sequence,
            "the recipient's cooldown and rescued health are consecutive frames",
        );
        client.shutdown().await?;
        Ok(())
    };
    tokio::time::timeout(Duration::from_secs(20), join_two(serving, driving)).await?.1
}

fn expected(spell: NonZeroU32, spell_ms: u64, group_ms: u64) -> Vec<wire::ActorCooldown> {
    vec![
        wire::ActorCooldown {
            kind: wire::ActorCooldownKind::Spell,
            id: spell.get(),
            expires_at_ms: spell_ms,
        },
        wire::ActorCooldown {
            kind: wire::ActorCooldownKind::Group,
            id: 2,
            expires_at_ms: group_ms,
        },
    ]
}

async fn rejoin(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    hold_durability: bool,
    read_barrier: bool,
) -> TestResult<(wire::ActorCooldowns, AdmittedSession)> {
    let held = if hold_durability {
        owner.root.await_ready().await?;
        Some(owner.root.try_issue_background_semantic_pass()?)
    } else {
        None
    };
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async { serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await };
    let driving = async {
        let (_, snapshot) = join_cooldowns(&mut client).await?;
        if let Some(held) = held {
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
        }
        if !read_barrier {
            // The stale lease may also be refused by command ingress. This leg's
            // barrier is the committed snapshot, after the valid leg's command barrier.
            client.shutdown().await?;
            return Ok(snapshot);
        }
        let barrier = admitted.continuity.next_command_id;
        send(&mut client, 2, barrier, 65_535, &[]).await?;
        loop {
            let bytes = read(&mut client).await?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            if envelope.message_type() == MessageType::StateDelta {
                let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
                assert_ne!(delta.domain_id, wire::STATE_DOMAIN_ACTOR_COOLDOWNS);
            }
            if envelope.message_type() == MessageType::CommandResult {
                assert_eq!(decode_command_result(envelope.payload()).map_err(debug)?.command_id, barrier);
                break;
            }
        }
        client.shutdown().await?;
        Ok(snapshot)
    };
    let (end, snapshot): (_, TestResult<wire::ActorCooldowns>) =
        tokio::time::timeout(Duration::from_secs(20), join_two(serving, driving)).await?;
    let ConnectionEnd::AdmittedThenDisconnected(continued) = end else {
        return Err(format!("rejoin connection ended: {end:?}").into());
    };
    Ok((snapshot?, continued))
}

async fn join_cooldowns(client: &mut DuplexStream) -> TestResult<(u64, wire::ActorCooldowns)> {
    let mut snapshot = None;
    loop {
        let bytes = read(client).await?;
        let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
        match envelope.message_type() {
            MessageType::SnapshotChunk => {
                for row in decode_snapshot_chunk(envelope.payload()).map_err(debug)?.1 {
                    if row.domain_id == wire::STATE_DOMAIN_ACTOR_COOLDOWNS {
                        snapshot = Some((row.revision, wire::decode_actor_cooldowns(row.payload).map_err(debug)?));
                    }
                }
            }
            MessageType::SnapshotCommit => return snapshot.ok_or_else(|| "no cooldown snapshot".into()),
            _ => {}
        }
    }
}

async fn send(client: &mut DuplexStream, generation: u64, command_id: u64, command_type: u32, payload: &[u8]) -> TestResult {
    let frame = encode_client_command(generation, &ClientCommandValue { command_id, command_type, payload }).map_err(debug)?;
    crate::gameplay_transport::tcp_tls::write_frame(client, &frame).await?;
    Ok(())
}
