//! PARTY-1 real admitted transport, native durable writer and reloaded owner.
use crate::bestiary_postgres_harness::{
    self as fixture, CHANNEL, Harness, TestResult, WORLD, debug, id,
};
use crate::foundation::{ChannelId, ChannelRuntimeV1, MessageType, WorldId, decode_wire_envelope};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::SelectedCapabilities,
    connection::{
        AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
        SessionContinuity, serve_admitted,
    },
    tcp_tls::{read_frame, write_frame},
};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command, party::*,
    world_spatial_entities::EntityRef,
};
use std::time::Duration;
use tokio::{io::AsyncWriteExt, sync::Mutex};

#[test]
fn party_membership_real_dispatch_postgres_end_to_end() -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime().map_err(debug)?.block_on(async move {
                let h = Harness::create(admin, "party_membership_wire", false)
                    .await
                    .map_err(debug)?;
                let result = scenario(&h).await.map_err(|e| e.to_string());
                h.cleanup().await.map_err(debug)?;
                result
            })
        })?
        .join()
        .map_err(|_| "party wire thread panicked")??;
    Ok(())
}
#[derive(Clone, Copy)]
enum Step {
    Query,
    Invite(EntityRef),
    Accept,
    Pass(usize),
    Kick(usize),
    Leave,
    Malformed,
}

