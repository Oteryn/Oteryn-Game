use super::TestResult;
use super::fixture::{Harness, debug, id};
use super::house_balloon::{Case, run_case};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{
        AdmittedSession, ConnectionEnd, FreshAdmissionAuthority, IDLE_LIVENESS, LivenessPolicy,
        serve_admitted,
    },
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use oteryn_protocol_oteryn::{item_inscription, item_view, world_spatial_entities as entities};
use oteryn_protocol_oteryn::{actor_motion, appearance};
use oteryn_protocol_oteryn::mail::{self, MailIntent, MailOutcome};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn house_resident_tick_postgres() -> TestResult {
    run_case(Case::ResidentTick)
}

#[test]
fn house_resident_tick_with_due_furniture_postgres() -> TestResult {
    run_case(Case::ResidentExpiryTick)
}

#[test]
fn house_resident_letter_6246_postgres() -> TestResult {
    run_case(Case::ResidentLetter)
}

#[test]
fn house_resident_mail_without_inscription_postgres() -> TestResult {
    run_case(Case::ResidentMail)
}

#[test]
fn house_resident_equip_moves_helmet_postgres() -> TestResult {
    run_case(Case::ResidentEquip)
}

#[test]
fn house_resident_energy_ring_expires_postgres() -> TestResult {
    run_case(Case::ResidentTimedEquip(ClockCase::Expiry))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ClockCase {
    Expiry,
    Handoff,
    Logout,
    Grace,
    Writer,
    Unequip,
}

#[test]
fn house_resident_ring_handoff_settles_elapsed_postgres() -> TestResult {
    run_case(Case::ResidentTimedEquip(ClockCase::Handoff))
}

#[test]
fn house_resident_ring_logout_after_acl_revocation_postgres() -> TestResult {
    run_case(Case::ResidentTimedEquip(ClockCase::Logout))
}

#[test]
fn house_resident_ring_expires_in_reconnect_grace_postgres() -> TestResult {
    run_case(Case::ResidentTimedEquip(ClockCase::Grace))
}

#[test]
fn house_resident_ring_tick_yields_to_registration_writer_postgres() -> TestResult {
    run_case(Case::ResidentTimedEquip(ClockCase::Writer))
}

#[test]
fn house_resident_ring_unequip_settles_elapsed_postgres() -> TestResult {
    run_case(Case::ResidentTimedEquip(ClockCase::Unequip))
}

#[test]
fn house_resident_revoked_appearance_postgres() -> TestResult {
    run_case(Case::ResidentAppearanceRevoked)
}

pub(super) async fn appearance_revoked(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("guest actor")?;
    let entity = entities::EntityRef {
        identity: actor.placement_identity(),
        generation: actor.actor_local_generation(),
    };
    let observe = || owner.appearance_observation(actor, admitted.game_session_id, vec![entity]);
    assert_eq!(observe().await.ok_or("guest appearance")?.bindings.len(), 1);
    let house = super::house_balloon::HOUSE;
    for operation in [Some(id(201)), None] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        let changed = sqlx::query("UPDATE game_house_properties SET content_fence_operation=encode($3,'hex')::uuid WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
            .bind(id(42).as_slice()).bind(house).bind(operation.as_ref().map(|id| id.as_slice()))
            .execute(&mut *tx).await?;
        assert_eq!(changed.rows_affected(), 1);
        tx.commit().await?;
        assert_eq!(
            observe().await.is_none(),
            operation.is_some(),
            "property content fence gates appearance"
        );
    }
    let mut tx = h.pool.begin().await?;
    // Model the committed ACL change while retaining the already admitted actor.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let deleted = sqlx::query("DELETE FROM game_house_acl_entries WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND list_key='guest' AND character_id=encode($3,'hex')::uuid")
        .bind(id(42).as_slice()).bind(house).bind(id(41).as_slice())
        .execute(&mut *tx).await?;
    assert_eq!(deleted.rows_affected(), 1);
    tx.commit().await?;
    assert!(
        observe().await.is_none(),
        "revoked guest still receives appearance"
    );
    Ok(())
}

#[test]
fn house_resident_mount_and_dismount_visible_postgres() -> TestResult {
    run_case(Case::ResidentMount)
}

#[test]
fn house_resident_quest_titles_postgres() -> TestResult {
    run_case(Case::ResidentQuest)
}

pub(super) async fn presentation_exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    case: Case,
) -> TestResult {
    use oteryn_protocol_oteryn::server_info::{self, Query, Reply};
    if case == Case::ResidentQuest {
        let (status, bytes) = super::cyclopedia::support::exchange(
            owner, &mut admitted, 1, server_info::COMMAND_TYPE_SERVER_INFO_QUERY,
            &Query::Quests.encode().ok_or("quest query")?,
        ).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert!(matches!(Reply::decode(&bytes), Some(Reply::Quests(titles))
            if titles.len() == 1 && !titles[0].1.is_empty()));
        return Ok(());
    }

    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async move {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let observing = async {
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            if decode_wire_envelope(&raw).map_err(debug)?.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        for (command_id, mounted, look) in [(1, true, 139), (3, false, 0), (5, true, 139)] {
            let frame = encode_client_command(
                admitted.continuity.connection_generation,
                &ClientCommandValue {
                    command_id,
                    command_type: actor_motion::COMMAND_TYPE_MOUNT,
                    payload: &actor_motion::encode_mount(mounted),
                },
            )
            .map_err(debug)?;
            crate::gameplay_transport::tcp_tls::write_frame(&mut client, &frame).await?;
            loop {
                let raw = super::item_look::read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&raw).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let reply = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(reply.command_id, command_id);
                    assert_eq!(reply.status, CommandStatus::Accepted);
                    break;
                }
            }
            let motion = next_motion(&mut client, admitted).await?;
            assert_eq!(motion.mount_look_type, look);
            let actor = admitted.runtime_actor.ok_or("mount observer")?;
            let view = owner.observe_visible_entities(actor).await.ok_or("mounted visibility")?;
            let motion = view.entities.iter().find(|entity| entity.identity == actor.placement_identity())
                .and_then(|entity| entity.motion).ok_or("House resident motion")?;
            assert_eq!(motion.mount_look_type, look);
            use oteryn_protocol_oteryn::world_spatial::{
                self as spatial, StepDirection, StepDisposition,
            };
            let frame = encode_client_command(
                admitted.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: command_id + 1,
                    command_type: 1,
                    payload: &spatial::encode_step_intent(StepDirection::East),
                },
            )
            .map_err(debug)?;
            crate::gameplay_transport::tcp_tls::write_frame(&mut client, &frame).await?;
            loop {
                let raw = super::item_look::read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&raw).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let reply = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(reply.command_id, command_id + 1);
                    assert_eq!(reply.status, CommandStatus::Accepted);
                    assert_eq!(
                        spatial::decode_step_result_paced(reply.payload, true).map_err(debug)?,
                        StepDisposition::Moved
                    );
                    break;
                }
            }
            let walked = next_motion(&mut client, admitted).await?;
            assert!(walked.translation_revision > motion.translation_revision);
            if command_id == 5 {
                let position = owner
                    .runtime
                    .lock()
                    .await
                    .read_house_movement(actor, admitted.game_session_id)
                    .map_err(debug)?
                    .state()
                    .position;
                assert_eq!((position.x, position.y), (32311, 32803));
                assert_eq!(walked.step_duration_ms, 0, "an adjacent teleport must snap");
            } else {
                assert!(walked.step_duration_ms > 0, "a House step must animate");
            }
            assert_eq!(walked.mount_look_type, look);
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (_, result) = tokio::time::timeout(std::time::Duration::from_secs(12), async {
        tokio::join!(serving, observing)
    }).await?;
    result
}

