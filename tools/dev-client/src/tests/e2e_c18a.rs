//! C18a replay owner: `oteryn_session::Session::apply_domain_delta` applies the
//! ACTOR_COMBAT_STATE payload, then `protocol::run_cast` reads that state and
//! `protocol::cast_selected` applies the spell result to `SpellFeedback`.

use oteryn_client_runtime::combat::{CombatCommand, CombatReport, run_combat};
use oteryn_client_runtime::spell::SpellFeedback;
use oteryn_protocol_oteryn::actor_spell::{self, SpellCastIntent, SpellTarget};
use oteryn_protocol_oteryn::attack::{self, ActorCombatState, FightModes};
use oteryn_protocol_oteryn::{
    DomainSnapshot, ServerAcceptedValue, encode_command_result, encode_server_accepted,
    encode_single_chunk_snapshot, world_clock,
};
use oteryn_session::{Admission, CharacterId, CommandStatus, Session, read_frame, write_frame};
use std::num::NonZeroU32;
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;

struct Fixture {
    combat: Vec<u8>,
    result: Vec<u8>,
    spell: NonZeroU32,
    expected: oteryn_session::SpellCastDisposition,
    feedback: &'static str,
    live: Option<LiveExchange>,
}

struct LiveExchange {
    inbound: Vec<u8>,
    cast_send_offset: usize,
    intent: Vec<u8>,
    session: oteryn_protocol_oteryn::GameSessionId,
    command_id: u64,
    preceding_sequence: u64,
    vitals_before: Vec<u8>,
    vitals_revision: u64,
    clock_before: Option<(u64, Vec<u8>)>,
    charged: actor_spell::ActorVitals,
    combat_after_exchange: Option<(u64, ActorCombatState)>,
}

fn fixture() -> Result<Fixture, Error> {
    use oteryn_session::SpellCastDisposition;
    let target = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: [7; 16],
        generation: 3,
    };
    Ok(Fixture {
        combat: attack::encode_actor_combat_state(&ActorCombatState {
            target: Some(target),
            modes: FightModes::DEFAULT,
            in_fight: true,
        })
        .map_err(|e| format!("{e:?}"))?,
        result: actor_spell::encode_spell_cast_result(SpellCastDisposition::TargetIllegal),
        spell: NonZeroU32::MIN,
        expected: SpellCastDisposition::TargetIllegal,
        feedback: "Illegal target",
        live: None,
    })
}

fn id(marker: u8) -> [u8; 16] {
    [
        1, 0x93, 0x4f, 0x10, 0x7c, 0, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        marker,
    ]
}

#[ignore = "requires an active cloud node-boot capture"]
#[test]
fn e2e_c18a_replays_captured_damage_or_unit_refusal() -> Result<(), Error> {
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR")
        .ok_or("C18a live replay requires E2E_ALL_CAPTURE_DIR")?;
    replay_capture(std::path::Path::new(&root))?;
    synthetic_refusal_keeps_combat_state()
}

#[test]
fn synthetic_refusal_keeps_combat_state() -> Result<(), Error> {
    replay_through_session(fixture()?)
}

async fn replay_wire_frames(
    peer: &mut tokio::io::DuplexStream,
    mut inbound: &[u8],
    generation: u64,
    last_sequence: &mut u64,
) -> Result<(), Error> {
    use oteryn_protocol_oteryn::{MessageType, decode_liveness_probe, decode_wire_envelope};
    use tokio::io::AsyncWriteExt;

    while !inbound.is_empty() {
        let framed = inbound;
        let frame = read_frame(&mut inbound).await?;
        let envelope = decode_wire_envelope(&frame)?;
        peer.write_all(&framed[..framed.len() - inbound.len()])
            .await?;
        if envelope.message_type() == MessageType::LivenessProbe {
            let probe = decode_liveness_probe(envelope.payload())?;
            let reply = tokio::time::timeout(Duration::from_secs(5), read_frame(peer)).await??;
            let acknowledgement = decode_wire_envelope(&reply)?.liveness_ack(generation)?;
            assert_eq!(acknowledgement.probe_id, probe);
            assert_eq!(
                acknowledgement.last_applied_server_sequence,
                Some(*last_sequence)
            );
        } else {
            *last_sequence = envelope.server_sequence();
        }
    }
    Ok(())
}

