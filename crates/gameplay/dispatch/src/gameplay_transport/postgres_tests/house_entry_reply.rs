//! I10.client: framed entry and refusal through the real House handoff owner.
use super::TestResult;
use super::fixture::{Harness, debug, id};
use super::house_transfer_fixtures::{
    RecoverySource, TRANSFER_TEST_HOUSE as HOUSE, create_other_house_owner,
    initialize_transfer_source_for_owner, prepare_owned_session, prepare_transfer_scene,
};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::{GameSessionId, MessageType, decode_wire_envelope};
use crate::gameplay_transport::canonical_uuid;
use crate::gameplay_transport::connection::{IDLE_LIVENESS, serve_admitted};
use crate::gameplay_transport::fresh_evidence::FreshEvidenceSource;
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use oteryn_protocol_oteryn::house_transfer::COMMAND_TYPE_HOUSE_TRANSFER;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, DuplexStream};

#[test]
fn house_entry_owner_can_retry_after_preflight_refusal_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, Entrant::Owner)
}

#[test]
fn house_entry_uninvited_non_owner_gets_refusal_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, Entrant::OtherCharacter)
}

#[test]
fn house_entry_uninvited_other_account_gets_refusal_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, Entrant::OtherAccount)
}

enum Entrant {
    Owner,
    OtherCharacter,
    OtherAccount,
}

fn scenario(admin: String, entrant: Entrant) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("house-entry-reply", move || {
        let runtime = crate::bestiary_postgres_harness::runtime()?;
        runtime.block_on(async {
            let h = Harness::create_with_admission_seed(admin, "house_entry_reply", true, false)
                .await?;
            let result = exercise(&h, entrant).await;
            h.cleanup().await?;
            result
        })
    })
}

async fn exercise(h: &Harness, entrant: Entrant) -> TestResult {
    let evidence = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let is_owner = matches!(entrant, Entrant::Owner);
    let house_owner = match entrant {
        Entrant::Owner => id(41),
        Entrant::OtherCharacter => bootstrap_other_owner(h, &authority, &evidence.evidence).await?,
        Entrant::OtherAccount => create_other_house_owner(h, &authority, &evidence.evidence).await?,
    };
    let (controller, cells, deps) = prepare_transfer_scene(h).await?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = controller.active();
    owner.movement_cells = (&cells).into();
    owner.evidence = &evidence.evidence;
    let mut admitted = initialize_transfer_source_for_owner(
        h,
        &owner,
        &deps,
        cells.scope().generation_digest,
        house_owner,
    )
    .await?;
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM game_house_access(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid)
         LIMIT 1",
    )
    .bind(admitted.world_id.as_bytes().as_slice())
    .bind(HOUSE)
    .bind(id(41).as_slice())
    .fetch_optional(&h.pool)
    .await?;
    assert_eq!(role.as_deref(), is_owner.then_some("OWNER"));
    admitted.continuity.house_input_pending = Some(false); // A fresh Channel login.
    let generation = admitted.continuity.connection_generation;
    let entry = crate::gameplay_transport::house_query::entrance(HOUSE).ok_or("house entrance")?;

    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS).await;
        drop(server);
        Ok(end)
    };
    let commands = async {
        read_message(&mut client, MessageType::SnapshotCommit).await?;
        let held = if is_owner {
            // Complete the input gate's cold quest recovery before holding the
            // publication store. The transfer itself still runs through ingress.
            assert!(
                owner
                    .reconcile_quest_callbacks(admitted.game_session_id)
                    .await
            );
            // Hold the real publication store unavailable through the first entry attempt.
            h.root.await_ready().await?;
            Some(h.root.try_issue_semantic_pass()?)
        } else {
            None
        };
        assert_eq!(
            entry_status(&mut client, generation, 1).await?,
            CommandStatus::Rejected
        );
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            crate::foundation::MovementLocalPosition {
                x: entry.x,
                y: entry.y,
                floor: entry.floor
            },
        );
        if let Some(held) = held {
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
        }
        let expected = if is_owner {
            CommandStatus::Accepted
        } else {
            CommandStatus::Rejected
        };
        assert_eq!(entry_status(&mut client, generation, 2).await?, expected);
        if is_owner {
            read_message(&mut client, MessageType::SnapshotCommit).await?;
        }
        h.root.await_ready().await?;
        let recovery = h
            .root
            .house_transfer_recovery(admitted.game_session_id)
            .await?;
        if is_owner {
            let recovery = recovery.ok_or("committed house entry")?;
            let destination = GameSessionId::decode(&recovery.ack.session).map_err(debug)?;
            let inside = owner
                .runtime
                .lock()
                .await
                .house_actor_state(deps.actor, destination)
                .map_err(debug)?;
            assert_eq!(inside.house, HOUSE);
            // The transfer scene authors the door immediately north of the entrance.
            assert_eq!(
                inside.position,
                crate::foundation::LocalPosition {
                    x: entry.x,
                    y: entry.y - 1,
                    floor: entry.floor,
                }
            );
        } else {
            assert!(recovery.is_none());
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    tokio::time::timeout(Duration::from_secs(20), async {
        tokio::try_join!(serving, commands)
    })
    .await??;
    eprintln!("I10_HOUSE_ENTRY_POSTGRES_COMPLETED is_owner={is_owner}");
    Ok(())
}