async fn next_motion(
    client: &mut tokio::io::DuplexStream,
    admitted: AdmittedSession,
) -> TestResult<actor_motion::ActorMotion> {
    let identity = admitted
        .runtime_actor
        .ok_or("mount observer")?
        .placement_identity();
    loop {
        let raw = super::item_look::read_frame(client).await?;
        let envelope = decode_wire_envelope(&raw).map_err(debug)?;
        if envelope.message_type() != MessageType::StateDelta {
            continue;
        }
        let delta =
            oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
        if delta.domain_id != 1 || delta.delta_type != 2 {
            continue;
        }
        let (delta, _, _) = entities::decode_world_spatial_entities_delta_with_sidecar(
            delta.payload,
            entities::EntityFields::selected(admitted.continuity.selected_capabilities.as_slice()),
        )
        .map_err(debug)?;
        if let Some(motion) = delta
            .update
            .iter()
            .find(|entry| entry.entity.identity == identity)
            .and_then(|entry| entry.motion)
        {
            return Ok(motion);
        }
    }
}

pub(super) async fn select_mount(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    mount_key: String,
) -> TestResult {
    sqlx::query("INSERT INTO game_account_appearance_unlock_receipts VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'resident-mount','content-1',encode($3,'hex')::uuid,'quest',$1)")
        .bind(id(199).as_slice()).bind(id(40).as_slice()).bind(id(41).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_account_appearance_unlocks VALUES(encode($1,'hex')::uuid,'resident-mount',encode($2,'hex')::uuid)")
        .bind(id(40).as_slice()).bind(id(199).as_slice()).execute(&h.pool).await?;
    owner.admit_progression_session(admitted).await;
    let actor = admitted.runtime_actor.ok_or("mount actor")?;
    let initial = owner.appearance_observation(actor, admitted.game_session_id, Vec::new()).await.ok_or("mount catalogue")?;
    let change = appearance::Change {
        context: initial.catalogue.context,
        expected_root_revision: initial.catalogue.root_revision,
        prior: None,
        selection: appearance::Selection {
            outfit_key: initial.catalogue.outfits.first().ok_or("outfit")?.key.clone(),
            colours: [0; 4],
            addons: 0,
            mount_key: Some(mount_key),
        },
        content_revision: "content-1".into(),
    };
    let outcome = owner.change_appearance(actor, crate::gameplay_transport::connection::UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 100,
        item_fence: admitted.item_fence,
    }, change).await;
    assert!(matches!(outcome, Some(appearance::ChangeResult::Committed(_))));
    Ok(())
}

