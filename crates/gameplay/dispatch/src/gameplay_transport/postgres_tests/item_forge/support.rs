//! Shared, native Forge setup and actual command-181 connection driver.
use super::super::fixture::{Harness, debug, id, run};
use super::super::{Dependencies, TestResult, item_look, prepare_current_session};
use crate::domain::CharacterRevision;
use crate::durability::character_forge_dust::{
    ForgeDustGainCause, ForgeDustWriteIds, gain_forge_dust_in_transaction,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, IDLE_LIVENESS, serve_admitted,
};
use oteryn_gameplay_items::forge::{EXALTED_CORE_KEY, FORGE_SLIVER_KEY};
use oteryn_protocol_oteryn::{
    self as protocol, CommandStatus,
    item_forge::{Operation, Request},
};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, Clone, Copy)]
pub(in super::super) enum Case {
    Conversions,
    DustLimit,
    DustCap,
    Refusals,
    Fences,
    Concurrent,
    NoRoom,
    Stash,
    Nested,
    HistoryBound,
    RevisionChanged,
    DustStageChanged,
    SqlGuards,
    LegacyBudget,
    Integrity,
    CoreRefusal,
    MixedFee,
    Items(Operation),
}

pub(in super::super) fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "item_forge", true, false).await?;
        super::super::reset();
        prepare_current_session(&h).await?;
        sqlx::query(
            "INSERT INTO game_type2_audit_activation VALUES(1,now()) ON CONFLICT DO NOTHING",
        )
        .execute(&h.pool)
        .await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::super::death_ml_loss_composed::production_generation_with_pin(
                world,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let room = pin.qualified_room().clone();
        let frame_digest = pin.frame_binding().digest();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                crate::foundation::MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        deps.initialize(&owner).await;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(frame_digest.as_slice()).execute(&h.pool).await?;
        item_look::seed_main_backpack(&h, &[]).await?;
        let (dust, limit) = match case {
            Case::Items(_) | Case::Integrity | Case::CoreRefusal | Case::MixedFee => (1000, 1000),
            Case::Refusals => (59, 100),
            Case::DustCap => {
                let max = crate::domain::forge_dust::DustLimit::from_upgrade_count(
                    crate::domain::forge_dust::DUST_LIMIT_UPGRADES_MAX,
                )
                .map_err(debug)?
                .get();
                (i64::from(max), max)
            }
            _ => (100, 100),
        };
        seed_dust(&h, dust, limit).await?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[4, 6, 12, 14, 152],
        )
        .ok_or("forge capabilities")?;
        if matches!(
            case,
            Case::RevisionChanged | Case::DustStageChanged | Case::LegacyBudget
        ) {
            let gameplay = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "Forge initial gameplay fence")?
                .ok_or("Forge initial active session")?;
            assert!(owner.track_restored(
                admitted.game_session_id,
                deps.actor,
                &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin,
                gameplay,
            ));
        }
        match case {
            Case::Conversions => super::conversions(&h, &owner, &mut admitted).await?,
            Case::DustLimit => super::dust_limit(&h, &owner, &mut admitted).await?,
            Case::DustCap => super::dust_cap(&h, &owner, &mut admitted).await?,
            Case::Refusals => super::refusals(&h, &owner, &mut admitted).await?,
            Case::Fences => super::fences(&h, &owner, &mut admitted).await?,
            Case::Concurrent => super::concurrent(&h, &owner, &mut admitted).await?,
            Case::NoRoom => super::no_room(&h, &owner, &mut admitted).await?,
            Case::Stash => super::materials_and_history::stash(&h, &owner, &mut admitted).await?,
            Case::Nested => super::materials_and_history::nested(&h, &owner, &mut admitted).await?,
            Case::HistoryBound => {
                super::materials_and_history::history_bound(&h, &owner, &mut admitted).await?
            }
            Case::RevisionChanged => {
                super::revision_and_guards::revision_changed(&h, &owner, &deps, &mut admitted)
                    .await?
            }
            Case::DustStageChanged => {
                super::revision_and_guards::dust_stage_changed(&h, &owner, &deps, &mut admitted)
                    .await?
            }
            Case::SqlGuards => {
                super::revision_and_guards::sql_guards(&h, &owner, &mut admitted).await?
            }
            Case::LegacyBudget => {
                super::legacy_budget::exercise(&h, &owner, &deps, &mut admitted).await?
            }
            Case::Items(operation) => {
                super::super::item_forge_fusion::exercise(operation, &h, &owner, &mut admitted)
                    .await?
            }
            Case::Integrity => {
                super::super::item_forge_integrity::exercise(&h, &owner, &mut admitted).await?
            }
            Case::CoreRefusal => {
                super::super::item_forge_fusion::insufficient_cores(&h, &owner, &mut admitted)
                    .await?
            }
            Case::MixedFee => {
                super::super::item_forge_fusion::fee::exercise(&h, &owner, &mut admitted).await?
            }
        }
        eprintln!("FORGE POSTGRES_CASE_COMPLETED {case:?}");
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

