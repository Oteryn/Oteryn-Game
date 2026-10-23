//! Qualification through serve_admitted and the composed native caster, sharing the
//! existing fresh-admission PostgreSQL fixture and the existing LOOK framing helper.

use super::death_ml_loss_composed::{current_fence, production_generation_with_source};
use super::fixture::{Harness, debug, id, join_two, run};
use super::item_look::{large_stack, read_frame, seed_main_backpack};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildCommitOutcome, BuildOccurrence, DurableBuildState,
    convert_vocation,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    ChannelContentPin, CharacterLease, MessageType, MovementLocalPosition, decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::{
    RUNE_USE_OFFERED, SelectedCapabilities, test_command,
};
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    serve_admitted,
};
use crate::gameplay_transport::rune_use_bridge::test_faults::{self, Point};
use crate::gameplay_transport::{ComposedFreshAdmission, SeamRooms, item_fence_of};
use crate::spell::cast::CharacterCastFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::actor_spell::SpellTargetPosition;
use oteryn_protocol_oteryn::item_view::{
    STATE_DOMAIN_CHARACTER_INVENTORY, decode_character_inventory,
};
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, UseDisposition, UseIntent, UseTarget, decode_use_result,
    encode_item_use_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use oteryn_protocol_oteryn::{CommandStatus, decode_command_result, decode_snapshot_chunk};
use tokio::io::{AsyncWriteExt, DuplexStream};

