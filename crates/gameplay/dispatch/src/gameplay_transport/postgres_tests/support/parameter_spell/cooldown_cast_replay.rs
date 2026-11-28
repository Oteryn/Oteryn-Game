use super::super::autowalk_fixtures::{read, send};
use super::super::srx3_native_source_cast_fixtures::NativeOwnerClock;
use super::*;
use crate::gameplay_transport::native_cue_offer;
use oteryn_protocol_oteryn::tutorial_notices::{self, TutorialNotice};
use oteryn_protocol_oteryn::{decode_state_delta, presentation as wire};
use std::num::NonZeroU32;
use tokio::io::DuplexStream;

pub(super) async fn check(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    book: &crate::spell::SpellBook,
    presentation_negotiated: bool,
) -> TestResult {
    // Start at the admitted dispatch boundary with the qualification world's presentation offer.
    let offered = native_cue_offer::offered(owner.offered_capabilities(), true);
    let requested: &[u32] = if presentation_negotiated {
        &[4, 6, 7, wire::CAPABILITY_PRESENTATION_V1]
    } else {
        &[4, 6, 7]
    };
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(&offered, requested)
            .ok_or("spell, chat and presentation capability selection")?;
    assert_eq!(
        admitted.continuity.selected_capabilities.as_slice(),
        requested
    );
    let index = |words| -> TestResult<NonZeroU32> {
        let key = &book.spoken(words).ok_or("spoken spell")?.spell.key;
        (1..=u32::try_from(book.source_len())?)
            .filter_map(NonZeroU32::new)
            .find(|index| book.indexed(*index).is_some_and(|spell| &spell.key == key))
            .ok_or_else(|| "spell index absent".into())
    };
    let light_healing = index("exura")?;
    let assert_cast_cooldowns = |entries: Vec<_>, revision, now| {
        if presentation_negotiated {
            assert_cooldowns(entries, light_healing, revision, now);
        } else {
            assert!(
                entries.is_empty(),
                "cooldown delta without the presentation capability"
            );
        }
    };
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
        assert!(entries.is_empty(), "PZ refusal published a cooldown");

        let indexed = cast(light_healing);
        send(&mut client, 2, spell_command, &indexed).await?;
        let (status, payload, entries) = result(&mut client, 2).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            actor_spell::decode_spell_cast_result(&payload).map_err(debug)?,
            actor_spell::SpellCastDisposition::Cast
        );
        assert_cast_cooldowns(entries, 0, 1_000);
        send(&mut client, 2, spell_command, &indexed).await?;
        let (status, _, entries) = result(&mut client, 2).await?;
        assert_eq!(status, CommandStatus::DuplicateReplay);
        assert!(entries.is_empty(), "indexed replay published a cooldown");

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
        assert_cast_cooldowns(entries, 1, 3_000);
        send(&mut client, 3, chat::COMMAND_TYPE_CHAT_INTENT, &spoken).await?;
        let (status, _, entries) = result(&mut client, 3).await?;
        assert_eq!(status, CommandStatus::DuplicateReplay);
        assert!(entries.is_empty(), "spoken replay published a cooldown");

        // The next result brackets every frame sent after the spoken replay.
        send(&mut client, 4, spell_command, &cast(beam)).await?;
        let (status, _, entries) = result(&mut client, 4).await?;
        assert_eq!(status, CommandStatus::Rejected);
        assert!(entries.is_empty(), "spoken replay published a cooldown");
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

