//! Real USE rune wire -> native field cast -> guarded writer -> durable row/result.
use super::death_ml_loss_composed::{current_fence, production_generation_with_pin};
use super::fixture::{Harness, debug, id, join_two, run};
use super::item_look::{large_stack, read_frame, seed_main_backpack};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    ChannelContentPin, CharacterLease, MessageType, MovementLocalPosition, decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::{OfferedCapability, test_command};
use crate::gameplay_transport::connection::{
    ConnectionEnd, FirstEntryOutcome, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities;
use crate::gameplay_transport::{SeamRooms, item_fence_of};
use crate::spell::cast::CharacterCastFacts;
use oteryn_protocol_oteryn::actor_spell::SpellTargetPosition;
use oteryn_protocol_oteryn::item_view::{
    STATE_DOMAIN_CHARACTER_INVENTORY, decode_character_inventory,
};
use oteryn_protocol_oteryn::world_object::{
    COMMAND_TYPE_USE_INTENT, UseDisposition, UseIntent, UseTarget, decode_use_result,
    encode_item_use_intent,
};
use oteryn_protocol_oteryn::{CommandStatus, decode_command_result, decode_snapshot_chunk};
use tokio::io::AsyncWriteExt;

// Future activation graph, only in this test. The production offer remains closed.
const RUNE_MAP_OFFER: &[OfferedCapability] = &[
    OfferedCapability {
        id: 4,
        requires: &[6],
    },
    OfferedCapability {
        id: 6,
        requires: &[],
    },
    OfferedCapability {
        id: 15,
        requires: &[4],
    },
    OfferedCapability {
        id: 18,
        requires: &[4, 6],
    },
    OfferedCapability {
        id: 22,
        requires: &[15],
    },
];
const FIELD_OFFER: &[OfferedCapability] = &[
    OfferedCapability {
        id: 4,
        requires: &[6],
    },
    OfferedCapability {
        id: 6,
        requires: &[],
    },
    OfferedCapability {
        id: 11,
        requires: &[15, 18],
    },
    OfferedCapability {
        id: 15,
        requires: &[4],
    },
    OfferedCapability {
        id: 18,
        requires: &[4, 6],
    },
    OfferedCapability {
        id: 22,
        requires: &[15],
    },
    OfferedCapability {
        id: 30,
        requires: &[6],
    },
];

#[test]
fn admit_cap_2_postgres_real_wire_refuses_missing_requirements_and_commits_field_once() -> TestResult
{
    large_stack(scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        for enabled in [false, true] {
            field_case(admin.clone(), enabled).await?;
        }
        Ok(())
    })
}