async fn bootstrap_other_owner(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    evidence: &FreshEvidenceSource,
) -> TestResult<[u8; 16]> {
    // Keep the owner on the entrant's account so access must distinguish Characters.
    let account = canonical_uuid(&id(40));
    h.root.await_ready().await?;
    evidence
        .refresh_account(&h.root, &h.node, &account)
        .await
        .map_err(debug)?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let operation = id(91);
    let wire = serde_json::json!({
        "contract_version": 4,
        "variant": "OPERATOR_CONTROL_PLANE_BOOTSTRAP",
        "issuer_authority": "OTERYN_PLATFORM_CHARACTER_AUTHORITY",
        "issuer_decision_id": canonical_uuid(&id(92)),
        "source_revision": "2",
        "operation_id": canonical_uuid(&operation),
        "operation": "INITIAL_CHARACTER_BOOTSTRAP",
        "account_id": account,
        "target_world_id": canonical_uuid(&id(42)),
        "requested_name": "Other House Owner",
        "requested_sex": "Male",
        "requested_vocation": "Knight",
        "interpretation_context": {
            "profile_revision": "profile-1", "ruleset_revision": "ruleset-1",
            "content_revision": "content-1", "starter_template_revision": "starter-1"
        },
        "issued_at_source": now.to_string(),
        "expires_at_source": (now + 120).to_string(),
        "audience": "OTERYN_GAME_CHARACTER_AUTHORITY",
    });
    let intent = crate::character_bootstrap_intent::decode_producer_response(
        &serde_json::to_vec(&wire)?,
        operation,
    )
    .map_err(debug)?;
    h.root.await_ready().await?;
    let created = h
        .root
        .bootstrap_character(authority, &h.node, &intent)
        .await
        .map_err(debug)?;
    Ok(*created.character_id.as_bytes())
}

async fn read_message(client: &mut DuplexStream, kind: MessageType) -> TestResult<Vec<u8>> {
    loop {
        let frame = read_frame(client).await?;
        if decode_wire_envelope(&frame)?.message_type() == kind {
            return Ok(frame);
        }
    }
}

async fn entry_status(
    client: &mut DuplexStream,
    generation: u64,
    id: u64,
) -> TestResult<CommandStatus> {
    let mut payload = vec![8, 1, 18, HOUSE.len() as u8];
    payload.extend_from_slice(HOUSE.as_bytes());
    let frame = encode_client_command(
        generation,
        &ClientCommandValue {
            command_id: id,
            command_type: COMMAND_TYPE_HOUSE_TRANSFER,
            payload: &payload,
        },
    )?;
    write_frame(client, &frame).await?;
    let reply = read_message(client, MessageType::CommandResult)
        .await
        .map_err(|error| format!("missing house-entry reply for command {id}: {error}"))?;
    let result = decode_command_result(decode_wire_envelope(&reply)?.payload())?;
    assert_eq!(result.command_id, id);
    Ok(result.status)
}