const CASTER: MovementLocalPosition = MovementLocalPosition {
    x: 2,
    y: 11,
    floor: -7,
};
const VICTIM: MovementLocalPosition = MovementLocalPosition {
    x: 1,
    y: 11,
    floor: -7,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    Replay,
    Protection,
    TooFar,
    NoCapability,
    UnqualifiedSource,
    StaleSession,
    LostContent,
    BeforeCommit,
    AfterCommit,
    StaleRevision,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Expected {
    Result(UseDisposition),
    Replay,
    Disconnect,
}

async fn send_use(
    client: &mut DuplexStream,
    id: u64,
    intent: &UseIntent,
    expected: Expected,
) -> TestResult {
    let payload = encode_item_use_intent(intent).map_err(debug)?;
    let frame = test_command(id, COMMAND_TYPE_USE_INTENT, &payload);
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    loop {
        let frame = match read_frame(client).await {
            Ok(frame) => frame,
            Err(error) if expected == Expected::Disconnect => {
                assert_eq!(
                    error
                        .downcast_ref::<std::io::Error>()
                        .map(std::io::Error::kind),
                    Some(std::io::ErrorKind::UnexpectedEof)
                );
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        assert_ne!(envelope.message_type(), MessageType::ProtocolError);
        if envelope.message_type() != MessageType::CommandResult {
            continue;
        }
        let result = decode_command_result(envelope.payload()).map_err(debug)?;
        assert_eq!(result.command_id, id);
        match expected {
            Expected::Result(expected) => {
                let status = if expected == UseDisposition::Rejected {
                    CommandStatus::Rejected
                } else {
                    CommandStatus::Accepted
                };
                assert_eq!(
                    result.status, status,
                    "command {id}: expected {expected:?}, payload {:?}",
                    result.payload
                );
                assert_eq!(decode_use_result(result.payload).map_err(debug)?, expected);
            }
            Expected::Replay => {
                assert_eq!(result.status, CommandStatus::DuplicateReplay);
                assert_eq!(
                    decode_use_result(result.payload).map_err(debug)?,
                    UseDisposition::Committed
                );
            }
            Expected::Disconnect => return Err("an unresolved cast sent a terminal result".into()),
        }
        return Ok(());
    }
}

async fn client_cast(
    client: &mut DuplexStream,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    victim: EntityRef,
    case: Case,
    original: Option<UseIntent>,
    first_id: u64,
) -> TestResult<UseIntent> {
    let mut source = None;
    loop {
        let frame = read_frame(client).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == MessageType::SnapshotChunk {
            let (_, domains) = decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
            for domain in domains {
                if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                    source = decode_character_inventory(domain.payload)
                        .map_err(debug)?
                        .entries
                        .first()
                        .map(|e| e.handle);
                }
            }
        }
        if envelope.message_type() == MessageType::SnapshotCommit {
            break;
        }
    }
    let retry = original.is_some();
    let intent = match original {
        Some(original) => original,
        None => UseIntent {
            target: UseTarget::Item(source.ok_or("snapshot exposes the direct entry handle")?),
            use_with: Some(victim),
            use_with_map_item: None,
            use_with_position: None,
        },
    };
    if retry {
        // FND-02 §13.3: unknown pending work cannot reopen the logical session. The
        // duplicate is already queued by serve_cast; it must never reach the family.
        let error = read_frame(client)
            .await
            .err()
            .ok_or("an unresolved session reopened instead of disconnecting")?;
        assert_eq!(
            error
                .downcast_ref::<std::io::Error>()
                .map(std::io::Error::kind),
            Some(std::io::ErrorKind::UnexpectedEof)
        );
        return Ok(intent);
    }
    let actor = owner.runtime.lock().await.visible_entities().players[0].actor;
    let session = crate::foundation::GameSessionId::decode(&id(50)).map_err(debug)?;
    let victim_actor = owner
        .runtime
        .lock()
        .await
        .positioned_actor_census()
        .map_err(debug)?
        .into_iter()
        .find(|(actor, _, _)| actor.placement_identity() == victim.identity)
        .map(|(actor, _, _)| actor);
    let before = match victim_actor {
        Some(actor) => owner
            .runtime
            .lock()
            .await
            .live_creature_health(actor)
            .unwrap_or(0),
        None => 0,
    };
    match case {
        Case::Replay => {
            let bad = UseIntent {
                use_with: Some(EntityRef {
                    generation: victim.generation + 1,
                    ..victim
                }),
                ..intent.clone()
            };
            send_use(
                client,
                first_id,
                &bad,
                Expected::Result(UseDisposition::TargetIllegal),
            )
            .await?;
            let invalid_handle = UseIntent {
                target: UseTarget::Item(
                    std::num::NonZeroU64::new(u64::MAX).ok_or("nonzero handle")?,
                ),
                ..intent.clone()
            };
            send_use(
                client,
                first_id + 1,
                &invalid_handle,
                Expected::Result(UseDisposition::StaleState),
            )
            .await?;
            let wrong_floor = UseIntent {
                use_with: None,
                use_with_position: Some(SpellTargetPosition {
                    x: VICTIM.x,
                    y: VICTIM.y,
                    floor: VICTIM.floor - 1,
                }),
                ..intent.clone()
            };
            send_use(
                client,
                first_id + 2,
                &wrong_floor,
                Expected::Result(UseDisposition::TargetIllegal),
            )
            .await?;
            let position = UseIntent {
                use_with: None,
                use_with_position: Some(SpellTargetPosition {
                    x: VICTIM.x,
                    y: VICTIM.y,
                    floor: VICTIM.floor,
                }),
                ..intent.clone()
            };
            send_use(
                client,
                first_id + 3,
                &position,
                Expected::Result(UseDisposition::Committed),
            )
            .await?;
            let victim_actor = victim_actor.ok_or("the original target exists")?;
            let after = owner
                .runtime
                .lock()
                .await
                .live_creature_health(victim_actor)
                .unwrap_or(0);
            assert!(after < before, "the real native cast damages its target");
            send_use(client, first_id + 3, &position, Expected::Replay).await?;
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .live_creature_health(victim_actor)
                    .unwrap_or(0),
                after
            );
        }
        Case::Protection => {
            // Reuse the existing condition owner fixture; both gates precede an invalid target.
            for (offset, gate) in ["pacified", "powerless"].into_iter().enumerate() {
                crate::gameplay_transport::actor_spell::tests::gate_player(
                    &mut *owner.states.spell_states.lock().await,
                    actor,
                    session,
                    gate,
                );
                let bad = UseIntent {
                    use_with: Some(EntityRef {
                        generation: victim.generation + 1,
                        ..victim
                    }),
                    ..intent.clone()
                };
                send_use(
                    client,
                    first_id + u64::try_from(offset)?,
                    &bad,
                    Expected::Result(UseDisposition::RequirementNotMet),
                )
                .await?;
                let runtime = owner.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                let state = states
                    .get_mut(&runtime, actor, session)
                    .ok_or("current player")?;
                let before = state.owned_conditions().clone();
                state
                    .apply_batch_conditions(&before, &Default::default())
                    .map_err(debug)?;
            }
            send_use(
                client,
                first_id + 2,
                &intent,
                Expected::Result(UseDisposition::ProtectionZone),
            )
            .await?;
        }
        Case::TooFar => {
            send_use(
                client,
                first_id,
                &intent,
                Expected::Result(UseDisposition::TooFar),
            )
            .await?;
        }
        Case::NoCapability => {
            send_use(
                client,
                first_id,
                &intent,
                Expected::Result(UseDisposition::Rejected),
            )
            .await?
        }
        Case::UnqualifiedSource => {
            send_use(
                client,
                first_id,
                &intent,
                Expected::Result(UseDisposition::NothingToUse),
            )
            .await?
        }
        Case::StaleSession => {
            sqlx::query("UPDATE game_durability_reconnect_sessions SET current_transport_ref=$1,current_generation=current_generation+1 WHERE game_session_id=encode($2,'hex')::uuid")
                .bind([11_u8;16].as_slice()).bind(id(50).as_slice()).execute(&harness.pool).await?;
            send_use(
                client,
                first_id,
                &intent,
                Expected::Result(UseDisposition::Rejected),
            )
            .await?;
        }
        Case::LostContent => {
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,2,1,$3,$4,$5)")
                .bind(id(42).as_slice()).bind(id(43).as_slice()).bind([9_u8;32].as_slice()).bind([8_u8;32].as_slice()).bind([7_u8;32].as_slice()).execute(&harness.pool).await?;
            send_use(client, first_id, &intent, Expected::Disconnect).await?;
        }
        Case::BeforeCommit | Case::AfterCommit | Case::StaleRevision => {
            test_faults::arm(if case == Case::AfterCommit {
                Point::AfterCommit
            } else {
                Point::BeforeCommit
            });
            send_use(client, first_id, &intent, Expected::Disconnect).await?;
        }
    }
    client.shutdown().await?;
    Ok(intent)
}

