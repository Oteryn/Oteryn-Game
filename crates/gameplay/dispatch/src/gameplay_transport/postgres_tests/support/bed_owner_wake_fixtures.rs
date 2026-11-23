use super::TestResult;
use super::fixture::{Harness, debug, id};
use super::server_modal_bed::command;
use crate::content::LogicalCell;
use crate::durability::bed_sleep::BedUseOutcome;
use crate::durability::consume_transform::HouseActionContext;
use crate::foundation::{AuthenticatedTransportRefV1, CommandId, CommandRef};
use crate::gameplay_transport::item_view::{ItemKey, SessionItemView};
use crate::gameplay_transport::session_ingress::{CommandAction, SessionCommandStream};
use crate::gameplay_transport::{
    AdmissionRetryBudget, ComposedFreshAdmission,
    connection::{AdmittedSession, ControlLossResult, IDLE_LIVENESS, serve_admitted},
    tcp_tls::read_frame,
};
use crate::map::{overlay::TilePos, view::placement_key};
use oteryn_protocol_oteryn::{
    ClientCommandView, CommandStatus, MessageType, decode_command_result, decode_snapshot_chunk,
    decode_wire_envelope,
    server_modal::{COMMAND_TYPE_SERVER_MODAL_QUERY, QUERY_WITH_TERMINAL_REPLY, Reply},
    world_map::{MapOrigin, STATE_DOMAIN_WORLD_MAP_VIEW, decode_world_map_snapshot},
    world_object::{
        COMMAND_TYPE_USE_INTENT, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
        decode_use_result, decode_use_result_with_gate, encode_item_use_intent,
    },
};

const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";

#[derive(Clone, Copy, Debug)]
pub(super) enum Case {
    Replay,
    Revoked,
    WorldOnly,
    NonPremium,
    Cooldown,
    Contended,
    Resume,
}

const HEAD: LogicalCell = LogicalCell {
    x: 32310,
    y: 32804,
    z: 7,
};
const FIRST_SLEEPER: u8 = 121;
const NEXT_SLEEPER: u8 = 122;