fn replay_through_session(fixture: Fixture) -> Result<(), Error> {
    let Fixture {
        combat,
        result,
        spell,
        expected,
        feedback: expected_feedback,
        live,
    } = fixture;
    assert!(attack::decode_actor_combat_state(&[0xff]).is_err());
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), Error> {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async move {
                    let charged = live.as_ref().map(|live| live.charged);
                    let combat_after_exchange = live.as_ref().and_then(|live| live.combat_after_exchange);
                    let selected: &[u32] = if live.as_ref().is_some_and(|live| live.clock_before.is_some()) {
                        &[6, 17, world_clock::CAPABILITY_WORLD_LIGHT_V1]
                    } else {
                        &[6, 17]
                    };
                    let (generation, command_id, sequence, session_id) = if let Some(live) = &live {
                        let frame = read_frame(&mut live.inbound.as_slice()).await?;
                        let generation = oteryn_protocol_oteryn::decode_wire_envelope(&frame)?.connection_generation();
                        (generation, live.command_id, live.preceding_sequence, live.session)
                    } else {
                        (1, 1, 1, oteryn_protocol_oteryn::GameSessionId::decode(&id(1))?)
                    };
                    let (client, mut peer) = tokio::io::duplex(64 * 1024);
                    let server = tokio::spawn(async move {
                        read_frame(&mut peer).await?;
                        write_frame(
                            &mut peer,
                            &encode_server_accepted(&ServerAcceptedValue {
                                game_session_id: session_id,
                                world_id: oteryn_protocol_oteryn::WorldId::decode(&id(2))?,
                                channel_id: oteryn_protocol_oteryn::ChannelId::decode(&id(3))?,
                                connection_generation: generation,
                                current_server_sequence: sequence - 1,
                                next_command_id: command_id,
                                schema_revision: 1,
                                selected_capabilities: selected,
                            })?,
                        )
                        .await?;
                        let position = oteryn_protocol_oteryn::world_spatial::ActorPosition {
                            x: 100,
                            y: 200,
                            floor: 7,
                        };
                        let spatial = oteryn_protocol_oteryn::world_spatial_entities::encode_world_spatial_entities_snapshot(
                            &oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntitiesSnapshot {
                                content_generation: [1; 32], actor_position: position,
                                own_identity: [5; 16],
                                entities: vec![oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntity {
                                    motion: None,
                                    entity: oteryn_protocol_oteryn::world_spatial_entities::EntityRef { identity: [5; 16], generation: 1 },
                                    kind: oteryn_protocol_oteryn::world_spatial_entities::EntityKind::Player,
                                    display_name: None, position,
                                    detail: oteryn_protocol_oteryn::world_spatial_entities::EntityDetail::Actor {
                                        direction: oteryn_protocol_oteryn::world_spatial::StepDirection::South,
                                        appearance_ref: 1, health_percent: 100,
                                    },
                                }],
                            },
                        ).map_err(|error| format!("{error:?}"))?;
                        let overlay = oteryn_protocol_oteryn::world_object::encode_world_object_overlay_snapshot(&[])
                            .map_err(|error| format!("{error:?}"))?;
                        let mut domains = vec![
                                DomainSnapshot { domain_id: oteryn_protocol_oteryn::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, revision: 1, snapshot_type: oteryn_protocol_oteryn::world_spatial_entities::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, payload: &spatial },
                                DomainSnapshot { domain_id: oteryn_protocol_oteryn::world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY, revision: 1, snapshot_type: oteryn_protocol_oteryn::world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, payload: &overlay },
                                DomainSnapshot { domain_id: attack::STATE_DOMAIN_ACTOR_COMBAT_STATE, revision: combat_after_exchange.map_or(1, |(revision, _)| revision), snapshot_type: attack::SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1, payload: &combat },
                        ];
                        if let Some(live) = &live {
                            domains.push(DomainSnapshot {
                                domain_id: actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                                revision: live.vitals_revision,
                                snapshot_type: actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                                payload: &live.vitals_before,
                            });
                            if let Some((revision, payload)) = &live.clock_before {
                                domains.push(DomainSnapshot {
                                    domain_id: world_clock::STATE_DOMAIN_WORLD_CLOCK,
                                    revision: *revision,
                                    snapshot_type: world_clock::SNAPSHOT_TYPE_WORLD_CLOCK_V1,
                                    payload,
                                });
                            }
                        }
                        for frame in encode_single_chunk_snapshot(generation, 1, sequence, &domains)? {
                            write_frame(&mut peer, &frame).await?;
                        }
                        if let Some(live) = &live {
                            let (before_cast, after_cast) = live.inbound
                                .split_at_checked(live.cast_send_offset)
                                .ok_or("C18a replay cast boundary is outside the raw exchange")?;
                            let mut last_sequence = sequence;
                            replay_wire_frames(&mut peer, before_cast, generation, &mut last_sequence).await?;
                            let frame = read_frame(&mut peer).await?;
                            let command = oteryn_protocol_oteryn::decode_wire_envelope(&frame)?
                                .client_command(generation)?;
                            assert_eq!(command.command_id, live.command_id);
                            assert_eq!(command.payload, live.intent);
                            replay_wire_frames(&mut peer, after_cast, generation, &mut last_sequence).await?;
                            return Ok::<_, Error>(());
                        }
                        let frame = read_frame(&mut peer).await?;
                        let command = oteryn_protocol_oteryn::decode_wire_envelope(&frame)?
                            .client_command(generation)?;
                        let intent = actor_spell::decode_spell_cast_intent(command.payload)
                            .map_err(|e| format!("{e:?}"))?;
                        assert_eq!(
                            intent,
                            SpellCastIntent {
                                spell,
                                target: SpellTarget::AttackTarget,
                                aim_at_target: true
                            }
                        );
                        write_frame(
                            &mut peer,
                            &encode_command_result(
                                1,
                                2,
                                command.command_id,
                                CommandStatus::Accepted,
                                &result,
                            )?,
                        )
                        .await?;
                        Ok::<_, Error>(())
                    });
                    let mut session = Session::admit(
                        client,
                        Admission {
                            appearance_items: None,
                            schema_revision: 1,
                            character_id: CharacterId::decode(&id(4))?,
                            admission_material: b"grant",
                            client_build_id: "c18a-replay",
                            supported_capabilities: selected,
                            deadline: Duration::from_secs(5),
                        },
                    )
                    .await?;
                    let before = session.combat_state().cloned();
                    let mut feedback = SpellFeedback::default();
                    let report = run_combat(
                        &mut session,
                        &mut feedback,
                        CombatCommand::Cast(spell),
                    )
                    .await?;
                    assert!(matches!(report, CombatReport::Cast(_)));
                    let CombatReport::Cast(disposition) = report else { return Err("cast returned a non-cast report".into()); };
                    assert_eq!(disposition, expected);
                    assert_eq!(feedback.text(), Some(expected_feedback));
                    assert_eq!(session.combat_state(), combat_after_exchange.as_ref().map(|(_, state)| state).or(before.as_ref()));
                    if let Some(charged) = charged {
                        assert_eq!(session.actor_vitals(), Some(&charged));
                    }
                    server.await??;
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "C18a replay thread panicked")?
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    command_id: u64,
    game_session_id: String,
    payload: Vec<u8>,
    server_sequence: u64,
    status: u32,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Proof {
    scenario: String,
    wrapper: String,
    regeneration: Vec<(u64, u32, u32, u64)>,
    logout_history: Vec<(u64, u32, u32, u64)>,
    conjure_debit_sequence: u64,
    capture_device: u64,
    capture_inode: u64,
    spell: u32,
    damage: u64,
    damage_sequence: u64,
    caster_identity: [u8; 16],
    caster_generation: u64,
    rat_position: (i32, i32, i16),
    mana_before: u32,
    vitals_before: Vec<u8>,
    mana_after: u32,
    mana_relog: u32,
    conjure_sequence: u64,
    mana_before_conjure: u32,
    soul_before_conjure: u32,
    mana_logout: u32,
    soul_logout: u32,
    pre_cast_sequence: u64,
    cast_send_offset: usize,
    vitals_sequence: u64,
    logout_sequence: u64,
    soul_before: u32,
    soul_after: u32,
    soul_relog: u32,
    clock_before: Option<(u64, Vec<u8>)>,
    combat_before: Vec<u8>,
    clear_sequence: u64,
    cleared_combat: Vec<u8>,
    combat_after: Vec<u8>,
    game_session_id: String,
    relog_session_id: String,
    command_id: u64,
    cast_sequence: u64,
    health_sequence: u64,
    identity: [u8; 16],
    generation: u64,
    before: u8,
    after: u8,
    capabilities: Vec<u32>,
}

fn session_id(text: &str) -> Result<oteryn_protocol_oteryn::GameSessionId, Error> {
    let uuid = uuid::Uuid::parse_str(text)?;
    if uuid.to_string() != text {
        return Err("C18a session must use canonical UUIDv7 text".into());
    }
    Ok(oteryn_protocol_oteryn::GameSessionId::decode(
        uuid.as_bytes(),
    )?)
}

fn replay_capture(root: &std::path::Path) -> Result<(), Error> {
    use oteryn_protocol_oteryn::world_spatial_entities as wire;
    let cast: Envelope = serde_json::from_slice(&std::fs::read(root.join("c18a-cast.json"))?)?;
    let health: Envelope = serde_json::from_slice(&std::fs::read(root.join("c18a-health.json"))?)?;
    let vitals: Envelope = serde_json::from_slice(&std::fs::read(root.join("c18a-vitals.json"))?)?;
    let lock: Envelope = serde_json::from_slice(&std::fs::read(root.join("c18a-lock.json"))?)?;
    let damage: Envelope = serde_json::from_slice(&std::fs::read(root.join("c18a-damage.json"))?)?;
    let proof: Proof = serde_json::from_slice(&std::fs::read(root.join("c18a-proof.json"))?)?;
    verify_capture_directory(root, &proof)?;
    let session = session_id(&proof.game_session_id)?;
    if session_id(&cast.game_session_id)? != session
        || session_id(&health.game_session_id)? != session
        || session_id(&damage.game_session_id)? != session
        || session_id(&vitals.game_session_id)? != session
        || session_id(&lock.game_session_id)? != session
        || lock.command_id != proof.command_id
        || lock.status != cast.status
        || lock.server_sequence <= proof.pre_cast_sequence
        || lock.server_sequence >= proof.clear_sequence
        || vitals.command_id != proof.command_id
        || vitals.status != cast.status
        || vitals.server_sequence != proof.vitals_sequence
        || !(proof.pre_cast_sequence < cast.server_sequence
            && proof.pre_cast_sequence < vitals.server_sequence
            && cast.server_sequence != vitals.server_sequence
            && vitals.server_sequence < proof.conjure_sequence
            && proof.conjure_sequence < proof.conjure_debit_sequence
            && proof.conjure_debit_sequence <= proof.logout_sequence)
        || damage.command_id != proof.command_id
        || damage.status != cast.status
        || damage.server_sequence != proof.damage_sequence
        || damage.server_sequence <= cast.server_sequence
        || session_id(&proof.relog_session_id)? == session
        || proof.command_id == 0
        || cast.command_id != proof.command_id
        || health.command_id != proof.command_id
        || cast.server_sequence == 0
        || cast.server_sequence != proof.cast_sequence
        || health.server_sequence != proof.health_sequence
        || health.server_sequence <= cast.server_sequence
        || cast.status != CommandStatus::Accepted as u32
        || health.status != cast.status
        || !(vitals.server_sequence < proof.clear_sequence
            && proof.clear_sequence < proof.conjure_sequence)
        || proof.generation == 0
        || proof.before > 100
        || proof.after == 0
        || proof.after >= proof.before
        || !exact_rat_loss(proof.before, proof.after, proof.damage)
        || !exact_vitals(
            [
                proof.mana_before,
                proof.mana_after,
                proof.mana_logout,
                proof.mana_relog,
            ],
            [
                proof.soul_before,
                proof.soul_after,
                proof.soul_logout,
                proof.soul_relog,
            ],
        )
    {
        return Err("C18a capture provenance, sequence, or health proof is invalid".into());
    }
    let charged =
        actor_spell::decode_actor_vitals(&vitals.payload).map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        (charged.mana, charged.soul),
        (proof.mana_after, proof.soul_after)
    );
    if proof.mana_logout >= charged.max_mana || proof.soul_logout >= 100 {
        return Err("C18a full logout vitals cannot catch a reset".into());
    }
    if !crate::qualification_mana_history(
        (vitals.server_sequence, charged.mana, charged.soul),
        &proof.regeneration,
        charged.max_mana,
        (
            proof.conjure_sequence.saturating_add(1),
            proof.mana_before_conjure,
            proof.soul_before_conjure,
        ),
    ) {
        return Err("C18a replay found unexplained mana or soul before logout".into());
    }
    if !proof.logout_history.starts_with(&proof.regeneration)
        || proof
            .logout_history
            .get(proof.regeneration.len())
            .map(|row| row.0)
            != Some(proof.conjure_debit_sequence)
        || !crate::qualification_mana_history_with_conjure(
            (vitals.server_sequence, charged.mana, charged.soul),
            &proof.logout_history,
            charged.max_mana,
            (
                proof.logout_sequence.saturating_add(1),
                proof.mana_logout,
                proof.soul_logout,
            ),
            proof.conjure_debit_sequence,
        )
    {
        return Err("C18a replay found unexplained mana or soul after conjure".into());
    }
    let intent = std::fs::read(root.join("c18a-intent.bin"))?;
    verify_aimed_intent(&intent, proof.spell)?;
    let before = attack::decode_actor_combat_state(&proof.combat_before)
        .map_err(|error| format!("{error:?}"))?;
    let after = attack::decode_actor_combat_state(&proof.combat_after)
        .map_err(|error| format!("{error:?}"))?;
    let rat = wire::EntityRef {
        identity: proof.identity,
        generation: proof.generation,
    };
    assert_eq!(before.target, Some(rat));
    assert_eq!(after.target, Some(rat));
    assert_eq!(before.modes, after.modes);
    assert!(!before.in_fight && after.in_fight);
    assert_eq!(
        attack::decode_actor_combat_state(&lock.payload).map_err(|error| format!("{error:?}"))?,
        after
    );
    let cleared = attack::decode_actor_combat_state(&proof.cleared_combat)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(cleared.target, None);
    assert_eq!(cleared.modes, before.modes);
    let inbound = std::fs::read(root.join("c18a-exchange.bin"))?;
    let vitals_before = actor_spell::decode_actor_vitals(&proof.vitals_before)
        .map_err(|error| format!("{error:?}"))?;
    let (vitals_revision, combat_after_exchange) = cast_exchange_revision(
        &inbound,
        &cast,
        &vitals,
        proof.pre_cast_sequence,
        proof.cast_send_offset,
        vitals_before,
    )?;
    replay_through_session(Fixture {
        combat: proof.combat_before,
        result: cast.payload,
        spell: NonZeroU32::new(proof.spell).ok_or("C18a invalid spell index")?,
        expected: oteryn_session::SpellCastDisposition::Cast,
        feedback: "Spell cast",
        live: Some(LiveExchange {
            inbound,
            cast_send_offset: proof.cast_send_offset,
            intent,
            session,
            command_id: cast.command_id,
            preceding_sequence: proof.pre_cast_sequence,
            vitals_before: proof.vitals_before,
            vitals_revision,
            clock_before: proof.clock_before,
            charged,
            combat_after_exchange,
        }),
    })?;

    let fields = wire::EntityFields::selected(&proof.capabilities);
    let (delta, _, _) =
        wire::decode_world_spatial_entities_delta_with_sidecar(&health.payload, fields)
            .map_err(|error| format!("{error:?}"))?;
    let target = wire::EntityRef {
        identity: proof.identity,
        generation: proof.generation,
    };
    let caster = wire::EntityRef {
        identity: proof.caster_identity,
        generation: proof.caster_generation,
    };
    let events = oteryn_protocol_oteryn::presentation::decode_presentation_batch(&damage.payload)
        .map_err(|error| format!("{error:?}"))?;
    use oteryn_protocol_oteryn::world_spatial::ActorPosition;
    let origin = ActorPosition {
        x: 32405,
        y: 32204,
        floor: 7,
    };
    let rat_tile = ActorPosition {
        x: proof.rat_position.0,
        y: proof.rat_position.1,
        floor: proof.rat_position.2,
    };
    assert_eq!(
        crate::qualification_scorch_impact(&events, origin, caster, rat_tile, target, proof.spell)?,
        Some(proof.damage)
    );
    let entry = delta
        .update
        .iter()
        .find(|entry| entry.entity == target)
        .ok_or("C18a health capture lacks the exact Rat generation")?;
    assert_eq!(entry.kind, wire::EntityKind::Creature);
    assert_eq!(entry.position, rat_tile);
    assert!(
        matches!(entry.detail, wire::EntityDetail::Actor { health_percent, .. }
        | wire::EntityDetail::ActorWithIcons { health_percent, .. } if health_percent == proof.after)
    );
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("c18a-replay-consumed"))?;
    Ok(())
}