async fn serve_cast(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: AdmittedSession,
    victim: EntityRef,
    case: Case,
    original: Option<UseIntent>,
) -> TestResult<(AdmittedSession, UseIntent)> {
    let first_id = admitted.continuity.next_command_id;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    if let Some(intent) = &original {
        let payload = encode_item_use_intent(intent).map_err(debug)?;
        let frame = test_command(first_id - 1, COMMAND_TYPE_USE_INTENT, &payload);
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
    }
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        match end {
            ConnectionEnd::AdmittedThenDisconnected(session) => {
                Ok::<_, Box<dyn std::error::Error>>(session)
            }
            other => Err(format!("unexpected connection end: {other:?}").into()),
        }
    };
    let (admitted, cast) = join_two(
        serving,
        client_cast(
            &mut client,
            owner,
            harness,
            victim,
            case,
            original,
            first_id,
        ),
    )
    .await;
    Ok((admitted?, cast?))
}

#[test]
fn rune_use_bridge_postgres_real_dispatch_casts_once_and_reloads_one_debit() -> TestResult {
    large_stack(scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        for case in [
            Case::Replay,
            Case::Protection,
            Case::TooFar,
            Case::NoCapability,
            Case::UnqualifiedSource,
            Case::StaleSession,
            Case::LostContent,
            Case::BeforeCommit,
            Case::AfterCommit,
            Case::StaleRevision,
        ] {
            run_case(admin.clone(), case)
                .await
                .map_err(|e| format!("{case:?}: {e}"))?;
        }
        Ok(())
    })
}