pub(super) async fn exchange(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    letter: std::num::NonZeroU32,
    case: Case,
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,\
                          vice_deficit_since,revision,created_at) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'House Guild',2,9999999999999,1,1,1)",
    )
    .bind(id(90).as_slice())
    .bind(id(42).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_guild_members(character_id,guild_id,rank_level,title,joined_at) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,'',1)",
    )
    .bind(id(41).as_slice())
    .bind(id(90).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_item_text_revisions VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(71).as_slice())
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_item_texts \
         VALUES(encode($1,'hex')::uuid,'House letter',encode($2,'hex')::uuid,now(),1)",
    )
    .bind(id(71).as_slice())
    .bind(id(41).as_slice())
    .execute(&mut *tx)
    .await?;
    if matches!(case, Case::ResidentTimedEquip(clock) if clock != ClockCase::Logout) {
        // The native ring requires an eligible vocation before its clock can start.
        sqlx::query("INSERT INTO game_character_build_state(character_id,vocation,magic_level,mana_spent,fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries,committed_character_revision,last_build_occurrence_id) VALUES(encode($1,'hex')::uuid,'monk',10,0,10,0,10,0,10,0,10,0,10,0,10,0,10,0,2,encode($2,'hex')::uuid) ON CONFLICT(character_id) DO UPDATE SET vocation='monk'")
            .bind(id(41).as_slice()).bind(id(30).as_slice()).execute(&mut *tx).await?;
    }
    if case == Case::ResidentExpiryTick {
        let house = super::house_balloon::HOUSE;
        let position = [32310_i32, 32804, 7].map(i32::to_be_bytes).concat();
        sqlx::query("INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING")
            .bind(id(42).as_slice()).bind(house).bind(&position).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i35950','definition-r1',1,1,encode($1,'hex')::uuid)")
            .bind(id(80).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_interior_locations VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,1,encode($1,'hex')::uuid)")
            .bind(id(80).as_slice()).bind(id(42).as_slice()).bind(house).bind(&position).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($1,'hex')::uuid,1)")
            .bind(id(80).as_slice()).bind(id(42).as_slice()).bind(house).bind(id(101).as_slice()).execute(&mut *tx).await?;
        sqlx::query(
            "INSERT INTO game_item_timed_states VALUES(encode($1,'hex')::uuid,NULL,1000,floor(extract(epoch FROM clock_timestamp())*1000)::bigint,1)",
        )
        .bind(id(80).as_slice())
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    let tick = matches!(case, Case::ResidentTick | Case::ResidentExpiryTick);
    if tick {
        use std::time::Duration;
        let mut writer = h.pool.begin().await?;
        sqlx::query("LOCK TABLE game_durability_reconnect_sessions IN EXCLUSIVE MODE")
            .execute(&mut *writer).await?;
        let (capacity, emblems) = tokio::time::timeout(Duration::from_secs(1), async {
            let cap = owner.observe_inventory_capacity(admitted).await;
            let emblems = owner.observe_guild_emblems(admitted).await;
            (cap, emblems)
        }).await?;
        assert!(capacity.is_some());
        assert!(!emblems.ok_or("House emblems while writer is busy")?.is_empty());
        writer.rollback().await?;
    }
    let shared = &*owner;
    let expiry = case == Case::ResidentExpiryTick;
    let policy = if expiry {
        LivenessPolicy {
            interval: std::time::Duration::from_millis(200),
            missed_limit: 1,
        }
    } else {
        IDLE_LIVENESS
    };
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let observe = async move {
        let mut writer = None;
        let mut capacity = false;
        let mut emblem = false;
        let mut letter_handle = None;
        let mut ack_after_tick = false;
        loop {
            let read = super::item_look::read_frame(&mut client);
            let raw = if expiry && writer.is_some() {
                tokio::time::timeout(std::time::Duration::from_millis(900), read)
                    .await
                    .map_err(|_| "House expiry stalled liveness with a locked registration")??
            } else {
                read.await?
            };
            let envelope = decode_wire_envelope(&raw).map_err(debug)?;
            match envelope.message_type() {
                MessageType::SnapshotCommit if tick => {
                    let mut lock = h.pool.begin().await?;
                    if case == Case::ResidentExpiryTick {
                        sqlx::query("SELECT node_id FROM game_node_registrations WHERE node_id=encode($1,'hex')::uuid FOR UPDATE")
                            .bind(shared.holder.fact().node_id().as_bytes().as_slice())
                            .fetch_one(&mut *lock).await?;
                        let house = crate::durability::house_scope_handoff::HouseId::new(
                            admitted.world_id,
                            super::house_balloon::HOUSE,
                        )
                        .ok_or("fixture House")?;
                        assert!(shared.root.observe_due_house_furniture(&house).await?);
                    } else {
                        sqlx::query(
                            "LOCK TABLE game_durability_reconnect_sessions IN EXCLUSIVE MODE",
                        )
                        .execute(&mut *lock)
                        .await?;
                    }
                    writer = Some((lock, tokio::time::Instant::now()));
                }
                MessageType::LivenessProbe if expiry => {
                    let probe = oteryn_protocol_oteryn::decode_liveness_probe(envelope.payload())
                        .map_err(debug)?;
                    let ack = oteryn_protocol_oteryn::encode_liveness_ack(
                        admitted.continuity.connection_generation,
                        probe,
                        0,
                    )
                    .map_err(debug)?;
                    crate::gameplay_transport::tcp_tls::write_frame(&mut client, &ack).await?;
                    if capacity && emblem {
                        if ack_after_tick && let Some((lock, _)) = writer.take() {
                            // With a one-miss limit, this probe proves ingress handled the
                            // previous ACK after the service tick, while the writer was held.
                            assert_eq!(decoration_phase(h).await?, "oteryn:item.tibia.i35950");
                            lock.rollback().await?;
                        }
                        ack_after_tick = true;
                        if writer.is_none()
                            && decoration_phase(h).await? == "oteryn:item.tibia.i35951"
                        {
                            client.shutdown().await?;
                            return Ok(());
                        }
                    }
                }
                MessageType::SnapshotCommit
                    if case == Case::ResidentMail && letter_handle.is_none() =>
                {
                    // Still drive the real MAIL route if a regression omits the inventory snapshot.
                    let payload = mail::encode_intent(&MailIntent::WriteText {
                        item_handle: 1,
                        text: "Edited in the House".into(),
                    })
                    .map_err(debug)?;
                    let frame = encode_client_command(
                        admitted.continuity.connection_generation,
                        &ClientCommandValue {
                            command_id: 1,
                            command_type: u32::from(mail::COMMAND_TYPE_MAIL),
                            payload: &payload,
                        },
                    )
                    .map_err(debug)?;
                    crate::gameplay_transport::tcp_tls::write_frame(&mut client, &frame).await?;
                }
                MessageType::SnapshotChunk
                    if matches!(
                        case,
                        Case::ResidentLetter
                            | Case::ResidentMail
                            | Case::ResidentEquip
                            | Case::ResidentTimedEquip(_)
                    ) =>
                {
                    let (_, domains) =
                        oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())
                            .map_err(debug)?;
                    for domain in domains {
                        if domain.domain_id != item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                            continue;
                        }
                        let inventory = item_view::decode_character_inventory_with_equip_drop(
                            domain.payload,
                            matches!(case, Case::ResidentEquip | Case::ResidentTimedEquip(_)),
                        )
                        .map_err(debug)?;
                        let handle = inventory
                            .entries
                            .iter()
                            .find(|entry| entry.item_definition_ref == letter)
                            .ok_or("letter handle")?
                            .handle;
                        letter_handle = Some(handle);
                        let (command_type, request) = if case == Case::ResidentMail {
                            (
                                u32::from(mail::COMMAND_TYPE_MAIL),
                                mail::encode_intent(&MailIntent::WriteText {
                                    item_handle: handle.get(),
                                    text: "Edited in the House".into(),
                                })
                                .map_err(debug)?,
                            )
                        } else if matches!(case, Case::ResidentEquip | Case::ResidentTimedEquip(_)) {
                            (
                                item_view::COMMAND_TYPE_ITEM_MOVE_INTENT,
                                item_view::encode_item_move_intent_with_equip_drop(
                                    &item_view::ItemMoveIntent {
                                        source: handle,
                                        destination: item_view::ItemMoveDestination::Equipment(
                                            if matches!(case, Case::ResidentTimedEquip(_)) {
                                                item_view::EquipmentSlot::Ring
                                            } else {
                                                item_view::EquipmentSlot::Head
                                            },
                                        ),
                                    },
                                    true,
                                )
                                .map_err(debug)?,
                            )
                        } else {
                            (
                                item_inscription::COMMAND_TYPE_ITEM_INSCRIPTION_READ,
                                item_inscription::Request {
                                    item_handle: handle.get(),
                                }
                                .encode()
                                .ok_or("request")?,
                            )
                        };
                        let frame = encode_client_command(
                            admitted.continuity.connection_generation,
                            &ClientCommandValue {
                                command_id: 1,
                                command_type,
                                payload: &request,
                            },
                        )
                        .map_err(debug)?;
                        client.write_u32(u32::try_from(frame.len())?).await?;
                        client.write_all(&frame).await?;
                    }
                }
                MessageType::CommandResult
                    if matches!(
                        case,
                        Case::ResidentMail | Case::ResidentEquip | Case::ResidentTimedEquip(_)
                    ) =>
                {
                    let reply = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(reply.command_id, 1);
                    if case == Case::ResidentMail {
                        assert_eq!(reply.status, CommandStatus::Accepted);
                        assert_eq!(reply.payload, [MailOutcome::TextWritten as u8]);
                    } else {
                        assert_eq!(reply.status, CommandStatus::Accepted);
                        assert_eq!(
                            item_view::decode_item_move_result_with_equip_drop(reply.payload, true)
                                .map_err(debug)?,
                            item_view::ItemMoveOutcome::Moved
                        );
                    }
                    break;
                }
                MessageType::CommandResult if case == Case::ResidentLetter => {
                    let reply = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(reply.status, CommandStatus::Accepted);
                    if reply.command_id == 2 {
                        assert_eq!(reply.payload, [MailOutcome::TextWritten as u8]);
                        break;
                    }
                    assert_eq!(
                        item_inscription::Reply::decode(reply.payload),
                        Some(item_inscription::Reply {
                            text: "House letter".into(),
                            writable: true,
                            max_length: 1999
                        })
                    );
                    let payload = mail::encode_intent(&MailIntent::WriteText {
                        item_handle: letter_handle.ok_or("letter handle missing")?.get(),
                        text: "Edited in the House".into(),
                    }).map_err(debug)?;
                    let frame = encode_client_command(
                        admitted.continuity.connection_generation,
                        &ClientCommandValue {
                            command_id: 2,
                            command_type: u32::from(mail::COMMAND_TYPE_MAIL),
                            payload: &payload,
                        },
                    ).map_err(debug)?;
                    client.write_u32(u32::try_from(frame.len())?).await?;
                    client.write_all(&frame).await?;
                }
                MessageType::StateDelta if tick => {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                        .map_err(debug)?;
                    if delta.domain_id != 1 || delta.delta_type != 2 {
                        continue;
                    }
                    let fields = entities::EntityFields::selected(
                        admitted.continuity.selected_capabilities.as_slice(),
                    );
                    match entities::decode_world_spatial_entities_delta_with_sidecar(
                        delta.payload,
                        fields,
                    )
                    .map_err(debug)?
                    .2
                    {
                        Some(entities::DeltaSidecar::InventoryCapacity(value)) => {
                            capacity = value.free < value.capacity
                        }
                        Some(entities::DeltaSidecar::GuildEmblems(entries)) => {
                            emblem = entries.iter().any(|entry| {
                                entry.emblem == oteryn_protocol_oteryn::guild_emblem::Emblem::Member
                                    && Some(entry.entity.identity)
                                        == admitted
                                            .runtime_actor
                                            .map(|actor| actor.placement_identity())
                            })
                        }
                        _ => (),
                    }
                    if capacity && emblem && !expiry {
                        let (lock, started) =
                            writer.take().ok_or("writer held during House tick")?;
                        assert!(
                            started.elapsed() < std::time::Duration::from_secs(2),
                            "House tick waited for admission writer"
                        );
                        // An ACK for an unsent probe yields a distinct ingress error.
                        // Keep the writer held until the connection handles it.
                        let ack = oteryn_protocol_oteryn::encode_liveness_ack(
                            admitted.continuity.connection_generation,
                            u64::MAX,
                            0,
                        )
                        .map_err(debug)?;
                        crate::gameplay_transport::tcp_tls::write_frame(&mut client, &ack).await?;
                        tokio::time::timeout(
                            std::time::Duration::from_secs(1),
                            client.read_to_end(&mut Vec::new()),
                        )
                        .await??;
                        lock.rollback().await?;
                        return Ok::<_, Box<dyn std::error::Error>>(());
                    }
                }
                _ => (),
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, result) = tokio::time::timeout(std::time::Duration::from_secs(12), async {
        tokio::join!(
            async move {
                let end = serve_admitted(&mut server, admitted, shared, policy).await;
                drop(server);
                end
            },
            observe
        )
    })
    .await?;
    result?;
    if case == Case::ResidentTick {
        assert!(
            matches!(
                end,
                ConnectionEnd::AdmittedThenClosed(
                    _,
                    oteryn_protocol_oteryn::FoundationProtocolError::InvalidWireIdentifier,
                )
            ),
            "House tick must service the queued ACK while the writer is held"
        );
    }
    if matches!(case, Case::ResidentLetter | Case::ResidentMail) {
        let text: String = sqlx::query_scalar(
            "SELECT text FROM game_item_texts WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(id(71).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(text, "Edited in the House");
    }
    if case == Case::ResidentEquip {
        let slot: Option<i16> = sqlx::query_scalar(
            "SELECT slot FROM game_character_equipment_slots WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(id(71).as_slice())
        .fetch_optional(&h.pool)
        .await?;
        assert_eq!(slot, Some(1));
    }
    if let Case::ResidentTimedEquip(clock) = case
        && clock != ClockCase::Expiry
    {
        let admitted = match end {
            ConnectionEnd::AdmittedThenDisconnected(current) => current,
            _ => return Err(format!("ring equip connection ended {end:?}").into()),
        };
        return check_ring_clock(h, owner, admitted, clock).await;
    }
    if matches!(case, Case::ResidentTimedEquip(_)) {
        owner.clock_origin -= std::time::Duration::from_secs(601);
        owner.tick_timed_items().await;
        let lifecycle: i16 = sqlx::query_scalar(
            "SELECT lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
        ).bind(id(71).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(lifecycle, 2, "the owner tick burns the worn energy ring");
    }
    // A retained session with the wrong named House cannot authorize any of these reads.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let changed = sqlx::query(
        "UPDATE game_durability_reconnect_sessions \
         SET runtime_scope_house_key=$3, runtime_scope_instance_id=game_house_scope_instance_id(runtime_scope_world_id,$3) \
         WHERE game_session_id=encode($1,'hex')::uuid AND runtime_scope_house_key=$2",
    )
    .bind(admitted.game_session_id.as_bytes().as_slice())
    .bind(super::house_balloon::HOUSE)
    .bind("oteryn:content.house.wrong")
    .execute(&mut *tx)
    .await?;
    assert_eq!(changed.rows_affected(), 1);
    tx.commit().await?;
    assert!(owner.observe_inventory_capacity(admitted).await.is_none());
    assert!(owner.observe_guild_emblems(admitted).await.is_none());
    assert!(
        owner
            .item_inscription_read(
                admitted,
                Some(crate::gameplay_transport::item_view::ItemKey::Instance(id(
                    71
                )))
            )
            .await
            .is_none()
    );
    Ok(())
}

async fn decoration_phase(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT definition_production_key FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(80).as_slice()).fetch_one(&h.pool).await?)
}

