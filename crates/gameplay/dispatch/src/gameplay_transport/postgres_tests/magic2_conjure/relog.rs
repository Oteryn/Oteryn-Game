use super::super::house_transfer_fixtures::RecoverySource;
use super::*;
use crate::foundation::GameSessionId;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, ConnectionIdentifiers, FirstEntryOutcome, IDLE_LIVENESS,
    admit_frame, serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use oteryn_protocol_oteryn::{self as wire, CommandStatus, MessageType, actor_spell};
use tokio::io::AsyncWriteExt;

struct RelogIds;

impl ConnectionIdentifiers for RelogIds {
    fn game_session_id(&self) -> Option<GameSessionId> {
        GameSessionId::decode(&id(91)).ok()
    }

    fn transport_ref(&self) -> Option<crate::foundation::AuthenticatedTransportRefV1> {
        crate::foundation::AuthenticatedTransportRefV1::decode(&[91; 16]).ok()
    }
}

pub(super) async fn expire_saved_cooldowns(h: &Harness) -> TestResult {
    let entries: serde_json::Value = sqlx::query_scalar(
        "SELECT snapshot->'spell_cooldowns_unix_us'
         FROM game_character_condition_snapshots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?;
    let entries = entries
        .as_array()
        .ok_or("saved spell and group deadlines")?;
    assert_eq!(entries.len(), 2);
    let expired: Vec<_> = entries
        .iter()
        .map(|row| serde_json::json!([row[0], 1]))
        .collect();
    let updated = sqlx::query(
        "UPDATE game_character_condition_snapshots
         SET snapshot=jsonb_set(snapshot, '{spell_cooldowns_unix_us}', $1)
         WHERE character_id=encode($2,'hex')::uuid",
    )
    .bind(serde_json::Value::Array(expired))
    .bind(id(41).as_slice())
    .execute(&h.pool)
    .await?;
    assert_eq!(updated.rows_affected(), 1);
    Ok(())
}

pub(super) async fn admit_and_round_trip(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    old: AdmittedSession,
    cast: SpellCastIntent,
    expired: bool,
) -> TestResult {
    use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
    use wire::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES;

    let new_session = RelogIds.game_session_id().ok_or("relog session")?;
    assert!(
        AdmissionRetryBudget::for_root(&new_session, owner.root)
            .retry_until(async || owner.ensure_source_map_initialized().await)
            .await,
        "source map before fresh admission"
    );
    let token = RecoverySource::fresh_token(91)?;
    let bootstrap = wire::encode_client_bootstrap(&wire::ClientBootstrapValue {
        schema_revision: 1,
        character_id: crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
        admission_material: token.as_bytes(),
        client_build_id: "w09-relog-wire",
        supported_capabilities: &[],
    })?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let mut admitted = admit_frame(&mut server, &bootstrap, owner, &RelogIds)
        .await
        .map_err(debug)?;
    let accepted = read_frame(&mut client).await?;
    assert_eq!(
        wire::decode_wire_envelope(&accepted)?.message_type(),
        MessageType::ServerAccepted
    );
    assert_ne!(admitted.game_session_id, old.game_session_id);
    assert!(
        matches!(
            admitted.first_entry,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        ),
        "{:?}",
        admitted.first_entry,
    );
    assert!(
        admitted
            .item_fence
            .ok_or("relog fence")?
            .character_lease_generation
            > old
                .item_fence
                .ok_or("old fence")?
                .character_lease_generation,
    );
    // Same presentation negotiation as the existing canary conjure fixture.
    let spatial = CAPABILITY_WORLD_SPATIAL_ENTITIES;
    let presentation = cooldown_wire::CAPABILITY_PRESENTATION_V1;
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        &[
            OfferedCapability {
                id: spatial,
                requires: &[],
            },
            OfferedCapability {
                id: presentation,
                requires: &cooldown_wire::CAPABILITY_PRESENTATION_V1_REQUIRES,
            },
        ],
        &[spatial, presentation],
    )
    .ok_or("relog presentation capabilities")?;
    assert!(
        owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .require()
            .is_ok()
    );
    let expected_vitals = owner
        .observe_vitals(
            admitted.runtime_actor.ok_or("relog caster")?,
            admitted.game_session_id,
        )
        .await
        .ok_or("relog vitals")?
        .1;

    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let driving = read_and_cast(&mut client, admitted, cast, expected_vitals, expired);
    let (end, result) = tokio::time::timeout(std::time::Duration::from_secs(45), async {
        tokio::join!(serving, driving)
    })
    .await?;
    result?;
    assert!(
        matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
        "{end:?}"
    );
    Ok(())
}

