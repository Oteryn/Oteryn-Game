//! C18a replay owner: `oteryn_session::Session::apply_domain_delta` applies the
//! ACTOR_COMBAT_STATE payload, then `protocol::run_cast` reads that state and
//! `protocol::cast_selected` applies the spell result to `SpellFeedback`.

use oteryn_client_runtime::combat::{CombatCommand, CombatReport, run_combat};
use oteryn_client_runtime::spell::SpellFeedback;
use oteryn_protocol_oteryn::actor_spell::{self, SpellCastIntent, SpellTarget};
use oteryn_protocol_oteryn::attack::{self, ActorCombatState, FightModes};
use oteryn_protocol_oteryn::{
    DomainSnapshot, ServerAcceptedValue, encode_command_result, encode_server_accepted,
    encode_single_chunk_snapshot,
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
    intent: Vec<u8>,
    session: oteryn_protocol_oteryn::GameSessionId,
    command_id: u64,
    preceding_sequence: u64,
    vitals_before: Vec<u8>,
    vitals_revision: u64,
    charged: actor_spell::ActorVitals,
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
                                selected_capabilities: &[6, 17],
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
                                DomainSnapshot { domain_id: attack::STATE_DOMAIN_ACTOR_COMBAT_STATE, revision: 1, snapshot_type: attack::SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1, payload: &combat },
                        ];
                        if let Some(live) = &live {
                            domains.push(DomainSnapshot {
                                domain_id: actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                                revision: live.vitals_revision,
                                snapshot_type: actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                                payload: &live.vitals_before,
                            });
                        }
                        for frame in encode_single_chunk_snapshot(generation, 1, sequence, &domains)? {
                            write_frame(&mut peer, &frame).await?;
                        }
                        let frame = read_frame(&mut peer).await?;
                        let command = oteryn_protocol_oteryn::decode_wire_envelope(&frame)?
                            .client_command(generation)?;
                        if let Some(live) = &live {
                            use tokio::io::AsyncWriteExt;
                            assert_eq!(command.command_id, live.command_id);
                            assert_eq!(command.payload, live.intent);
                            peer.write_all(&live.inbound).await?;
                            return Ok::<_, Error>(());
                        }
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
                            supported_capabilities: &[6, 17],
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
                    assert_eq!(session.combat_state(), before.as_ref());
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
    wrapper: String,
    regeneration: Vec<(u64, u32, u32)>,
    capture_device: u64,
    capture_inode: u64,
    spell: u32,
    damage: u64,
    damage_sequence: u64,
    caster_identity: [u8; 16],
    caster_generation: u64,
    mana_before: u32,
    vitals_before: Vec<u8>,
    mana_after: u32,
    mana_relog: u32,
    mana_logout: u32,
    soul_logout: u32,
    pre_cast_sequence: u64,
    vitals_sequence: u64,
    logout_sequence: u64,
    soul_before: u32,
    soul_after: u32,
    soul_relog: u32,
    combat_before: Vec<u8>,
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
    let damage: Envelope = serde_json::from_slice(&std::fs::read(root.join("c18a-damage.json"))?)?;
    let proof: Proof = serde_json::from_slice(&std::fs::read(root.join("c18a-proof.json"))?)?;
    verify_capture_directory(root, &proof)?;
    let session = session_id(&proof.game_session_id)?;
    if session_id(&cast.game_session_id)? != session
        || session_id(&health.game_session_id)? != session
        || session_id(&damage.game_session_id)? != session
        || session_id(&vitals.game_session_id)? != session
        || vitals.command_id != proof.command_id
        || vitals.status != cast.status
        || vitals.server_sequence != proof.vitals_sequence
        || !(proof.pre_cast_sequence < cast.server_sequence
            && cast.server_sequence < vitals.server_sequence
            && vitals.server_sequence < proof.logout_sequence)
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
        (proof.logout_sequence, proof.mana_logout, proof.soul_logout),
    ) {
        return Err("C18a replay found unexplained mana or soul before logout".into());
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
    let inbound = std::fs::read(root.join("c18a-exchange.bin"))?;
    let vitals_revision = cast_exchange_revision(&inbound, &cast, &vitals)?;
    replay_through_session(Fixture {
        combat: proof.combat_before,
        result: cast.payload,
        spell: NonZeroU32::new(proof.spell).ok_or("C18a invalid spell index")?,
        expected: oteryn_session::SpellCastDisposition::Cast,
        feedback: "Spell cast",
        live: Some(LiveExchange {
            inbound,
            intent,
            session,
            command_id: cast.command_id,
            preceding_sequence: proof.pre_cast_sequence,
            vitals_before: proof.vitals_before,
            vitals_revision,
            charged,
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
    verify_damage(&damage.payload, caster, target, proof.damage)?;
    let entry = delta
        .update
        .iter()
        .find(|entry| entry.entity == target)
        .ok_or("C18a health capture lacks the exact Rat generation")?;
    assert_eq!(entry.kind, wire::EntityKind::Creature);
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
fn strict_envelope_rejects_unknown_duplicate_and_missing_keys() {
    let base =
        r#"{"command_id":1,"game_session_id":"x","payload":[1],"server_sequence":2,"status":1}"#;
    assert!(serde_json::from_str::<Envelope>(base).is_ok());
    for extra in [r#","status":1}"#, r#","unexpected":0}"#] {
        let invalid = format!("{}{extra}", base.strip_suffix('}').unwrap());
        assert!(serde_json::from_str::<Envelope>(&invalid).is_err());
    }
    assert!(serde_json::from_str::<Envelope>(r#"{"payload":[1]}"#).is_err());
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
    let log = std::fs::read_to_string(parent.join("stage-seam_stages.log"))?;
    if !valid_run_log(&log, &binding) {
        return Err("C18a node-boot seam log lacks one eligible current capture".into());
    }
    Ok(())
}

fn valid_run_log(log: &str, binding: &str) -> bool {
    log.lines().filter(|line| *line == binding).count() == 1
        && log
            .lines()
            .filter(|line| {
                line.starts_with("SEAM_EVIDENCE e2e_capture scenario=C18a.client status=CAPTURED ")
            })
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
    damage > 0
        && after > 0
        && u64::from(before).checked_sub(u64::from(after)) == damage.checked_mul(5)
}

fn exact_vitals(mana: [u32; 4], soul: [u32; 4]) -> bool {
    mana[0].checked_sub(8) == Some(mana[1])
        && mana[2] == mana[3]
        && soul[0] == soul[1]
        && soul[2] == soul[3]
}

fn verify_damage(
    payload: &[u8],
    caster: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    target: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    expected: u64,
) -> Result<(), Error> {
    use oteryn_protocol_oteryn::presentation;
    let presentation =
        presentation::decode_presentation_batch(payload).map_err(|error| format!("{error:?}"))?;
    use oteryn_protocol_oteryn::world_spatial::ActorPosition;
    let cast_tile = ActorPosition {
        x: 32490,
        y: 32169,
        floor: 7,
    };
    let rat_tile = ActorPosition {
        x: 32491,
        y: 32169,
        floor: 7,
    };
    if !crate::qualification_scorch_cue(&presentation, cast_tile) {
        return Err("C18a damage batch lacks the caster's Scorch cue".into());
    }
    let attributed: Vec<_> = presentation
        .iter()
        .filter_map(|event| match event.kind {
            presentation::PresentationEventKind::ValueText {
                kind: presentation::PresentationValueKind::DamageDealt,
                position,
                target: victim,
                attacker: Some(source),
                primary,
                secondary: None,
                ..
            } if position == rat_tile
                && victim == target
                && source == caster
                && primary.color == presentation::PresentationColor::Red =>
            {
                Some(primary.value)
            }
            _ => None,
        })
        .collect();
    if attributed != [expected] {
        return Err("C18a damage presentation lacks one exact caster-to-Rat hit".into());
    }
    Ok(())
}

#[test]
fn rejects_false_c18a_damage_cost_and_aim() {
    let spell = std::num::NonZeroU32::new(1).unwrap();
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
    assert!(!exact_rat_loss(100, 60, 7));
    assert!(!exact_rat_loss(100, 100, 0));
    assert!(exact_vitals([30, 22, 29, 29], [0, 0, 0, 0]));
    assert!(!exact_vitals([30, 23, 29, 29], [0, 0, 0, 0]));
    assert!(!exact_vitals([30, 22, 29, 30], [0, 0, 0, 0]));
}

#[test]
fn rejects_health_loss_without_the_exact_damage_source() -> Result<(), Error> {
    use oteryn_protocol_oteryn::presentation::{
        PresentationColor, PresentationEvent, PresentationEventKind, PresentationValue,
        PresentationValueKind, encode_presentation_batch,
    };
    use oteryn_protocol_oteryn::world_spatial::ActorPosition;
    use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
    let caster = EntityRef {
        identity: [1; 16],
        generation: 1,
    };
    let target = EntityRef {
        identity: [2; 16],
        generation: 1,
    };
    let event = |attacker, victim| PresentationEvent {
        decision_ordinal: 1,
        emission_ordinal: 1,
        kind: PresentationEventKind::ValueText {
            kind: PresentationValueKind::DamageDealt,
            position: ActorPosition {
                x: 32491,
                y: 32169,
                floor: 7,
            },
            target: victim,
            attacker: Some(attacker),
            primary: PresentationValue {
                value: 8,
                color: PresentationColor::Red,
            },
            secondary: None,
        },
    };
    let cue = PresentationEvent {
        decision_ordinal: 2,
        emission_ordinal: 1,
        kind: PresentationEventKind::Sound {
            position: ActorPosition {
                x: 32490,
                y: 32169,
                floor: 7,
            },
            cue_id: 1178,
            source: oteryn_protocol_oteryn::presentation::PresentationSource::Own,
            secondary_cue_id: 0,
        },
    };
    let unrelated = encode_presentation_batch(&[event(caster, target)])
        .map_err(|error| format!("{error:?}"))?;
    assert!(verify_damage(&unrelated, caster, target, 8).is_err());
    let payload = encode_presentation_batch(&[event(caster, target), cue])
        .map_err(|error| format!("{error:?}"))?;
    verify_damage(&payload, caster, target, 8)?;
    assert!(verify_damage(&payload, caster, target, 7).is_err());
    assert!(
        verify_damage(
            &payload,
            EntityRef {
                generation: 2,
                ..caster
            },
            target,
            8
        )
        .is_err()
    );
    assert!(
        verify_damage(
            &payload,
            caster,
            EntityRef {
                generation: 2,
                ..target
            },
            8
        )
        .is_err()
    );
    assert!(verify_damage(&[], caster, target, 8).is_err());
    Ok(())
}

#[test]
fn rejects_unbound_or_retried_wrapper_log() {
    let binding = "SEAM_EVIDENCE c18a_capture_binding directory=captures.test session=test";
    let pass = "SEAM_EVIDENCE e2e_capture scenario=C18a.client status=CAPTURED evidence=live";
    assert!(valid_run_log(&format!("{binding}\n{pass}"), binding));
    assert!(!valid_run_log(pass, binding));
    assert!(!valid_run_log(
        &format!("{binding}\n{binding}\n{pass}"),
        binding
    ));
    assert!(!valid_run_log(
        &format!("{binding}\nFIXTURE_RETRY step=logout reason=busy attempt=1\n{pass}"),
        binding
    ));
}

#[test]
fn rejects_an_unexplained_cast_refund() {
    let charged = (10, 22, 100);
    let logout = (20, 30, 100);
    let verified = [(12, 24, 100), (14, 26, 100), (16, 28, 100), (18, 30, 100)];
    assert!(crate::qualification_mana_history(
        charged, &verified, 30, logout
    ));
    assert!(!crate::qualification_mana_history(charged, &[], 30, logout));
    assert!(!crate::qualification_mana_history(
        charged,
        &[(12, 30, 100)],
        30,
        logout
    ));
}

fn cast_exchange_revision(
    inbound: &[u8],
    cast: &Envelope,
    vitals: &Envelope,
) -> Result<u64, Error> {
    use oteryn_protocol_oteryn::{
        MessageType, decode_command_result, decode_state_delta, decode_wire_envelope,
    };
    tokio::runtime::Builder::new_current_thread()
        .build()?
        .block_on(async {
            let mut inbound = inbound;
            let mut saw_result = false;
            while !inbound.is_empty() {
                let frame = read_frame(&mut inbound).await?;
                let wire = decode_wire_envelope(&frame)?;
                match wire.message_type() {
                    MessageType::CommandResult
                        if wire.server_sequence() == cast.server_sequence =>
                    {
                        let result = decode_command_result(wire.payload())?;
                        if result.command_id != cast.command_id
                            || result.status as u32 != cast.status
                            || result.payload != cast.payload
                        {
                            return Err("C18a raw cast reply disagrees with capture".into());
                        }
                        saw_result = true;
                    }
                    MessageType::StateDelta
                        if wire.server_sequence() == vitals.server_sequence && saw_result =>
                    {
                        let result = decode_state_delta(wire.payload())?;
                        if result.domain_id != actor_spell::STATE_DOMAIN_ACTOR_VITALS
                            || result.payload != vitals.payload
                        {
                            return Err("C18a raw charged stats disagree with capture".into());
                        }
                        return Ok(result.base_revision);
                    }
                    _ => {}
                }
            }
            Err("C18a raw exchange lacks the cast or charged stats".into())
        })
}

#[test]
fn successful_aimed_exchange_runs_the_admitted_client() -> Result<(), Error> {
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
    let mut inbound = Vec::new();
    for frame in [
        encode_command_result(1, 41, command_id, CommandStatus::Accepted, &result)?,
        encode_state_delta(
            1,
            42,
            actor_spell::STATE_DOMAIN_ACTOR_VITALS,
            3,
            4,
            actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
            &actor_spell::encode_actor_vitals(&charged).map_err(|e| format!("{e:?}"))?,
        )?,
    ] {
        inbound.extend(u32::try_from(frame.len())?.to_be_bytes());
        inbound.extend(frame);
    }
    fixture.live = Some(LiveExchange {
        inbound,
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
        charged,
    });
    replay_through_session(fixture)
}