async fn roundtrip(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    steps: &[Step],
) -> TestResult<Vec<PartyResult>> {
    let session = *admitted;
    let (mut server, mut client) = tokio::io::duplex(16_384);
    // Each peer owns its stream so an early return closes it and wakes the other peer.
    let serving = async move { serve_admitted(&mut server, session, owner, IDLE_LIVENESS).await };
    let driving = async move {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        let mut results: Vec<PartyResult> = Vec::new();
        for (offset, step) in steps.iter().enumerate() {
            let command_id = session.continuity.next_command_id + offset as u64;
            let intent = match step {
                Step::Invite(target) => Some(PartyIntent::Invite(*target)),
                Step::Accept => Some(PartyIntent::Accept(
                    results
                        .last()
                        .ok_or("query first")?
                        .invitations
                        .first()
                        .ok_or("invitation")?
                        .handle,
                )),
                Step::Pass(index) => Some(PartyIntent::PassLeadership(
                    results
                        .last()
                        .ok_or("query first")?
                        .members
                        .get(*index)
                        .ok_or("member")?
                        .handle,
                )),
                Step::Kick(index) => Some(PartyIntent::Kick(
                    results
                        .last()
                        .ok_or("query first")?
                        .members
                        .get(*index)
                        .ok_or("member")?
                        .handle,
                )),
                Step::Leave => Some(PartyIntent::Leave),
                Step::Query | Step::Malformed => None,
            };
            let payload = if matches!(step, Step::Malformed) {
                vec![8, 0]
            } else if let Some(intent) = intent {
                encode_party_intent(intent).map_err(debug)?
            } else {
                Vec::new()
            };
            let command_type = if matches!(step, Step::Query) {
                COMMAND_TYPE_PARTY_QUERY
            } else {
                COMMAND_TYPE_PARTY_INTENT
            };
            let frame = encode_client_command(
                session.continuity.connection_generation,
                &ClientCommandValue {
                    command_id,
                    command_type,
                    payload: &payload,
                },
            )
            .map_err(debug)?;
            write_frame(&mut client, &frame).await?;
            loop {
                let frame = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() != MessageType::CommandResult {
                    continue;
                }
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, command_id);
                if matches!(step, Step::Malformed)
                    || !session
                        .continuity
                        .selected_capabilities
                        .contains(CAPABILITY_PARTY_V1)
                {
                    assert_eq!(result.status, CommandStatus::Rejected);
                    assert!(result.payload.is_empty());
                } else {
                    assert_eq!(result.status, CommandStatus::Accepted);
                    results.push(decode_party_result(result.payload).map_err(debug)?);
                }
                break;
            }
        }
        client.shutdown().await?;
        Ok(results)
    };
    let (end, result): (_, TestResult<Vec<PartyResult>>) = tokio::time::timeout(
        Duration::from_secs(30),
        super::fixture::join_two(serving, driving),
    )
    .await?;
    match end {
        ConnectionEnd::AdmittedThenDisconnected(current) => *admitted = current,
        other => return Err(format!("party serving ended: {other:?}").into()),
    }
    result
}
async fn scenario(h: &Harness) -> TestResult {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let mut currents = Vec::new();
    for (character, account, session) in [(60, 63, 70), (61, 64, 71), (62, 65, 72)] {
        currents.push(
            crate::world_party_runtime_postgres_cases::committed_fresh_player(
                h, character, account, session, now,
            )
            .await?,
        );
    }
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(WORLD)).map_err(debug)?;
    let channel = ChannelId::decode(&id(CHANNEL)).map_err(debug)?;
    let room = crate::content::qualify_native_entry_room(world).map_err(debug)?;
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        h.node.fact().node_id(),
        h.node.fact().registration_revision(),
        1,
        1,
        "runtime-scope-assignment:1",
        5,
        ChannelContentPin::test(world),
    )
    .map_err(debug)?;
    let mut admitted = Vec::new();
    for current in &currents {
        let session = current.current_game_session_id();
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, current.current_character_lease())
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        admitted.push(AdmittedSession {
            game_session_id: session,
            world_id: world,
            channel_id: channel,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: crate::gameplay_transport::item_fence_of(*current),
        });
    }
    let entities = runtime.visible_entities().players;
    let targets: Vec<_> = entities
        .iter()
        .map(|p| EntityRef {
            identity: p.actor.placement_identity(),
            generation: p.generation,
        })
        .collect();
    let content = crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
        .map_err(debug)?;
    let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_test(
        currents[0].current_runtime_scope(),
        currents[0].current_scope_generation(),
        content,
    );
    let door = Mutex::new(
        crate::world_runtime::bind_native_entry_door(
            room.door(),
            &door_fence,
            currents[0].current_runtime_scope(),
            currents[0].current_scope_generation(),
        )
        .map_err(debug)?,
    );
    let descriptor=crate::native_admission_source::descriptor::ProducerDescriptor::new("platform-test".into(),("127.0.0.1".into(),1),"source.test".into(),"source.test".into(),
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/ca.der").to_vec())],
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec())],
        rustls::pki_types::PrivateKeyDer::try_from(include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der").to_vec())?).map_err(debug)?;
    let evidence = crate::gameplay_transport::fresh_evidence::FreshEvidenceSource::new(descriptor);
    let spells = crate::spell::cast::v1_spell_book().map_err(debug)?;
    let achievements =
        crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?;
    let charms =
        crate::content::charm_source::CanonicalCharmCatalogue::embedded().map_err(debug)?;
    let runtime = Mutex::new(runtime);
    let make_owner = || {
        crate::gameplay_transport::composed_test_owner::ComposedTestOwner {
        root: &h.root, character: &recovery, holder: &h.node, evidence: &evidence, world_id: world, channel_id: channel,
        required_capabilities: crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(&[],currents[0].current_runtime_scope(),1,"policy-1",crate::gameplay_transport::world_required_capabilities::offered_for_scope(false,None)).expect("bound"),
        runtime: &runtime, door: &door, spells: &spells, achievements: &achievements, imported_charms: &charms, room: &room,
    }.build()
    };
    let mut owner = make_owner();
    for a in &mut admitted {
        a.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[6, 42])
                .ok_or("party offer")?;
    }
    // Old clients and malformed payloads cannot reach the durable writer.
    let mut old = admitted[0];
    old.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[6]).ok_or("old selection")?;
    roundtrip(&owner, &mut old, &[Step::Leave]).await?;
    // One GameSession keeps one command stream: the ended continuity carries into the next
    // connection, restoring the party selection for the current client.
    let selected = admitted[0].continuity.selected_capabilities;
    admitted[0].continuity = old.continuity;
    admitted[0].continuity.selected_capabilities = selected;
    roundtrip(
        &owner,
        &mut admitted[0],
        &[
            Step::Malformed,
            Step::Invite(targets[1]),
            Step::Invite(targets[2]),
        ],
    )
    .await?;
    let mut invisible = targets[1];
    invisible.generation += 1;
    let refused = roundtrip(&owner, &mut admitted[0], &[Step::Invite(invisible)]).await?;
    assert_eq!(refused[0].disposition, PartyDisposition::NotVisible);
    // Use a genuinely fresh connection: carried continuity first reconciles the original
    // bank command, which must disconnect on a stale fence before party dispatch can run.
    let mut stale = admitted[2];
    stale
        .item_fence
        .as_mut()
        .ok_or("fence")?
        .character_lease_generation = 1;
    let refused = roundtrip(&owner, &mut stale, &[Step::Leave]).await?;
    assert_eq!(refused[0].disposition, PartyDisposition::Rejected);
    // Carry the stale connection's ended continuity while keeping the current lease.
    admitted[2].continuity = stale.continuity;
    for a in admitted.iter_mut().skip(1) {
        let results = roundtrip(&owner, a, &[Step::Query, Step::Accept]).await?;
        assert_eq!(results[0].invitations.len(), 1);
        assert_eq!(results[1].disposition, PartyDisposition::Ok);
    }
    let result = roundtrip(&owner, &mut admitted[0], &[Step::Query, Step::Pass(1)]).await?;
    assert_eq!(result[0].members.len(), 3);
    assert_eq!(result[1].disposition, PartyDisposition::Ok);
    let result = roundtrip(&owner, &mut admitted[0], &[Step::Query, Step::Kick(2)]).await?;
    assert_eq!(result[1].disposition, PartyDisposition::NotLeader);
    let result = roundtrip(&owner, &mut admitted[1], &[Step::Query, Step::Pass(0)]).await?;
    assert_eq!(result[1].disposition, PartyDisposition::Ok);
    let result = roundtrip(&owner, &mut admitted[0], &[Step::Leave]).await?;
    assert_eq!(result[0].disposition, PartyDisposition::Ok);
    // Rebuild the production owner and all connection-local handles. SQL remains authority.
    // The Channel keeps each GameSession's command stream (SESSION-INGRESS-1), so the
    // sessions continue on their ended continuity while the party caches start empty.
    let commands = std::mem::take(&mut owner.states.command_ingress);
    drop(owner);
    owner = make_owner();
    owner.states.command_ingress = commands;
    let result = roundtrip(&owner, &mut admitted[1], &[Step::Query]).await?;
    assert_eq!(result[0].members.len(), 2);
    assert!(
        result[0].members[0].leader,
        "earliest invited member succeeds"
    );
    // Re-form three members, then exercise the current leader's kick.
    let result = roundtrip(&owner, &mut admitted[1], &[Step::Invite(targets[0])]).await?;
    assert_eq!(result[0].disposition, PartyDisposition::Ok);
    roundtrip(&owner, &mut admitted[0], &[Step::Query, Step::Accept]).await?;
    let result = roundtrip(&owner, &mut admitted[1], &[Step::Query, Step::Kick(1)]).await?;
    assert_eq!(result[1].disposition, PartyDisposition::Ok);
    let result = roundtrip(&owner, &mut admitted[2], &[Step::Query]).await?;
    assert!(result[0].members.is_empty());
    let actor = admitted[1].runtime_actor.ok_or("actor")?;
    {
        let rt = runtime.lock().await;
        owner.states.attack.lock().await.record_hit_taken(
            &rt,
            actor,
            admitted[1].game_session_id,
            owner.owner_now(),
        );
    }
    let blocked_command = admitted[1].continuity.next_command_id;
    let blocked = roundtrip(&owner, &mut admitted[1], &[Step::Leave]).await?;
    assert_eq!(blocked[0].disposition, PartyDisposition::LogoutBlocked);
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(61))
        .ok_or("clock")?;
    // The same receipted command keeps its original refusal even after its lock expires.
    let payload = encode_party_intent(PartyIntent::Leave).map_err(debug)?;
    let mut view = crate::gameplay_transport::party_command::SessionPartyView::default();
    let replay = owner
        .party_command(
            admitted[1],
            blocked_command,
            COMMAND_TYPE_PARTY_INTENT,
            &payload,
            &mut view,
        )
        .await
        .ok_or("replay result")?;
    assert_eq!(
        decode_party_result(&replay).map_err(debug)?.disposition,
        PartyDisposition::LogoutBlocked
    );
    // Each durable PvP deadline independently bars leaving, including state restored on restart.
    let member = currents[1].current_character_lease().character_id();
    for column in ["logout_block_until", "pz_block_until", "kill_block_until"] {
        let sql = format!(
            "INSERT INTO game_character_pvp_state(character_id,world_id,{column}) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,clock_timestamp()+interval '60 seconds') ON CONFLICT(character_id) DO UPDATE SET {column}=EXCLUDED.{column}"
        );
        sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(member.as_bytes().as_slice())
            .bind(world.as_bytes().as_slice())
            .execute(&h.pool)
            .await?;
        let blocked = roundtrip(&owner, &mut admitted[1], &[Step::Leave]).await?;
        assert_eq!(blocked[0].disposition, PartyDisposition::LogoutBlocked);
        // Test fixture expiry only: production writers never lower a deadline.
        sqlx::query(
            "DELETE FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(member.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    }
    let result = roundtrip(&owner, &mut admitted[1], &[Step::Leave]).await?;
    assert_eq!(result[0].disposition, PartyDisposition::Ok);
    let rows: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_parties),(SELECT count(*) FROM game_party_members),(SELECT count(*) FROM game_world_party_receipts)").fetch_one(&h.pool).await?;
    assert_eq!(rows.0, 0);
    assert_eq!(rows.1, 0);
    assert!(rows.2 >= 12, "native command receipts retained");
    Ok(())
}