async fn read_and_cast(
    client: &mut tokio::io::DuplexStream,
    admitted: AdmittedSession,
    cast: SpellCastIntent,
    expected_vitals: actor_spell::ActorVitals,
    expired: bool,
) -> TestResult {
    let generation = admitted.continuity.connection_generation;
    let mut body = Vec::new();
    let mut snapshot_sequence = None;
    loop {
        let frame = read_frame(client).await?;
        let envelope = wire::decode_wire_envelope(&frame)?;
        assert_eq!(envelope.connection_generation(), generation);
        match envelope.message_type() {
            MessageType::SnapshotBegin => {
                snapshot_sequence =
                    Some(wire::decode_snapshot_begin(envelope.payload())?.target_server_sequence);
            }
            MessageType::SnapshotChunk => {
                body.extend_from_slice(wire::decode_snapshot_chunk_framing(envelope.payload())?.1);
            }
            MessageType::SnapshotCommit => break,
            _ => {}
        }
    }
    let mut sequence = snapshot_sequence.ok_or("relog snapshot sequence")?;
    assert_eq!(sequence, 0);
    let domains = wire::decode_snapshot_body(&body)?;
    let vitals = domains
        .iter()
        .find(|domain| domain.domain_id == actor_spell::STATE_DOMAIN_ACTOR_VITALS)
        .ok_or("relog vitals domain")?;
    assert_eq!(
        vitals.snapshot_type,
        actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1
    );
    assert_eq!(round_trip_vitals(vitals.payload)?, expected_vitals);
    let cooldowns = domains
        .iter()
        .find(|domain| domain.domain_id == cooldown_wire::STATE_DOMAIN_ACTOR_COOLDOWNS)
        .ok_or("relog cooldown domain")?;
    assert_eq!(
        cooldowns.snapshot_type,
        cooldown_wire::SNAPSHOT_TYPE_ACTOR_COOLDOWNS_V1
    );
    let snapshot = round_trip_cooldowns(cooldowns.payload)?;
    if expired {
        assert!(snapshot.entries.is_empty());
    } else {
        use cooldown_wire::ActorCooldownKind::{Group, Spell};
        assert!(
            snapshot
                .entries
                .iter()
                .any(|entry| entry.kind == Spell && entry.id == cast.spell.get())
        );
        // The two-second support group can elapse during admission. The
        // thirty-minute spell must still keep its original deadline.
        for entry in snapshot.entries {
            let maximum = match entry.kind {
                Spell => {
                    assert_eq!(entry.id, cast.spell.get());
                    1_800_000
                }
                Group => {
                    assert_eq!(entry.id, 3);
                    2_000
                }
            };
            let remaining = entry
                .expires_at_ms
                .checked_sub(snapshot.server_now_ms)
                .ok_or("negative cooldown")?;
            assert!((1..=maximum).contains(&remaining));
        }
    }

    let command_id = admitted.continuity.next_command_id;
    assert_eq!(command_id, 1);
    write_frame(
        client,
        &wire::encode_client_command(
            generation,
            &wire::ClientCommandValue {
                command_id,
                command_type: actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
                payload: &actor_spell::encode_spell_cast_intent(&cast),
            },
        )?,
    )
    .await?;
    client.shutdown().await?;
    let mut result_seen = false;
    let mut paid_vitals = None;
    loop {
        let frame = match read_frame(client).await {
            Ok(frame) => frame,
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error.into()),
        };
        let envelope = wire::decode_wire_envelope(&frame)?;
        assert_eq!(envelope.connection_generation(), generation);
        if matches!(
            envelope.message_type(),
            MessageType::CommandResult | MessageType::StateDelta
        ) {
            sequence += 1;
            assert_eq!(envelope.server_sequence(), sequence);
        }
        match envelope.message_type() {
            MessageType::CommandResult => {
                assert!(!result_seen);
                assert_eq!(sequence, 1);
                let result = wire::decode_command_result(envelope.payload())?;
                assert_eq!(result.command_id, command_id);
                assert_eq!(result.status, CommandStatus::Accepted);
                let disposition =
                    actor_spell::decode_spell_cast_result(result.payload).map_err(debug)?;
                assert_eq!(
                    actor_spell::encode_spell_cast_result(disposition),
                    result.payload
                );
                assert_eq!(
                    disposition,
                    if expired {
                        SpellCastDisposition::Cast
                    } else {
                        SpellCastDisposition::CoolingDown
                    }
                );
                result_seen = true;
            }
            MessageType::StateDelta => {
                let delta = wire::decode_state_delta(envelope.payload())?;
                match delta.domain_id {
                    actor_spell::STATE_DOMAIN_ACTOR_VITALS => {
                        assert!(result_seen);
                        assert_eq!(sequence, 2);
                        assert_eq!(delta.delta_type, actor_spell::DELTA_TYPE_ACTOR_VITALS_V1);
                        assert!(
                            paid_vitals
                                .replace(round_trip_vitals(delta.payload)?)
                                .is_none()
                        );
                    }
                    cooldown_wire::STATE_DOMAIN_ACTOR_COOLDOWNS => {
                        assert_eq!(
                            delta.delta_type,
                            cooldown_wire::DELTA_TYPE_ACTOR_COOLDOWNS_V1
                        );
                        round_trip_cooldowns(delta.payload)?;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    assert!(result_seen, "missing post-relog cast result");
    if expired {
        let paid = paid_vitals.ok_or("missing paid vitals")?;
        assert_eq!(
            paid.mana,
            expected_vitals.mana.checked_sub(250).ok_or("cast mana")?
        );
        assert_eq!(paid.max_mana, expected_vitals.max_mana);
        assert_eq!(paid.max_health, expected_vitals.max_health);
        assert_eq!(
            paid.soul, expected_vitals.soul,
            "Conjure Wand of Darkness only pays mana"
        );
    } else {
        assert!(
            paid_vitals.is_none(),
            "cooldown refusal has no vitals delta"
        );
    }
    Ok(())
}

fn round_trip_vitals(payload: &[u8]) -> TestResult<actor_spell::ActorVitals> {
    let value = actor_spell::decode_actor_vitals(payload).map_err(debug)?;
    assert_eq!(
        actor_spell::encode_actor_vitals(&value).map_err(debug)?,
        payload
    );
    assert_eq!((value.harmony, value.serene), (0, false), "Sorcerer vitals");
    assert!(value.health <= value.max_health && value.max_health <= actor_spell::MAX_VITAL_POOL);
    assert!(value.mana <= value.max_mana && value.max_mana <= actor_spell::MAX_VITAL_POOL);
    assert!(value.soul <= actor_spell::MAX_SOUL);
    Ok(value)
}

fn round_trip_cooldowns(payload: &[u8]) -> TestResult<cooldown_wire::ActorCooldowns> {
    let value = cooldown_wire::decode_actor_cooldowns(payload).map_err(debug)?;
    assert_eq!(
        cooldown_wire::encode_actor_cooldowns(&value).map_err(debug)?,
        payload
    );
    assert_eq!(value.rtt_ms, None);
    Ok(value)
}