#[test]
fn strict_envelope_rejects_unknown_duplicate_and_missing_keys() -> Result<(), Error> {
    let base =
        r#"{"command_id":1,"game_session_id":"x","payload":[1],"server_sequence":2,"status":1}"#;
    assert!(serde_json::from_str::<Envelope>(base).is_ok());
    for extra in [r#","status":1}"#, r#","unexpected":0}"#] {
        let invalid = format!(
            "{}{extra}",
            base.strip_suffix('}')
                .ok_or("C18a envelope fixture lacks a closing brace")?
        );
        assert!(serde_json::from_str::<Envelope>(&invalid).is_err());
    }
    assert!(serde_json::from_str::<Envelope>(r#"{"payload":[1]}"#).is_err());
    Ok(())
}

fn verify_capture_directory(root: &std::path::Path, proof: &Proof) -> Result<(), Error> {
    use std::os::unix::fs::MetadataExt;
    if crate::qualification_runner_identity()? != proof.wrapper {
        return Err("C18a capture belongs to another node-boot invocation".into());
    }
    let dir = std::fs::metadata(root)?;
    if (dir.dev(), dir.ino()) != (proof.capture_device, proof.capture_inode) {
        return Err("C18a capture was copied out of its run directory".into());
    }
    let created = std::fs::metadata(root.join("c18a-proof.json"))?.modified()?;
    if created.elapsed()? > std::time::Duration::from_secs(600) {
        return Err("C18a capture expired before replay".into());
    }
    let name = root
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or("C18a replay lacks a run directory name")?;
    let parent = root
        .parent()
        .ok_or("C18a replay lacks a node-boot work directory")?;
    let binding = format!(
        "SEAM_EVIDENCE c18a_capture_binding directory={name} session={}",
        proof.game_session_id
    );
    let work = if proof.scenario == "C18a.full" {
        parent
            .parent()
            .ok_or("C18a full replay lacks a node-boot work directory")?
    } else {
        parent
    };
    let log = std::fs::read_to_string(work.join("stage-seam_stages.log"))?;
    if !valid_run_log(&log, &binding, &proof.scenario) {
        return Err("C18a node-boot seam log lacks one eligible current capture".into());
    }
    Ok(())
}

fn valid_run_log(log: &str, binding: &str, scenario: &str) -> bool {
    let captured = format!("SEAM_EVIDENCE e2e_capture scenario={scenario} status=CAPTURED ");
    matches!(scenario, "C18a.client" | "C18a.full")
        && log.lines().filter(|line| *line == binding).count() == 1
        && log
            .lines()
            .filter(|line| line.starts_with(&captured))
            .count()
            == 1
        && !log.lines().any(|line| line.contains("FIXTURE_RETRY "))
}

fn verify_aimed_intent(payload: &[u8], spell: u32) -> Result<(), Error> {
    let intent =
        actor_spell::decode_spell_cast_intent(payload).map_err(|error| format!("{error:?}"))?;
    if intent.spell.get() != spell
        || intent.target != SpellTarget::AttackTarget
        || !intent.aim_at_target
    {
        return Err("C18a outbound cast was did not aim at the selected opponent".into());
    }
    Ok(())
}

fn exact_rat_loss(before: u8, after: u8, damage: u64) -> bool {
    before == 100
        && damage > 0
        && after > 0
        && u64::from(before).checked_sub(u64::from(after)) == damage.checked_mul(5)
}

fn exact_vitals(mana: [u32; 4], soul: [u32; 4]) -> bool {
    mana[0].checked_sub(8) == Some(mana[1])
        && mana[2] == mana[3]
        && soul[0] == soul[1]
        && soul[2] == soul[3]
}

#[test]
fn rejects_false_c18a_damage_cost_and_aim() {
    let spell = NonZeroU32::MIN;
    for (target, aim_at_target, expected) in [
        (SpellTarget::None, false, false),
        (SpellTarget::AttackTarget, true, true),
    ] {
        let payload = actor_spell::encode_spell_cast_intent(&SpellCastIntent {
            spell,
            target,
            aim_at_target,
        });
        assert_eq!(verify_aimed_intent(&payload, spell.get()).is_ok(), expected);
    }
    assert!(exact_rat_loss(100, 60, 8));
    assert!(!exact_rat_loss(75, 50, 5));
    assert!(!exact_rat_loss(100, 60, 7));
    assert!(!exact_rat_loss(100, 100, 0));
    assert!(exact_vitals([30, 22, 29, 29], [0, 0, 0, 0]));
    assert!(!exact_vitals([30, 23, 29, 29], [0, 0, 0, 0]));
    assert!(!exact_vitals([30, 22, 29, 30], [0, 0, 0, 0]));
}

#[test]
fn scorch_requires_the_exact_fire_cone_and_cast_decision() -> Result<(), Error> {
    use oteryn_protocol_oteryn::presentation::{
        PresentationColor, PresentationEvent, PresentationEventKind as Kind, PresentationSource,
        PresentationValue, PresentationValueKind,
    };
    use oteryn_protocol_oteryn::world_spatial::ActorPosition;
    use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
    let origin = ActorPosition {
        x: 32405,
        y: 32204,
        floor: 7,
    };
    let rat_tile = ActorPosition { x: 32407, ..origin };
    let caster = EntityRef {
        identity: [1; 16],
        generation: 1,
    };
    let target = EntityRef {
        identity: [2; 16],
        generation: 1,
    };
    let spell = 13;
    let event = |kind| PresentationEvent {
        decision_ordinal: 1,
        emission_ordinal: 1,
        kind,
    };
    let words = event(Kind::SpellWords {
        caster,
        position: origin,
        spell,
    });
    let damage = |color| {
        event(Kind::ValueText {
            kind: PresentationValueKind::DamageDealt,
            position: rat_tile,
            target,
            attacker: Some(caster),
            primary: PresentationValue { value: 7, color },
            secondary: None,
        })
    };
    let mut events = vec![words, damage(PresentationColor::Orange)];
    for (dx, min, max) in [(1, 0, 0), (2, -1, 1), (3, -1, 1), (4, -2, 2)] {
        for dy in min..=max {
            events.push(event(Kind::MagicEffect {
                position: ActorPosition {
                    x: origin.x + dx,
                    y: origin.y + dy,
                    ..origin
                },
                effect_id: 16,
                source: PresentationSource::Own,
            }));
        }
    }
    let verify = |events: &[_]| {
        crate::qualification_scorch_impact(events, origin, caster, rat_tile, target, spell)
    };
    assert_eq!(verify(&events)?, Some(7));
    let mut moved = events.clone();
    let Kind::ValueText {
        ref mut position, ..
    } = moved[1].kind
    else {
        return Err("missing test damage event".into());
    };
    *position = ActorPosition {
        x: origin.x + 3,
        y: origin.y - 1,
        ..origin
    };
    let moved_tile = *position;
    let Err(mismatch) = verify(&moved) else {
        return Err("stale Rat position must fail".into());
    };
    assert!(mismatch.contains(&format!("position={rat_tile:?}")));
    assert!(mismatch.contains(&format!("position: {moved_tile:?}")));
    assert_eq!(
        crate::qualification_scorch_impact(&moved, origin, caster, moved_tile, target, spell)?,
        Some(7)
    );
    let Kind::ValueText {
        ref mut position, ..
    } = moved[1].kind
    else {
        return Err("missing test damage event".into());
    };
    *position = ActorPosition {
        x: origin.x + 5,
        ..origin
    };
    let outside = *position;
    assert!(
        crate::qualification_scorch_impact(&moved, origin, caster, outside, target, spell).is_err()
    );
    events.push(damage(PresentationColor::Orange));
    assert!(verify(&events).is_err());
    events.pop();
    events.push(event(Kind::MagicEffect {
        position: rat_tile,
        effect_id: 17,
        source: PresentationSource::Own,
    }));
    assert!(verify(&events).is_err());
    events.pop();
    events.push(event(Kind::ValueText {
        kind: PresentationValueKind::DamageDealt,
        position: rat_tile,
        target: EntityRef {
            identity: [3; 16],
            ..target
        },
        attacker: Some(caster),
        primary: PresentationValue {
            value: 1,
            color: PresentationColor::Orange,
        },
        secondary: None,
    }));
    assert!(verify(&events).is_err());
    events.pop();

    assert!(verify(&events[..3]).is_err());
    events.push(event(Kind::MagicEffect {
        position: ActorPosition {
            x: origin.x + 5,
            ..origin
        },
        effect_id: 16,
        source: PresentationSource::Own,
    }));
    assert!(verify(&events).is_err());
    events.pop();
    events[1] = damage(PresentationColor::Red);
    assert!(verify(&events).is_err());
    events[1] = damage(PresentationColor::Orange);
    events[1].decision_ordinal = 2;
    assert!(verify(&events).is_err());
    Ok(())
}

#[test]
fn rejects_unbound_or_retried_wrapper_log() {
    let binding = "SEAM_EVIDENCE c18a_capture_binding directory=captures.test session=test";
    let pass = "SEAM_EVIDENCE e2e_capture scenario=C18a.client status=CAPTURED evidence=live";
    assert!(valid_run_log(
        &format!("{binding}\n{pass}"),
        binding,
        "C18a.client"
    ));
    assert!(!valid_run_log(pass, binding, "C18a.client"));
    let full = pass.replace("C18a.client", "C18a.full");
    assert!(valid_run_log(
        &format!("{binding}\n{full}"),
        binding,
        "C18a.full"
    ));
    assert!(!valid_run_log(
        &format!("{binding}\n{pass}"),
        binding,
        "C18a.full"
    ));
    assert!(!valid_run_log(
        &format!("{binding}\n{binding}\n{pass}"),
        binding,
        "C18a.client"
    ));
    assert!(!valid_run_log(
        &format!("{binding}\nFIXTURE_RETRY step=logout reason=busy attempt=1\n{pass}"),
        binding,
        "C18a.client"
    ));
}

#[test]
fn rejects_an_unexplained_cast_refund() {
    let charged = (10, 22, 100);
    let logout = (20, 30, 100);
    let verified = [
        (12, 24, 100, 100),
        (14, 26, 100, 3_100),
        (16, 28, 100, 6_100),
        (18, 30, 100, 9_100),
    ];
    let mut early_credit = verified;
    early_credit[1].3 -= 201;
    assert!(!crate::qualification_mana_history(
        charged,
        &early_credit,
        30,
        logout
    ));
    assert!(crate::qualification_mana_history(
        charged, &verified, 30, logout
    ));
    // Receive-time jitter can shorten one measured gap without moving a server tick.
    let jittered = [
        (12, 24, 100, 3_934),
        (14, 26, 100, 14_554),
        (16, 28, 100, 21_691),
        (18, 30, 100, 24_663),
    ];
    assert!(crate::qualification_mana_history(
        charged, &jittered, 30, logout
    ));
    // Three early intervals cannot all fit one timeline within the sampling window.
    let too_fast = [
        (12, 24, 100, 1_000),
        (14, 26, 100, 3_900),
        (16, 28, 100, 6_800),
        (18, 30, 100, 9_700),
    ];
    assert!(!crate::qualification_mana_history(
        charged, &too_fast, 30, logout
    ));

    assert!(!crate::qualification_mana_history(charged, &[], 30, logout));
    assert!(!crate::qualification_mana_history(
        charged,
        &verified[..1],
        30,
        (20, 24, 100),
    ));
    assert!(!crate::qualification_mana_history(
        charged,
        &[(12, 30, 100, 6_000)],
        30,
        logout
    ));
}

#[test]
fn logout_vitals_require_the_exact_conjure_and_verified_food_credit() {
    use crate::qualification_mana_history_with_conjure as verify;
    let initial = (10, 292, 100);
    let history = [
        (12, 294, 100, 3_000),
        (14, 296, 100, 6_000),
        (16, 176, 99, 8_000),
        (18, 178, 99, 9_000),
    ];
    assert!(verify(initial, &history, 300, (20, 178, 99), 16));
    assert!(!verify(initial, &history, 300, (20, 180, 99), 16));
    let mut wrong_debit = history;
    wrong_debit[2].2 = 100;
    assert!(!verify(initial, &wrong_debit, 300, (20, 178, 99), 16));
    let mut early_credit = history;
    early_credit[3].3 = 8_500;
    assert!(!verify(initial, &early_credit, 300, (20, 178, 99), 16));
}

fn cast_exchange_revision(
    inbound: &[u8],
    cast: &Envelope,
    vitals: &Envelope,
    preceding_sequence: u64,
    cast_send_offset: usize,
    before: actor_spell::ActorVitals,
) -> Result<(u64, Option<(u64, ActorCombatState)>), Error> {
    use oteryn_protocol_oteryn::{
        MessageType, decode_command_result, decode_state_delta, decode_wire_envelope,
    };
    tokio::runtime::Builder::new_current_thread()
        .build()?
        .block_on(async {
            let total_bytes = inbound.len();
            if cast_send_offset >= total_bytes {
                return Err("C18a invalid raw cast send boundary".into());
            }
            let mut inbound = inbound;
            let mut saw_result = false;
            let mut first_stats_base = None;
            let mut saw_charge = false;
            let mut combat_after_exchange = None;
            while !inbound.is_empty() {
                let offset = total_bytes - inbound.len();
                let frame = read_frame(&mut inbound).await?;
                let wire = decode_wire_envelope(&frame)?;
                let before_send = offset < cast_send_offset;
                if before_send && total_bytes - inbound.len() > cast_send_offset {
                    return Err("C18a cast send boundary splits an inbound frame".into());
                }
                match wire.message_type() {
                    MessageType::CommandResult
                        if wire.server_sequence() == cast.server_sequence =>
                    {
                        let result = decode_command_result(wire.payload())?;
                        if before_send
                            || result.command_id != cast.command_id
                            || result.status as u32 != cast.status
                            || result.payload != cast.payload
                        {
                            return Err("C18a raw cast reply disagrees with capture".into());
                        }
                        saw_result = true;
                    }
                    MessageType::StateDelta => {
                        let delta = decode_state_delta(wire.payload())?;
                        if delta.domain_id == attack::STATE_DOMAIN_ACTOR_COMBAT_STATE {
                            let state = attack::decode_actor_combat_state(delta.payload)
                                .map_err(|error| format!("{error:?}"))?;
                            if before_send && state.in_fight {
                                return Err("C18a raw pre-send combat lock was already set".into());
                            }
                            let revision =
                                combat_after_exchange.map_or(delta.base_revision, |(base, _)| base);
                            combat_after_exchange = Some((revision, state));
                        }
                        if delta.domain_id != actor_spell::STATE_DOMAIN_ACTOR_VITALS || saw_charge {
                            continue;
                        }
                        let stats = actor_spell::decode_actor_vitals(delta.payload)
                            .map_err(|error| format!("{error:?}"))?;
                        let sequence = wire.server_sequence();
                        first_stats_base.get_or_insert(delta.base_revision);
                        if sequence > preceding_sequence && stats == before {
                            continue;
                        }
                        if before_send
                            || delta.delta_type != actor_spell::DELTA_TYPE_ACTOR_VITALS_V1
                            || sequence != vitals.server_sequence
                            || delta.payload != vitals.payload
                            || !crate::qualification_scorch_charge(
                                preceding_sequence,
                                sequence,
                                before,
                                stats,
                            )
                        {
                            return Err(
                                "C18a raw first stats change disagrees with the exact charge"
                                    .into(),
                            );
                        }
                        saw_charge = true;
                    }
                    _ => {}
                }
            }
            if let (true, true, Some(revision)) = (saw_result, saw_charge, first_stats_base) {
                Ok((revision, combat_after_exchange))
            } else {
                Err("C18a raw exchange lacks the cast or charged stats".into())
            }
        })
}

#[test]
fn successful_aimed_exchange_runs_the_admitted_client() -> Result<(), Error> {
    for with_clock in [false, true] {
        for charge_before_receipt in [false, true] {
            for with_probes in [false, true] {
                replay_successful_aimed_exchange(with_clock, charge_before_receipt, with_probes)?;
            }
        }
    }
    Ok(())
}

fn replay_successful_aimed_exchange(
    with_clock: bool,
    charge_before_receipt: bool,
    with_probes: bool,
) -> Result<(), Error> {
    use oteryn_protocol_oteryn::encode_state_delta;
    let mut fixture = fixture()?;
    fixture.expected = oteryn_session::SpellCastDisposition::Cast;
    fixture.feedback = "Spell cast";
    let command_id = 7;
    let preceding_sequence = 40;
    let before = actor_spell::ActorVitals {
        health: 150,
        max_health: 150,
        mana: 55,
        max_mana: 55,
        soul: 100,
        ..Default::default()
    };
    let charged = actor_spell::ActorVitals { mana: 47, ..before };
    let result = actor_spell::encode_spell_cast_result(fixture.expected);
    let clock = world_clock::WorldClock {
        tibian_minute: 1110,
        light_level: 117,
        light_color: 215,
    };
    let mut frames = Vec::new();
    if with_clock {
        frames.push(encode_state_delta(
            1,
            41,
            world_clock::STATE_DOMAIN_WORLD_CLOCK,
            8,
            9,
            world_clock::DELTA_TYPE_WORLD_CLOCK_V1,
            &world_clock::encode_world_clock(world_clock::WorldClock {
                tibian_minute: 1111,
                ..clock
            })
            .map_err(|error| format!("{error:?}"))?,
        )?);
    }
    let first_sequence = preceding_sequence + 1 + u64::from(with_clock);
    let reply_sequence = first_sequence + u64::from(charge_before_receipt);
    let charge_sequence = first_sequence + u64::from(!charge_before_receipt);
    let mut cast_frames = [
        (
            reply_sequence,
            encode_command_result(
                1,
                reply_sequence,
                command_id,
                CommandStatus::Accepted,
                &result,
            )?,
        ),
        (
            charge_sequence,
            encode_state_delta(
                1,
                charge_sequence,
                actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                3,
                4,
                actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
                &actor_spell::encode_actor_vitals(&charged).map_err(|e| format!("{e:?}"))?,
            )?,
        ),
    ];
    cast_frames.sort_by_key(|(sequence, _)| *sequence);
    frames.extend(cast_frames.into_iter().map(|(_, frame)| frame));
    // The production Session also claims the mandatory post-receipt stats publication.
    if charge_before_receipt {
        frames.push(encode_state_delta(
            1,
            reply_sequence + 1,
            actor_spell::STATE_DOMAIN_ACTOR_VITALS,
            4,
            5,
            actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
            &actor_spell::encode_actor_vitals(&charged).map_err(|e| format!("{e:?}"))?,
        )?);
    }
    assert!(crate::qualification_scorch_charge(
        preceding_sequence,
        charge_sequence,
        before,
        charged
    ));
    assert!(!crate::qualification_scorch_charge(
        preceding_sequence,
        preceding_sequence,
        before,
        charged
    ));
    assert!(!crate::qualification_scorch_charge(
        preceding_sequence,
        charge_sequence,
        before,
        actor_spell::ActorVitals { mana: 48, ..before }
    ));
    if with_probes {
        use oteryn_protocol_oteryn::encode_liveness_probe;
        frames.insert(0, encode_liveness_probe(1, 1)?);
        frames.insert(frames.len() - 1, encode_liveness_probe(1, 2)?);
    }
    let mut inbound = Vec::new();
    let mut charge_end = 0;
    let mut cast_send_offset = 0;
    for (index, frame) in frames.into_iter().enumerate() {
        inbound.extend(u32::try_from(frame.len())?.to_be_bytes());
        let sequence = oteryn_protocol_oteryn::decode_wire_envelope(&frame)?.server_sequence();
        inbound.extend(frame);
        if with_probes && index == 0 {
            cast_send_offset = inbound.len();
        }
        if sequence == charge_sequence {
            charge_end = inbound.len();
        }
    }
    let cast_envelope = Envelope {
        command_id,
        game_session_id: String::new(),
        payload: result,
        server_sequence: reply_sequence,
        status: CommandStatus::Accepted as u32,
    };
    let charge_envelope = Envelope {
        payload: actor_spell::encode_actor_vitals(&charged).map_err(|e| format!("{e:?}"))?,
        server_sequence: charge_sequence,
        game_session_id: String::new(),
        ..cast_envelope
    };
    assert_eq!(
        cast_exchange_revision(
            &inbound,
            &cast_envelope,
            &charge_envelope,
            preceding_sequence,
            cast_send_offset,
            before
        )?,
        (3, None),
    );
    assert!(
        cast_exchange_revision(
            &inbound,
            &cast_envelope,
            &charge_envelope,
            preceding_sequence,
            charge_end,
            before
        )
        .is_err()
    );
    assert!(
        cast_exchange_revision(
            &inbound,
            &cast_envelope,
            &charge_envelope,
            preceding_sequence,
            0,
            actor_spell::ActorVitals { mana: 54, ..before }
        )
        .is_err()
    );
    let clock_before = if with_clock {
        Some((
            8,
            world_clock::encode_world_clock(clock).map_err(|error| format!("{error:?}"))?,
        ))
    } else {
        None
    };
    fixture.live = Some(LiveExchange {
        inbound,
        cast_send_offset,
        intent: actor_spell::encode_spell_cast_intent(&SpellCastIntent {
            spell: fixture.spell,
            target: SpellTarget::AttackTarget,
            aim_at_target: true,
        }),
        session: oteryn_protocol_oteryn::GameSessionId::decode(&id(1))?,
        command_id,
        preceding_sequence,
        vitals_before: actor_spell::encode_actor_vitals(&before).map_err(|e| format!("{e:?}"))?,
        vitals_revision: 3,
        clock_before,
        charged,
        combat_after_exchange: None,
    });
    replay_through_session(fixture)
}