pub(super) async fn prepare_logout_vocation(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildOccurrence, convert_vocation,
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("Channel gameplay fence before House entry")?;
    let formula = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.training_formula())
        .ok_or("logout training formula")?;
    let before = owner
        .root
        .read_character_build_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let after = convert_vocation(formula, &before, "monk").map_err(debug)?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    slot.commit_build(
        owner.root,
        owner.character,
        owner.holder,
        fence,
        BuildChangeRequest {
            occurrence: BuildOccurrence::from_bytes(id(74)).map_err(debug)?,
            cause: BuildCause::VocationChoice,
            before,
            after,
            pruned_stance: None,
        },
        formula,
        None,
    )
    .await
    .map_err(debug)?;
    Ok(())
}

async fn check_ring_clock(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    case: ClockCase,
) -> TestResult {
    use crate::gameplay_transport::connection::ControlLossResult;
    use std::time::Duration;
    let actor = admitted.runtime_actor.ok_or("House actor")?;
    let session = admitted.game_session_id;
    let budget = || async {
        let remaining: Option<i64> = sqlx::query_scalar(
            "SELECT remaining_ms FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid",
        ).bind(id(71).as_slice()).fetch_optional(&h.pool).await?;
        Ok::<_, sqlx::Error>(remaining.unwrap_or(600_000))
    };
    if case == ClockCase::Grace {
        let (before, after) = mana_after_hit(owner, actor, session).await?;
        assert!(after < before, "worn ring grants a shield");
        assert_eq!(
            owner.lose_control(admitted, Duration::ZERO, None).await,
            ControlLossResult::Recorded
        );
        owner.clock_origin -= Duration::from_secs(601);
        owner.tick_timed_items().await;
        owner.clock_origin -= Duration::from_secs(2);
        owner.tick_timed_items().await;
        {
            let runtime = owner.runtime.lock().await;
            assert!(owner.states.spell_states.lock().await.resume_house(
                &runtime,
                actor,
                session,
                owner.owner_now(),
            ));
        }
        let (before, after) = mana_after_hit(owner, actor, session).await?;
        assert_eq!(before, after, "expired ring must leave no shield on resume");
        return Ok(());
    }

    owner.clock_origin -= Duration::from_secs(30);
    if case == ClockCase::Writer {
        let mut writer = h.pool.begin().await?;
        sqlx::query("SELECT node_id FROM game_node_registrations WHERE node_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(owner.holder.fact().node_id().as_bytes().as_slice())
            .fetch_one(&mut *writer).await?;
        tokio::time::timeout(Duration::from_millis(500), owner.tick_timed_items())
            .await
            .map_err(|_| "resident clock held the spell lane behind the registration writer")?;
        writer.rollback().await?;
        return Ok(());
    }

    owner.tick_timed_items().await;
    let before = budget().await?;
    assert!(
        before > 590_000,
        "periodic checkpoint should defer the partial debit"
    );
    match case {
        ClockCase::Handoff => {
            let house = super::house_balloon::HOUSE;
            let mut exit = vec![8, 2, 18, u8::try_from(house.len())?];
            exit.extend_from_slice(house.as_bytes());
            let (outside, _) = owner
                .transfer_house(admitted, 2, &exit)
                .await
                .ok_or("House exit")?;
            assert!(outside.is_some(), "the native handoff must commit");
        }
        ClockCase::Logout => {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            let removed = sqlx::query("DELETE FROM game_house_acl_entries WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND list_key='guest' AND character_id=encode($3,'hex')::uuid")
                .bind(id(42).as_slice()).bind(super::house_balloon::HOUSE)
                .bind(id(41).as_slice()).execute(&mut *tx).await?;
            assert_eq!(removed.rows_affected(), 1);
            tx.commit().await?;
            assert_eq!(
                owner.logout(admitted).await,
                crate::gameplay_transport::logout::LogoutReply::Result(
                    oteryn_protocol_oteryn::logout::LogoutResult::Accepted,
                ),
            );
        }
        ClockCase::Unequip => unequip_ring(owner, admitted).await?,
        _ => return Err("unexpected energy ring clock case".into()),
    }
    let remaining = budget().await?;
    assert!(
        (560_000..=570_000).contains(&remaining),
        "{case:?} must settle 30 seconds of equipped time, remaining {remaining}"
    );
    Ok(())
}

async fn mana_after_hit(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> TestResult<(u32, u32)> {
    let spells = owner.states.spell_states.lock().await;
    let (_, _, state) = spells
        .actors
        .iter()
        .find(|(held, id, _)| *held == actor && *id == session)
        .ok_or("retained spell state")?;
    let (hit, _) =
        crate::spell::actor_conditions::stage_creature_hit(state, 5, owner.owner_now().get())
            .map_err(debug)?;
    Ok::<_, Box<dyn std::error::Error>>((state.vitals().mana, hit.vitals().mana))
}

async fn unequip_ring(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serve = async {
        serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
    };
    let drive = async {
        let mut handle = None;
        loop {
            let bytes = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            match envelope.message_type() {
                MessageType::SnapshotChunk => {
                    let (_, domains) =
                        oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())
                            .map_err(debug)?;
                    for domain in domains {
                        if domain.domain_id == item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                            let inventory = item_view::decode_character_inventory_with_equip_drop(
                                domain.payload,
                                true,
                            )
                            .map_err(debug)?;
                            handle = inventory
                                .equipment
                                .into_iter()
                                .find(|entry| entry.slot == item_view::EquipmentSlot::Ring)
                                .map(|entry| entry.item.handle);
                        }
                    }
                }
                MessageType::SnapshotCommit => break,
                _ => (),
            }
        }
        let request = item_view::encode_item_move_intent_with_equip_drop(
            &item_view::ItemMoveIntent {
                source: handle.ok_or("ring handle in current snapshot")?,
                destination: item_view::ItemMoveDestination::MainBackpack,
            },
            true,
        )
        .map_err(debug)?;
        let command_id = admitted.continuity.next_command_id;
        let frame = encode_client_command(
            admitted.continuity.connection_generation,
            &ClientCommandValue {
                command_id,
                command_type: item_view::COMMAND_TYPE_ITEM_MOVE_INTENT,
                payload: &request,
            },
        )
        .map_err(debug)?;
        write_frame(&mut client, &frame).await?;
        loop {
            let bytes = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let reply = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(reply.command_id, command_id);
                assert_eq!(reply.status, CommandStatus::Accepted);
                assert_eq!(
                    item_view::decode_item_move_result_with_equip_drop(reply.payload, true)
                        .map_err(debug)?,
                    item_view::ItemMoveOutcome::Moved,
                );
                break;
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (_, result) = tokio::time::timeout(
        std::time::Duration::from_secs(12),
        super::fixture::join_two(serve, drive),
    )
    .await?;
    result
}