async fn run_case(admin: String, case: Case) -> TestResult {
    let harness = Harness::create_with_admission_seed(admin, "rune_bridge", true, false).await?;
    prepare_current_session(&harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = production_generation_with_source(
        world,
        "content-1",
        Some(include_bytes!(
            "../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
        )),
    )?;
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let room = pin.qualified_room().clone();
    let start = if case == Case::Protection {
        MovementLocalPosition {
            x: 2,
            y: 0,
            floor: -7,
        }
    } else {
        CASTER
    };
    let target_position = if case == Case::Protection {
        MovementLocalPosition {
            x: 1,
            y: 0,
            floor: -7,
        }
    } else if case == Case::TooFar {
        MovementLocalPosition {
            x: 10,
            y: 10,
            floor: CASTER.floor,
        }
    } else {
        VICTIM
    };
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let dependencies = Dependencies::with_content(
        channel_pin.clone(),
        room.clone(),
        native.spell_book().clone(),
    )?;
    let (actor, session) = (dependencies.actor, dependencies.admitted.game_session_id);
    let victim;
    {
        let mut runtime = dependencies.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        runtime
            .realize_native_qualification_spawn(channel_pin, "oteryn:creature.rat", target_position)
            .map_err(debug)?;
        let creature = runtime.visible_entities().creatures[0];
        victim = EntityRef {
            identity: creature.actor.placement_identity(),
            generation: creature.generation,
        };
    }
    let mut owner = dependencies.owner(&harness, &authority);
    owner.active_generation = Some(active);
    owner.rooms = SeamRooms::new(Some(&room), Some(&room));
    let mut admitted = dependencies.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let (current, _) = FreshAdmissionStore::from_root(harness.root.clone())
        .current_session_at(session)
        .await?;
    admitted.item_fence = item_fence_of(current);
    let formula = native.training_formula().ok_or("training formula")?;
    let druid = convert_vocation(formula, &DurableBuildState::default(), "druid").map_err(debug)?;
    assert!(matches!(
        harness
            .root
            .commit_character_build(
                &authority,
                &harness.node,
                current_fence(&owner, session).await?,
                BuildChangeRequest {
                    occurrence: BuildOccurrence::from_bytes(id(90)).map_err(debug)?,
                    cause: BuildCause::VocationChoice,
                    before: DurableBuildState::default(),
                    after: druid.clone(),
                    pruned_stance: None,
                },
                formula
            )
            .await
            .map_err(debug)?,
        BuildCommitOutcome::Committed(_)
    ));
    {
        let runtime = dependencies.runtime.lock().await;
        let mut states = owner.states.spell_states.lock().await;
        assert!(
            states
                .initialize(
                    &runtime,
                    actor,
                    session,
                    CharacterCastFacts {
                        level: 50,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now()
                )
                .is_some()
        );
        assert!(states.load_owned_build(&runtime, actor, session, &druid));
        states
            .get_mut(&runtime, actor, session)
            .ok_or("present caster")?
            .enable_owned_training(&druid, formula)
            .map_err(debug)?;
    }
    assert!(owner.offered_capabilities().iter().all(|c| c.id != 22));
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        RUNE_USE_OFFERED,
        if case == Case::NoCapability {
            &[4, 6, 15]
        } else {
            &[4, 6, 15, 22]
        },
    )
    .ok_or("injected selection")?;
    // Fixture data only. Admission, source policy, caster facts and payment below
    // are read by the real current owners, with all normal runtime fences.
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&harness.pool).await?;
    let source_key = if case == Case::UnqualifiedSource {
        "oteryn:item.tibia.i3607"
    } else {
        "oteryn:item.tibia.i3174"
    };
    seed_main_backpack(&harness, &[(source_key, 3)]).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
    let victim_actor = owner.runtime.lock().await.visible_entities().creatures[0].actor;
    let initial_health = owner
        .runtime
        .lock()
        .await
        .live_creature_health(victim_actor)
        .ok_or("live target")?;
    let (resumed, original) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        serve_cast(&owner, &harness, admitted, victim, case, None),
    )
    .await??;
    if case == Case::LostContent {
        assert_eq!(
            resumed.continuity.next_command_id,
            admitted.continuity.next_command_id + 1
        );
    }
    if matches!(
        case,
        Case::BeforeCommit | Case::AfterCommit | Case::StaleRevision
    ) {
        assert_eq!(
            resumed.continuity.next_command_id,
            admitted.continuity.next_command_id + 1
        );
        assert!(!owner.states.command_ingress.resumable(&resumed));
        let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(quantity, if case == Case::AfterCommit { 2 } else { 3 });
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .live_creature_health(victim_actor),
            Some(initial_health),
            "no owner effect is installed before reconciliation"
        );
        if case == Case::StaleRevision {
            // Adversarial persisted-state substitution, using the fixture's seed role.
            // The normal Item guard deliberately rejects this direct write.
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE game_item_instances SET state_revision=state_revision+1 WHERE item_instance_id=encode($1,'hex')::uuid")
                .bind(id(71).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
        }
        let (refused, _) = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            serve_cast(
                &owner,
                &harness,
                resumed,
                victim,
                case,
                Some(original.clone()),
            ),
        )
        .await??;
        assert_eq!(
            refused.continuity.next_command_id,
            resumed.continuity.next_command_id
        );
        let after_refusal: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(after_refusal, quantity);
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .live_creature_health(victim_actor),
            Some(initial_health)
        );
        // Retain the family's original cancellation/reconciliation qualification. This
        // is owner reconciliation of its retained batch, not another client execution
        // or permission to resume the unresolved common ingress.
        let command = crate::gameplay_transport::connection::UseCommand {
            game_session_id: session,
            command_id: admitted.continuity.next_command_id,
            item_fence: admitted.item_fence,
        };
        let expected = if case == Case::StaleRevision {
            UseDisposition::Rejected
        } else {
            UseDisposition::Committed
        };
        let result = owner
            .use_rune(actor, command, original.clone(), None, true)
            .await
            .ok_or("owner reconciles its retained original")?;
        assert_eq!(result.disposition, expected);
        if case != Case::StaleRevision {
            let health = owner
                .runtime
                .lock()
                .await
                .live_creature_health(victim_actor);
            let replay = owner
                .use_rune(actor, command, original, None, true)
                .await
                .ok_or("owner retains its reconciled original")?;
            assert_eq!(replay.disposition, UseDisposition::Committed);
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .live_creature_health(victim_actor),
                health
            );
        }
        assert!(!owner.states.command_ingress.resumable(&resumed));
    }
    let (quantity, revision): (i64, String) = sqlx::query_as("SELECT quantity,state_revision::text FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
    let committed = matches!(case, Case::Replay | Case::BeforeCommit | Case::AfterCommit);
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(id(50).as_slice()).fetch_one(&harness.pool).await?;
    assert_eq!(receipts, i64::from(committed));
    assert_eq!(quantity, if committed { 2 } else { 3 });
    assert_eq!(
        revision,
        if committed || case == Case::StaleRevision {
            "2"
        } else {
            "1"
        }
    );
    let health = owner
        .runtime
        .lock()
        .await
        .live_creature_health(victim_actor)
        .unwrap_or(0);
    if committed {
        assert!(health < initial_health);
    } else {
        assert_eq!(health, initial_health);
    }
    drop(owner);
    drop(authority);
    drop(seal);
    harness.cleanup().await
}