pub(in super::super) fn request(operation: Operation, seed: u8) -> Request {
    Request {
        operation,
        occurrence: if operation == Operation::QueryHistory {
            [0; 16]
        } else {
            id(seed)
        },
        first_item: None,
        second_item: None,
        boost: false,
        protect: false,
        history_limit: if operation == Operation::QueryHistory {
            50
        } else {
            0
        },
    }
}

/// Fixture data only, following the existing native dust-chain PostgreSQL fixtures.
/// Production operations below retain all chain and cause guards.
async fn seed_dust(h: &Harness, dust: i64, limit: u16) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_forge_dust(character_id,balance,dust_limit) VALUES(encode($1,'hex')::uuid,0,100)").bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_forge_dust_entries(entry_id,character_id,kind,cause,cause_occurrence_id,transaction_id,amount,lost_amount,balance_before,balance_after,dust_limit_before,dust_limit_after) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'GAIN','creature_kill',encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,0,0,$5,$6,$6)")
        .bind(id(225).as_slice()).bind(id(41).as_slice()).bind(id(226).as_slice()).bind(id(227).as_slice()).bind(dust).bind(i16::try_from(limit)?).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_character_forge_dust SET balance=$1,dust_limit=$2,last_entry_id=encode($3,'hex')::uuid WHERE character_id=encode($4,'hex')::uuid")
        .bind(dust).bind(i16::try_from(limit)?).bind(id(225).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(in super::super) async fn gain_dust(
    h: &Harness,
    admitted: AdmittedSession,
    amount: u32,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("item fence")?;
    let revision: String = sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let gameplay = CurrentCharacterGameplayFence {
        character_id: fence.character_id,
        game_session_id: fence.game_session_id,
        connection_generation: fence.connection_generation,
        character_lease_generation: fence.character_lease_generation,
        runtime_scope: fence.runtime_scope,
        scope_ownership_generation: fence.scope_ownership_generation,
        expected_character_revision: CharacterRevision::new(revision.parse()?).map_err(debug)?,
    };
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    gain_forge_dust_in_transaction(
        &mut tx,
        &gameplay,
        ForgeDustGainCause::CreatureKill {
            occurrence: id(228),
        },
        amount,
        ForgeDustWriteIds {
            entry_id: id(229),
            transaction_id: id(230),
        },
    )
    .await
    .map_err(debug)?;
    tx.commit().await?;
    Ok(())
}

/// Arbitrary fixture ids, placed in the existing main backpack (id70).
pub(in super::super) async fn seed_items(h: &Harness, items: &[(u8, &str, i64, u8)]) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let ordinal: String=sqlx::query_scalar("SELECT COALESCE(max(placement_ordinal),0)::text FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(id(70).as_slice()).fetch_one(&mut *tx).await?;
    let ordinal = ordinal.parse::<u64>()?;
    for (offset, &(seed, key, quantity, tier)) in items.iter().enumerate() {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',$4,1,encode($5,'hex')::uuid)")
            .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(key).bind(quantity).bind(id(72).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(70).as_slice()).bind(id(seed).as_slice()).bind((ordinal+u64::try_from(offset)?+1).to_string()).bind(id(72).as_slice()).execute(&mut *tx).await?;
        if tier > 0 {
            sqlx::query("INSERT INTO game_item_tiers(item_instance_id,tier) VALUES(encode($1,'hex')::uuid,$2)").bind(id(seed).as_slice()).bind(i16::from(tier)).execute(&mut *tx).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

pub(in super::super) async fn cooldown_elapsed(h: &Harness) -> TestResult {
    // Advance the one durable test clock fact; never sleep a guessed wall-clock interval.
    sqlx::query("UPDATE game_forge_cooldowns SET next_action_at=1 WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).execute(&h.pool).await?;
    Ok(())
}

pub(in super::super) async fn balances(h: &Harness) -> TestResult<(i64, i16, i64, i64)> {
    let (dust,limit):(i64,i16)=sqlx::query_as("SELECT balance,dust_limit FROM game_character_forge_dust WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let count = async |key: &str| -> TestResult<i64> {
        Ok(sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE world_id=encode($1,'hex')::uuid AND definition_production_key=$2 AND lifecycle=1").bind(id(42).as_slice()).bind(key).fetch_one(&h.pool).await?)
    };
    Ok((
        dust,
        limit,
        count(FORGE_SLIVER_KEY).await?,
        count(EXALTED_CORE_KEY).await?,
    ))
}

pub(in super::super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    request: Request,
) -> TestResult<(CommandStatus, Value)> {
    let payload = request.encode().ok_or("canonical forge request")?;
    let (status, bytes) = exchange_raw(owner, admitted, &payload).await?;
    Ok((
        status,
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)?
        },
    ))
}

pub(in super::super) async fn exchange_raw(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let current = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, current, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let exercise = async {
        let result = send(
            &mut client,
            current.continuity.connection_generation,
            current.continuity.next_command_id,
            payload,
        )
        .await?;
        client.shutdown().await?;
        // Let the real command finish its inventory/container deltas before reconnecting.
        let mut remaining = Vec::new();
        client.read_to_end(&mut remaining).await?;
        Ok::<_, Box<dyn std::error::Error>>(result)
    };
    let (end, result) = tokio::time::timeout(
        std::time::Duration::from_secs(45),
        together(serving, exercise),
    )
    .await?;
    let server_end = server_end_summary(&end);
    *admitted = match end {
        ConnectionEnd::AdmittedThenDisconnected(next) => next,
        _ => return Err(format!("forge connection ended unexpectedly: {server_end}").into()),
    };
    result.map_err(|error| {
        format!(
            "Forge {:?} command {} failed: {error}; awaited server outcome: {server_end}",
            Request::decode(payload).map(|request| request.operation),
            current.continuity.next_command_id,
        )
        .into()
    })
}

pub(in super::super) fn server_end_summary(end: &ConnectionEnd) -> String {
    let (kind, admitted) = match end {
        ConnectionEnd::ProtocolViolation(error) => (format!("protocol violation: {error:?}"), None),
        ConnectionEnd::AdmissionRefused(reason) => (format!("admission refused: {reason:?}"), None),
        ConnectionEnd::ResumeUnavailable => ("resume unavailable".to_owned(), None),
        ConnectionEnd::AdmittedThenClosed(admitted, error) => {
            (format!("admitted then closed: {error:?}"), Some(admitted))
        }
        ConnectionEnd::AdmittedThenDisconnected(admitted) => {
            ("admitted then disconnected".to_owned(), Some(admitted))
        }
        ConnectionEnd::AdmittedThenControlLost(admitted) => {
            ("admitted then control lost".to_owned(), Some(admitted))
        }
        ConnectionEnd::AdmittedThenLoggedOut(admitted) => {
            ("admitted then logged out".to_owned(), Some(admitted))
        }
        ConnectionEnd::TransportFailed => ("transport failed".to_owned(), None),
    };
    if let Some(admitted) = admitted {
        format!(
            "{kind}; generation={}; next_command={}",
            admitted.continuity.connection_generation, admitted.continuity.next_command_id
        )
    } else {
        kind
    }
}

async fn send(
    client: &mut tokio::io::DuplexStream,
    generation: u64,
    id: u64,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    send_command(
        client,
        generation,
        id,
        protocol::item_forge::COMMAND_TYPE_ITEM_FORGE,
        payload,
    )
    .await
}

pub(in super::super) async fn send_command(
    client: &mut tokio::io::DuplexStream,
    generation: u64,
    id: u64,
    command_type: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let frame = protocol::encode_client_command(
        generation,
        &protocol::ClientCommandValue {
            command_id: id,
            command_type,
            payload,
        },
    )
    .map_err(debug)?;
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    let mut received = 0_u64;
    let mut last_message = None;
    loop {
        let frame = item_look::read_frame(client).await.map_err(|error| {
            format!(
                "command {id} type {command_type}: server frame read failed after {received} frames \
                 (last message: {last_message:?}): {error}"
            )
        })?;
        let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        received += 1;
        last_message = Some(envelope.message_type());
        if envelope.message_type() == protocol::MessageType::CommandResult {
            let result = protocol::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(
                result.command_id, id,
                "one response for the requested command"
            );
            return Ok((result.status, result.payload.to_vec()));
        }
    }
}

pub(in super::super) async fn together<A: std::future::Future, B: std::future::Future>(
    a: A,
    b: B,
) -> (A::Output, B::Output) {
    use std::task::Poll;
    let mut a = std::pin::pin!(a);
    let mut b = std::pin::pin!(b);
    let mut first = None;
    let mut second = None;
    std::future::poll_fn(|cx| {
        if first.is_none()
            && let Poll::Ready(value) = a.as_mut().poll(cx)
        {
            first = Some(value);
        }
        if second.is_none()
            && let Poll::Ready(value) = b.as_mut().poll(cx)
        {
            second = Some(value);
        }
        match (first.take(), second.take()) {
            (Some(a), Some(b)) => Poll::Ready((a, b)),
            (a, b) => {
                first = a;
                second = b;
                Poll::Pending
            }
        }
    })
    .await
}