pub(super) async fn scenario(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    case: Case,
) -> TestResult {
    // Only the historical sleepers are seeded. The USE actor's session, lease
    // and House custody come from genuine fresh admission and native handoff.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    for (sleeper, name) in [
        (FIRST_SLEEPER, "First Sleeper"),
        (NEXT_SLEEPER, "Second Sleeper"),
    ] {
        let record: String = sqlx::query_scalar(
            "SELECT (to_jsonb(r) || jsonb_build_object('character_id',encode($1,'hex')::uuid,'name',$2::text))::text \
             FROM game_character_roots r WHERE character_id=encode($3,'hex')::uuid",
        )
        .bind(id(sleeper).as_slice()).bind(name).bind(id(41).as_slice())
        .fetch_one(&mut *tx).await?;
        super::hireling_lifecycle_fixtures::insert_character_root_copy(&mut tx, &record).await?;
    }
    tx.commit().await?;
    seed_sleeper(h, FIRST_SLEEPER).await?;

    match case {
        Case::Replay => {}
        Case::Revoked => return revoked(h, owner, admitted).await,
        Case::Contended => return contended(h, owner, admitted).await,
        Case::Resume => return resume(h, owner, admitted).await,
        Case::WorldOnly | Case::NonPremium | Case::Cooldown => {
            return wire_case(h, owner, admitted, case).await;
        }
    }

    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let playing = async move {
        let (head, foot) = bed_targets(&mut client).await?;
        let mut next = admitted.continuity.next_command_id;
        let mut stale = admitted;
        stale
            .controller
            .as_mut()
            .ok_or("owner transport")?
            .transport = AuthenticatedTransportRefV1::decode(&id(123)).map_err(debug)?;
        let revision = bed_revision(h).await?;
        let refused = owner
            .offer_bed_modal(stale, next, foot.clone(), None)
            .await
            .map_err(debug)?
            .ok_or("stale transport must reach the bed owner")?;
        assert_eq!(refused.disposition, UseDisposition::StaleState);
        assert_eq!(bed_revision(h).await?, revision);

        let (status, bytes) = command(
            &mut client,
            admitted,
            &mut next,
            COMMAND_TYPE_SERVER_MODAL_QUERY,
            QUERY_WITH_TERMINAL_REPLY,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(Reply::decode(&bytes).map_err(debug)?, Reply::None);
        super::m44_source_lifecycle_fixtures::wait_for_world_action(owner, &admitted).await?;
        let wake_occurrence = next;
        assert_eq!(
            use_bed(&mut client, admitted, &mut next, foot.clone()).await?,
            (CommandStatus::Accepted, UseDisposition::BedWoken)
        );
        assert_eq!(bed_revision(h).await?, revision + 1);
        let first_free = freed(h, FIRST_SLEEPER).await?;
        let freed_at = first_free.1.ok_or("owner wake timestamp")?;
        let observed_at: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&h.pool)
                .await?;
        assert!((first_free.0 + 31_000..=observed_at).contains(&freed_at));
        assert_eq!(first_free.2.as_deref(), Some("owner_wake"));

        seed_sleeper(h, NEXT_SLEEPER).await?;
        let revision = bed_revision(h).await?;
        // The in-order connection returns its cached reply to the original wire
        // occurrence. Then exercise the durable replay below that cache as well.
        let mut repeated = wake_occurrence;
        assert_eq!(
            use_bed(&mut client, admitted, &mut repeated, foot).await?,
            (CommandStatus::DuplicateReplay, UseDisposition::BedWoken)
        );
        let context = {
            let runtime = owner.runtime.lock().await;
            HouseActionContext::from_native_owner(
                &runtime,
                admitted.runtime_actor.ok_or("House actor")?,
                admitted.game_session_id,
                admitted.item_fence.ok_or("House custody fence")?,
            )?
        };
        let replay = owner
            .production_bed_use(
                admitted,
                wake_occurrence,
                context,
                HEAD,
                &AdmissionRetryBudget::for_command(owner.root),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(replay, BedUseOutcome::Woken));
        assert_eq!(bed_revision(h).await?, revision);
        assert_eq!(freed(h, NEXT_SLEEPER).await?.1, None);
        assert_eq!(freed(h, FIRST_SLEEPER).await?, first_free);
        let (status, bytes) = command(
            &mut client,
            admitted,
            &mut next,
            COMMAND_TYPE_SERVER_MODAL_QUERY,
            QUERY_WITH_TERMINAL_REPLY,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(Reply::decode(&bytes).map_err(debug)?, Reply::None);

        super::m44_source_lifecycle_fixtures::wait_for_world_action(owner, &admitted).await?;
        assert_eq!(
            use_bed(&mut client, admitted, &mut next, head.clone()).await?,
            (CommandStatus::Accepted, UseDisposition::BedWoken)
        );
        assert_eq!(bed_revision(h).await?, revision + 1);
        assert_eq!(
            freed(h, NEXT_SLEEPER).await?.2.as_deref(),
            Some("owner_wake")
        );

        make_guest(h).await?;
        sqlx::query(
            "DELETE FROM game_house_bed_sleepers WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(NEXT_SLEEPER).as_slice())
        .execute(&h.pool)
        .await?;
        seed_sleeper(h, NEXT_SLEEPER).await?;
        let revision = bed_revision(h).await?;
        super::m44_source_lifecycle_fixtures::wait_for_world_action(owner, &admitted).await?;
        assert_eq!(
            use_bed(&mut client, admitted, &mut next, head).await?,
            (CommandStatus::Accepted, UseDisposition::BedOccupied)
        );
        assert_eq!(bed_revision(h).await?, revision);
        assert_eq!(freed(h, NEXT_SLEEPER).await?.1, None);
        let (status, bytes) = command(
            &mut client,
            admitted,
            &mut next,
            COMMAND_TYPE_SERVER_MODAL_QUERY,
            QUERY_WITH_TERMINAL_REPLY,
        )
        .await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(Reply::decode(&bytes).map_err(debug)?, Reply::None);
        let receipts: Vec<String> = sqlx::query_scalar(
            "SELECT outcome FROM game_house_bed_use_receipts ORDER BY command_id",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(receipts, ["woken", "woken", "occupied"]);
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (_, result) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serving, playing),
    )
    .await?;
    result
}

async fn house_context(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult<HouseActionContext> {
    let runtime = owner.runtime.lock().await;
    Ok(HouseActionContext::from_native_owner(
        &runtime,
        admitted.runtime_actor.ok_or("House actor")?,
        admitted.game_session_id,
        admitted.item_fence.ok_or("House custody fence")?,
    )?)
}

async fn revoked(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    use crate::durability::platform_streams::{SecurityRevocation, StreamPage};
    let item = SecurityRevocation {
        revocation_id: id(91),
        account_id: id(40),
        native_security_generation: 2,
        cause: 1,
        effective_at_ms: 1_700_000_000_000,
    };
    h.root
        .apply_platform_stream_page(
            None,
            StreamPage::SecurityRevocations(vec![item]),
            Some("v1:1".into()),
        )
        .await
        .map_err(debug)?;
    assert!(
        h.root
            .security_session_revoked(admitted.game_session_id)
            .await
            .map_err(debug)?
    );
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session(admitted.game_session_id)
            .await
            .map_err(debug)?;
    assert_eq!(
        current.session_state(),
        crate::foundation::GameSessionState::Active
    );
    assert_eq!(
        current.current_transport(),
        admitted.controller.map(|binding| binding.transport)
    );

    let decision = owner
        .production_bed_use(
            admitted,
            admitted.continuity.next_command_id,
            house_context(owner, admitted).await?,
            HEAD,
            &AdmissionRetryBudget::for_command(owner.root),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(decision, BedUseOutcome::Stale));
    assert_eq!(freed(h, FIRST_SLEEPER).await?.1, None);
    Ok(())
}

async fn contended(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let context = house_context(owner, admitted).await?;
    assert!(
        owner
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current()
    );
    let held = h.root.try_acquire_ready().map_err(debug)?;
    assert!(!h.root.is_ready(), "the sole holder is checked out");
    let (decision, returned) = super::fixture::join_two(
        owner.production_bed_use(
            admitted,
            admitted.continuity.next_command_id + 1,
            context,
            HEAD,
            &AdmissionRetryBudget::for_command(owner.root),
        ),
        return_contended_holder(held),
    )
    .await;
    assert!(matches!(
        returned,
        sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle
    ));
    assert!(matches!(decision.map_err(debug)?, BedUseOutcome::Woken));
    assert_eq!(
        freed(h, FIRST_SLEEPER).await?.2.as_deref(),
        Some("owner_wake")
    );

    seed_sleeper(h, NEXT_SLEEPER).await?;
    wire_case(h, owner, admitted, Case::Contended).await
}

async fn return_contended_holder(
    mut holder: sqlx::pool::PoolConnection<sqlx::Postgres>,
) -> sqlx::pool::PoolConnectionReturnDisposition {
    tokio::time::sleep(std::time::Duration::from_millis(75)).await;
    holder
        .return_to_pool_observed_until(
            std::time::Instant::now() + crate::durability::DB_PASS_DEADLINE,
        )
        .await
}

async fn resume(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let command = admitted.continuity.next_command_id;
    let reference = CommandRef::new(
        admitted.game_session_id,
        CommandId::new(command).map_err(debug)?,
    );
    let mut stream = SessionCommandStream::open(Some(&owner.states.command_ingress), &admitted)
        .ok_or("original ingress stream")?;
    stream
        .begin(
            command,
            admitted.continuity.connection_generation,
            admitted.continuity.server_sequence,
            false,
        )
        .map_err(debug)?;
    let outcome = owner
        .production_bed_use(
            admitted,
            command,
            house_context(owner, admitted).await?,
            HEAD,
            &AdmissionRetryBudget::for_command(owner.root),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(outcome, BedUseOutcome::Woken));
    // Model a lost storage acknowledgement after COMMIT. Do not publish the
    // result to the connection's still-pending occurrence.
    drop(stream);
    assert!(
        owner
            .states
            .command_ingress
            .pending_reconciliation(reference)
    );
    assert_eq!(
        owner
            .lose_control_for_cause(admitted, std::time::Duration::ZERO, None)
            .await,
        ControlLossResult::Recorded
    );
    let wrong_transport = AuthenticatedTransportRefV1::decode(&id(123)).map_err(debug)?;
    let fence = admitted.item_fence.ok_or("original House fence")?;
    assert!(
        h.root
            .read_house_bed_use_receipt(
                fence,
                wrong_transport,
                reference.command_id(),
                std::time::Instant::now() + crate::durability::DB_PASS_DEADLINE,
            )
            .await
            .map_err(debug)?
            .is_none()
    );
    seed_sleeper(h, NEXT_SLEEPER).await?;
    let revision = bed_revision(h).await?;
    let resumed = super::hireling_lifecycle_fixtures::bed_owner_resume(owner, admitted).await?;
    assert_eq!(resumed.continuity.next_command_id, command + 1);
    // Resume accounts for the recovered result, then publishes the retained House handoff ACK.
    assert_eq!(
        resumed.continuity.server_sequence,
        admitted.continuity.server_sequence + 2
    );
    assert!(
        owner
            .states
            .command_ingress
            .original_terminalized(reference)
    );
    let mut resumed_stream =
        SessionCommandStream::open(Some(&owner.states.command_ingress), &resumed)
            .ok_or("reconciled stream")?;
    let CommandAction::Replay(frame) = resumed_stream
        .begin(
            command,
            resumed.continuity.connection_generation,
            resumed.continuity.server_sequence,
            false,
        )
        .map_err(debug)?
    else {
        return Err("original wake must replay its retained result".into());
    };
    let envelope = decode_wire_envelope(&frame).map_err(debug)?;
    let result = decode_command_result(envelope.payload()).map_err(debug)?;
    assert_eq!(result.status, CommandStatus::DuplicateReplay);
    assert_eq!(
        decode_use_result(result.payload).map_err(debug)?,
        UseDisposition::BedWoken
    );
    assert_eq!(bed_revision(h).await?, revision);
    assert_eq!(freed(h, NEXT_SLEEPER).await?.1, None);
    Ok(())
}

async fn wire_case(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    case: Case,
) -> TestResult {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let playing = async move {
        let (head, foot) = bed_targets(&mut client).await?;
        let mut next = admitted.continuity.next_command_id;
        if matches!(case, Case::NonPremium) {
            let body = bed_use_payload(head)?;
            let (status, reply) = command(
                &mut client,
                admitted,
                &mut next,
                COMMAND_TYPE_USE_INTENT,
                &body,
            )
            .await?;
            assert_eq!(
                decode_use_result_with_gate(&reply).map_err(debug)?,
                (UseDisposition::Sealed, Some(54))
            );
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(freed(h, FIRST_SLEEPER).await?.1, None);
        } else if matches!(case, Case::Contended) {
            let held = h.root.try_acquire_ready().map_err(debug)?;
            let (reply, returned) = super::fixture::join_two(
                use_bed(&mut client, admitted, &mut next, foot),
                return_contended_holder(held),
            )
            .await;
            assert!(matches!(
                returned,
                sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle
            ));
            assert_eq!(reply?, (CommandStatus::Accepted, UseDisposition::BedWoken));
            assert_eq!(
                freed(h, NEXT_SLEEPER).await?.2.as_deref(),
                Some("owner_wake")
            );
        } else {
            assert_eq!(
                use_bed(&mut client, admitted, &mut next, head).await?,
                (CommandStatus::Accepted, UseDisposition::BedWoken)
            );
            if matches!(case, Case::Cooldown) {
                assert_eq!(
                    use_bed(&mut client, admitted, &mut next, foot.clone()).await?,
                    (CommandStatus::Accepted, UseDisposition::Exhausted)
                );
                assert_eq!(
                    handle_use(owner, admitted, next, foot).await?,
                    UseDisposition::Exhausted
                );
                let receipt_count: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_house_bed_use_receipts")
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(receipt_count, 1);
            } else {
                super::m44_source_lifecycle_fixtures::wait_for_world_action(owner, &admitted)
                    .await?;
                assert_eq!(
                    use_bed(&mut client, admitted, &mut next, foot.clone()).await?,
                    (CommandStatus::Rejected, UseDisposition::Rejected),
                    "a vacant bed still requires the skill-selection capability"
                );
                seed_sleeper(h, NEXT_SLEEPER).await?;
                super::m44_source_lifecycle_fixtures::wait_for_world_action(owner, &admitted)
                    .await?;
                assert_eq!(
                    handle_use(owner, admitted, next, foot).await?,
                    UseDisposition::BedWoken
                );
                assert_eq!(
                    freed(h, NEXT_SLEEPER).await?.2.as_deref(),
                    Some("owner_wake")
                );
            }
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (_, result) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serving, playing),
    )
    .await?;
    result
}

async fn handle_use(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    command_id: u64,
    target: UseTarget,
) -> TestResult<UseDisposition> {
    let UseTarget::WorldObject(target) = target else {
        return Err("reference bed part".into());
    };
    let key = ItemKey::MapBase {
        bundle_digest: target.placement[..32].try_into()?,
        placement_key: u64::from_be_bytes(target.placement[32..].try_into()?),
        reset_epoch: owner.reset_epoch,
    };
    let mut view = SessionItemView::resume(admitted.continuity.item_view).with_map_view();
    let handle = view
        .map_view(&[key], |table| Ok(table.handle(&key)))
        .map_err(debug)?
        .ok_or("bed map handle")?;
    let payload = bed_use_payload(UseTarget::Item(handle))?;
    let command = ClientCommandView {
        command_id,
        command_type: COMMAND_TYPE_USE_INTENT,
        payload: &payload,
    };
    Ok(
        Box::pin(owner.route_bed_handle_for_test(&mut admitted, command, view))
            .await
            .ok_or("item-handle route must produce a terminal bed use outcome")?
            .disposition,
    )
}

async fn bed_targets(client: &mut tokio::io::DuplexStream) -> TestResult<(UseTarget, UseTarget)> {
    let mut head = None;
    let mut foot = None;
    loop {
        let frame = read_frame(client).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        match envelope.message_type() {
            MessageType::SnapshotChunk => {
                for domain in decode_snapshot_chunk(envelope.payload()).map_err(debug)?.1 {
                    if domain.domain_id != STATE_DOMAIN_WORLD_MAP_VIEW {
                        continue;
                    }
                    let map = decode_world_map_snapshot(domain.payload).map_err(debug)?;
                    for tile in &map.tiles {
                        let target = match (tile.position.x, tile.position.y, tile.position.floor) {
                            (32310, 32804, 7) => &mut head,
                            (32311, 32804, 7) => &mut foot,
                            _ => continue,
                        };
                        let MapOrigin::BaseOrdinal {
                            ordinal,
                            object_revision,
                        } = tile.items.get(1).ok_or("visible bed part")?.origin
                        else {
                            return Err("fixed bed part must have a base ordinal".into());
                        };
                        let key = placement_key(
                            TilePos {
                                x: u16::try_from(tile.position.x)?,
                                y: u16::try_from(tile.position.y)?,
                                floor: -7,
                            },
                            ordinal,
                        )
                        .ok_or("bed placement")?;
                        let mut placement = map.header.bundle_digest.to_vec();
                        placement.extend_from_slice(&key.to_be_bytes());
                        *target = Some(UseTarget::WorldObject(WorldObjectTarget {
                            placement,
                            expected_revision: object_revision,
                        }));
                    }
                }
            }
            MessageType::SnapshotCommit => {
                return Ok((
                    head.ok_or("bed head from House snapshot")?,
                    foot.ok_or("bed foot from House snapshot")?,
                ));
            }
            _ => {}
        }
    }
}

async fn use_bed(
    client: &mut tokio::io::DuplexStream,
    admitted: AdmittedSession,
    next: &mut u64,
    target: UseTarget,
) -> TestResult<(CommandStatus, UseDisposition)> {
    let payload = bed_use_payload(target)?;
    let (status, bytes) =
        command(client, admitted, next, COMMAND_TYPE_USE_INTENT, &payload).await?;
    Ok((status, decode_use_result(&bytes).map_err(debug)?))
}

fn bed_use_payload(target: UseTarget) -> TestResult<Vec<u8>> {
    encode_item_use_intent(&UseIntent {
        target,
        use_with: None,
        use_with_map_item: None,
        use_with_position: None,
    })
    .map_err(debug)
}

async fn seed_sleeper(h: &Harness, sleeper: u8) -> TestResult {
    sqlx::query(
        "INSERT INTO game_house_bed_sleepers \
         (character_id,world_id,house_key,bed_x,bed_y,bed_z,lease_generation,occupied_variant,slept_at) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,32310,32804,7,1,'male',\
         floor(extract(epoch FROM clock_timestamp())*1000)::bigint - 31000)",
    )
    .bind(id(sleeper).as_slice()).bind(id(42).as_slice()).bind(HOUSE)
    .execute(&h.pool).await?;
    Ok(())
}

async fn freed(h: &Harness, sleeper: u8) -> TestResult<(i64, Option<i64>, Option<String>)> {
    Ok(sqlx::query_as("SELECT slept_at,freed_at,freed_cause FROM game_house_bed_sleepers WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(sleeper).as_slice()).fetch_one(&h.pool).await?)
}

async fn bed_revision(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT revision::bigint FROM game_house_bed_revisions WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
        .bind(id(42).as_slice()).bind(HOUSE).fetch_one(&h.pool).await?)
}

async fn make_guest(h: &Harness) -> TestResult {
    // Import a settled change of landlord and guest grant after both sleepers
    // were freed. The original owner's runtime custody and transport stay live.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_house_properties SET owner_character_id=encode($1,'hex')::uuid,ownership_generation=ownership_generation+1,revision=revision+1,acl_revision=acl_revision+1 WHERE world_id=encode($2,'hex')::uuid AND house_key=$3")
        .bind(id(FIRST_SLEEPER).as_slice()).bind(id(42).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_house_ownership SET owner_character_id=encode($1,'hex')::uuid,ownership_revision=ownership_revision+1 WHERE world_id=encode($2,'hex')::uuid AND house_key=$3")
        .bind(id(FIRST_SLEEPER).as_slice()).bind(id(42).as_slice()).bind(HOUSE).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_house_acl_entries(world_id,house_key,list_key,character_id) VALUES(encode($1,'hex')::uuid,$2,'guest',encode($3,'hex')::uuid)")
        .bind(id(42).as_slice()).bind(HOUSE).bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