pub(super) async fn check_protection_zone(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    book: &crate::spell::SpellBook,
) -> TestResult {
    use oteryn_gameplay_spell_rules::cast::{STAIRHOP_COOLDOWN_KEY, STAIRHOP_MICROS};

    let offered = native_cue_offer::offered(owner.offered_capabilities(), true);
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        &offered,
        &[
            4,
            6,
            7,
            wire::CAPABILITY_PRESENTATION_V1,
            tutorial_notices::CAPABILITY_TUTORIAL_NOTICES_V1,
        ],
    )
    .ok_or("spell, presentation and status notice capabilities")?;
    let scorch = book.spoken("exevo infir flam hur").ok_or("Scorch")?.spell;
    assert!(scorch.aggressive);
    let spell = (1..=u32::try_from(book.source_len())?)
        .filter_map(NonZeroU32::new)
        .find(|&index| book.indexed(index) == Some(scorch))
        .ok_or("Scorch index")?;
    let actor = admitted.runtime_actor.ok_or("caster")?;
    let session = admitted.game_session_id;
    let clock = NativeOwnerClock::new(owner);
    clock.set(1_000_000);
    let before_cooldowns = {
        let mut runtime = owner.runtime.lock().await;
        assert_eq!(
            crate::gameplay_transport::actor_spell::qualified_protection_zone(
                &mut runtime,
                owner.movement_cells(),
                actor,
            ),
            Some(true)
        );
        let mut states = owner.states.spell_states.lock().await;
        let state = states
            .get_mut(&runtime, actor, session)
            .ok_or("spell state")?;
        let until = owner
            .owner_now()
            .checked_add(STAIRHOP_MICROS)
            .map_err(debug)?;
        state
            .cooldowns
            .groups
            .insert(STAIRHOP_COOLDOWN_KEY.into(), until);
        state.cooldowns.clone()
    };
    let before_vitals = owner
        .observe_vitals(actor, session)
        .await
        .ok_or("caster vitals")?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async { serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await };
    let driving = async move {
        while decode_wire_envelope(&read(&mut client).await?)
            .map_err(debug)?
            .message_type()
            != MessageType::SnapshotCommit
        {}
        let cast = actor_spell::encode_spell_cast_intent(&actor_spell::SpellCastIntent {
            spell,
            target: actor_spell::SpellTarget::None,
            aim_at_target: false,
        });
        let command = actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT;
        send(&mut client, 1, command, &cast).await?;
        let mut saw_result = false;
        let mut notices = Vec::new();
        loop {
            let bytes = read(&mut client).await?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            match envelope.message_type() {
                MessageType::StateDelta => {
                    let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
                    assert_ne!(delta.domain_id, actor_spell::STATE_DOMAIN_ACTOR_VITALS);
                    assert_ne!(delta.domain_id, wire::STATE_DOMAIN_ACTOR_COOLDOWNS);
                    if delta.domain_id == tutorial_notices::STATE_DOMAIN_TUTORIAL_NOTICES {
                        notices.push(
                            tutorial_notices::decode_tutorial_notice(delta.payload)
                                .map_err(debug)?,
                        );
                    }
                }
                MessageType::CommandResult => {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, 1);
                    if saw_result {
                        assert_eq!(result.status, CommandStatus::DuplicateReplay);
                        break;
                    }
                    assert_eq!(
                        result.status,
                        CommandStatus::Rejected,
                        "PZ takes precedence over stair-hop cooldown"
                    );
                    assert_eq!(
                        actor_spell::decode_spell_cast_result(result.payload).map_err(debug)?,
                        actor_spell::SpellCastDisposition::Rejected
                    );
                    saw_result = true;
                    // The duplicate result delimits the frames sent after the refusal.
                    send(&mut client, 1, command, &cast).await?;
                }
                _ => (),
            }
        }
        assert_eq!(
            notices,
            vec![TutorialNotice::StatusText(
                "This action is not permitted in a protection zone.".into()
            )]
        );
        assert_eq!(
            owner.observe_vitals(actor, session).await,
            Some(before_vitals),
            "PZ refusal changed mana or soul"
        );
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states
                .get(&runtime, actor, session)
                .ok_or("spell state after refusal")?;
            assert_eq!(state.cooldowns, before_cooldowns);
        }
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

fn assert_cooldowns(
    cooldowns: Vec<(u64, u64, wire::ActorCooldowns)>,
    spell: NonZeroU32,
    revision: u64,
    now: u64,
) {
    // TibiaWiki Light Healing: Exura starts 1000 ms for both the spell and its group.
    // The authored healing group has S9 ordinal 2.
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