async fn field_case(admin: String, enabled: bool) -> TestResult {
    let harness =
        Harness::create_with_admission_seed(admin, "admit_cap_2_wire", true, false).await?;
    prepare_current_session(&harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) = production_generation_with_pin(world, "content-1")?;
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let entry_room = pin.qualified_room();
    let map = crate::map::spell_tiles::tests::movement_arena::world();
    let cells = map
        .movement_cells(entry_room.movement_cells())
        .map_err(debug)?;
    let start = MovementLocalPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    let room = entry_room
        .bundle_view(
            cells.clone(),
            map.frame_binding_digest(world),
            map.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: start.x,
                y: start.y,
                floor: start.floor,
            },
        )
        .ok_or("bundle field room")?;
    let channel_pin = ChannelContentPin::from_activation(
        world,
        pin.activation_sequence(),
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let dependencies =
        Dependencies::with_content(channel_pin, entry_room.clone(), native.spell_book().clone())?;
    let (actor, session) = (dependencies.actor, dependencies.admitted.game_session_id);
    {
        let mut runtime = dependencies.runtime.lock().await;
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
    }
    let mut owner = dependencies.owner(&harness, &authority);
    owner.active_generation = Some(active);
    owner.rooms = SeamRooms::new(None, Some(&room));
    owner.movement_cells = &cells;
    owner.required_capabilities = BoundRequiredCapabilities::for_channel_boot(
        if enabled { &[11, 30] } else { &[18] },
        crate::foundation::RuntimeScopeRefV1::channel(world, dependencies.admitted.channel_id),
        1,
        "policy-1",
        if enabled { FIELD_OFFER } else { RUNE_MAP_OFFER },
    )
    .map_err(debug)?;
    let formula = native.training_formula().ok_or("training formula")?;
    let druid = convert_vocation(formula, &DurableBuildState::default(), "druid").map_err(debug)?;
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
            formula,
        )
        .await
        .map_err(debug)?;
    // Fire Field requires magic level 1. Use the real build writer, and load
    // that same durable build into the caster before sending the rune command.
    let trained =
        DurableBuildState::new(druid.vocation(), (1, 0), druid.skills()).map_err(debug)?;
    harness
        .root
        .commit_character_build(
            &authority,
            &harness.node,
            current_fence(&owner, session).await?,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(91)).map_err(debug)?,
                cause: BuildCause::Training,
                before: druid,
                after: trained.clone(),
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
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
                        magic_level: 1,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS
                    },
                    (0, 0),
                    owner.owner_now()
                )
                .is_some()
        );
        assert!(states.load_owned_build(&runtime, actor, session, &trained));
        states
            .get_mut(&runtime, actor, session)
            .ok_or("caster")?
            .enable_owned_training(&trained, formula)
            .map_err(debug)?;
    }
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&harness.pool).await?;
    seed_main_backpack(&harness, &[("oteryn:item.tibia.i3188", 3)]).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice())
        .bind(pin.identity().client_artifact_digest().as_slice()).bind(room.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_control_field_policy_grants VALUES(session_user,encode($1,'hex')::uuid,'policy-1')")
        .bind(id(42).as_slice()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_spell_field_world_policies(world_id,world_policy_revision,world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity) VALUES(encode($1,'hex')::uuid,'policy-1',1,20,60000,'99902524e052f37574194466c2949c576e4ab269',session_user,'admit-cap-2-fixture')")
        .bind(id(42).as_slice()).execute(&harness.pool).await?;
    let mut admitted = dependencies.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let (current, _) = FreshAdmissionStore::from_root(harness.root.clone())
        .current_session_at(session)
        .await?;
    admitted.item_fence = item_fence_of(current);
    admitted.continuity.selected_capabilities = owner
        .required_capabilities
        .authenticate_support(if enabled {
            &[4, 6, 11, 15, 18, 22, 30]
        } else {
            &[4, 6, 15, 18, 22]
        })
        .map_err(debug)?
        .selected();
    let before = {
        let runtime = dependencies.runtime.lock().await;
        owner
            .states
            .spell_states
            .lock()
            .await
            .get(&runtime, actor, session)
            .ok_or("caster")?
            .vitals()
    };
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS).await;
        eprintln!("ADMIT-CAP-2-PG connection enabled={enabled} ended");
        drop(server);
        match end {
            ConnectionEnd::AdmittedThenDisconnected(_) => {}
            other => return Err(format!("unexpected connection end: {other:?}").into()),
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let casting = async {
        let mut handle = None;
        let mut saw_map = false;
        loop {
            let bytes = read_frame(&mut client)
                .await
                .map_err(|error| format!("field wire enabled={enabled}: {error}"))?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                for domain in decode_snapshot_chunk(envelope.payload()).map_err(debug)?.1 {
                    saw_map |= domain.domain_id
                        == oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW;
                    if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                        handle = decode_character_inventory(domain.payload)
                            .map_err(debug)?
                            .entries
                            .first()
                            .map(|item| item.handle);
                    }
                }
            }
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        assert!(
            saw_map,
            "the required map capability has a real bundle baseline"
        );
        let intent = UseIntent {
            target: UseTarget::Item(handle.ok_or("rune handle")?),
            use_with: None,
            use_with_map_item: None,
            use_with_position: Some(SpellTargetPosition {
                x: 101,
                y: 100,
                floor: 7,
            }),
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        let bytes = test_command(
            admitted.continuity.next_command_id,
            COMMAND_TYPE_USE_INTENT,
            &payload,
        );
        client.write_u32(u32::try_from(bytes.len())?).await?;
        client.write_all(&bytes).await?;
        loop {
            let bytes = read_frame(&mut client)
                .await
                .map_err(|error| format!("field wire enabled={enabled}: {error}"))?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                let disposition = decode_use_result(result.payload).map_err(debug)?;
                eprintln!("ADMIT-CAP-2-PG result enabled={enabled} disposition={disposition:?}");
                assert_eq!(
                    disposition,
                    if enabled {
                        UseDisposition::Committed
                    } else {
                        UseDisposition::Rejected
                    }
                );
                break;
            }
        }
        if enabled {
            client.write_u32(u32::try_from(bytes.len())?).await?;
            client.write_all(&bytes).await?;
            loop {
                let bytes = read_frame(&mut client)
                    .await
                    .map_err(|error| format!("field wire enabled={enabled}: {error}"))?;
                let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
                assert_ne!(envelope.message_type(), MessageType::ProtocolError);
                if envelope.message_type() == MessageType::CommandResult {
                    // FND-02 §13.2: the retained outcome replays; the family is not called
                    // again, which the single receipt and quantity below prove durably.
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.status, CommandStatus::DuplicateReplay);
                    assert_eq!(
                        decode_use_result(result.payload).map_err(debug)?,
                        UseDisposition::Committed
                    );
                    break;
                }
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let (served, result) = join_two(serving, casting).await;
        served?;
        result
    })
    .await??;
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE game_session_id=encode($1,'hex')::uuid")
        .bind(id(50).as_slice()).fetch_one(&harness.pool).await?;
    let quantity: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(71).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(receipts, i64::from(enabled));
    assert_eq!(quantity, if enabled { 2 } else { 3 });
    if !enabled {
        let runtime = dependencies.runtime.lock().await;
        assert_eq!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .get(&runtime, actor, session)
                .ok_or("caster")?
                .vitals(),
            before
        );
    }
    // Reload durable Ground custody independently of the runtime's physical installation.
    let ground: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_ground_locations g JOIN game_spell_item_lines l USING(item_instance_id) WHERE l.transaction_id IN(SELECT transaction_id FROM game_spell_item_receipts WHERE game_session_id=encode($1,'hex')::uuid)")
        .bind(id(50).as_slice()).fetch_one(&harness.pool).await?;
    assert_eq!(ground, i64::from(enabled));
    drop(owner);
    drop(authority);
    drop(seal);
    harness.cleanup().await?;
    eprintln!("ADMIT-CAP-2-PG real-wire enabled={enabled} verified");
    Ok(())
}
