//! Party chat through the real connection, Party owner, PostgreSQL relay and egress custody.
use super::party_membership;
use crate::bestiary_postgres_harness::{Harness, TestResult, WORLD, debug, id};
use crate::chat::relay::{
    RelayDestination, RelayKey, RelayKeyRing, RelayLine, ReplayWindow, channel_name,
};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::SelectedCapabilities,
    connection::{
        AdmittedSession, ConnectionEnd, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
    },
    tcp_tls::{read_frame, write_frame},
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus,
    chat::{self, ChatDisposition, ChatIntent, ChatLine},
    decode_command_result, encode_client_command,
};
use std::{sync::Arc, time::Duration};
use tokio::io::AsyncWriteExt;

#[test]
fn party_chat_postgres_real_wire_members_only_departure_refusal_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    party_membership::run_chat()
}

async fn command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    expected: ChatDisposition,
) -> TestResult {
    let session = *admitted;
    let (mut server, mut client) = tokio::io::duplex(16_384);
    let serving = async { serve_admitted(&mut server, session, owner, IDLE_LIVENESS).await };
    let driving = async {
        loop {
            let bytes = read_frame(&mut client).await?;
            if decode_wire_envelope(&bytes).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        let payload = chat::encode_chat_intent(&ChatIntent::Party {
            text: "party only".into(),
        })
        .map_err(debug)?;
        let frame = encode_client_command(
            session.continuity.connection_generation,
            &ClientCommandValue {
                command_id: session.continuity.next_command_id,
                command_type: chat::COMMAND_TYPE_CHAT_INTENT,
                payload: &payload,
            },
        )
        .map_err(debug)?;
        // An identical command is answered from the real retained ingress result.
        for status in [CommandStatus::Accepted, CommandStatus::DuplicateReplay] {
            write_frame(&mut client, &frame).await?;
            loop {
                let bytes = read_frame(&mut client).await?;
                let env = decode_wire_envelope(&bytes).map_err(debug)?;
                if env.message_type() != MessageType::CommandResult {
                    continue;
                }
                let result = decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(result.command_id, session.continuity.next_command_id);
                assert_eq!(result.status, status);
                assert_eq!(
                    chat::decode_chat_intent_result(result.payload)
                        .map_err(debug)?
                        .disposition,
                    expected
                );
                break;
            }
        }
        client.shutdown().await?;
        Ok(())
    };
    let (end, result): (_, TestResult) = tokio::time::timeout(
        Duration::from_secs(30),
        super::fixture::join_two(serving, driving),
    )
    .await?;
    match end {
        ConnectionEnd::AdmittedThenDisconnected(current) => *admitted = current,
        other => return Err(format!("party chat end: {other:?}").into()),
    }
    result
}

pub(super) async fn check(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut [AdmittedSession],
) -> TestResult<RelayLine> {
    eprintln!(
        "CHATCH-PARTY-PG: real Party membership, connection, encrypted relay and recipient custody"
    );
    let ring = RelayKeyRing::new(vec![RelayKey::new(1, 0, [1; 32])]).map_err(debug)?;
    let relay = crate::gameplay_transport::chat_relay::ChatRelay::new(
        ring.clone(),
        id(WORLD),
        h.root
            .chat_relay_connect_options()
            .ok_or("relay endpoint")?,
    )
    .map_err(debug)?;
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&h.pool)
            .await?;
    relay.force_ready(u64::try_from(now)?);
    owner.states.chat.install_relay(Arc::new(relay));
    let mut listener = sqlx::postgres::PgListener::connect_with(&h.pool).await?;
    let channel =
        channel_name(&crate::durability::guild_chat_fixture::uuid(id(WORLD))).map_err(debug)?;
    listener.listen(&channel).await?;
    for a in admitted.iter_mut() {
        // Reuse the native Party fixture's real admissions with the published bundle
        // offer set. Map loading is independent of chat custody and is not this case.
        a.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[6, 7, 42, 240],
        )
        .ok_or("party chat selection")?;
        assert!(a.continuity.selected_capabilities.contains(240));
        owner
            .chat_admit(a.runtime_actor.ok_or("actor")?, a.game_session_id)
            .await;
        owner.chat_party_admit(*a).await;
    }
    command(owner, &mut admitted[0], ChatDisposition::Ok).await?;
    let notification = tokio::time::timeout(Duration::from_secs(5), listener.recv()).await??;
    let mut receiver = ReplayWindow::starting_at(0);
    let receive_now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&h.pool)
            .await?;
    let line = receiver
        .receive(
            &ring,
            &channel,
            notification.payload(),
            u64::try_from(receive_now)?,
        )
        .map_err(debug)?;
    assert!(matches!(line.destination, RelayDestination::Party { .. }));
    // A query/NOTIFY barrier proves the replay emitted no second notification.
    sqlx::query("SELECT pg_notify($1,'barrier')")
        .bind(&channel)
        .execute(&h.pool)
        .await?;
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(5), listener.recv())
            .await??
            .payload(),
        "barrier"
    );
    // Membership at dequeue cannot disclose a line from before this membership began.
    let before_join = RelayLine {
        sent_at_micros: 1,
        ..line.clone()
    };
    owner.states.chat.deliver(&before_join);
    for a in admitted.iter() {
        assert!(owner.chat_next_bound(*a, &mut 64).await.is_none());
    }
    owner.states.chat.deliver(&line);
    for a in admitted.iter() {
        let received = owner
            .chat_next_bound(*a, &mut 64)
            .await
            .ok_or("member receives")?;
        assert!(matches!(received, ChatLine::Party { text, .. } if text == "party only"));
        owner.chat_delivered(a.game_session_id).await;
        assert!(owner.chat_next_bound(*a, &mut 64).await.is_none());
    }
    let mut stale = admitted[1];
    stale
        .item_fence
        .as_mut()
        .ok_or("fence")?
        .character_lease_generation += 1;
    owner.states.chat.deliver(&line);
    assert!(owner.chat_next_bound(stale, &mut 64).await.is_none());
    let mut unselected = admitted[2];
    unselected.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[6, 7]).ok_or("old client")?;
    assert!(owner.chat_next_bound(unselected, &mut 64).await.is_none());
    // Drain the sender's second queued copy before the leave scenario.
    assert!(owner.chat_next_bound(admitted[0], &mut 64).await.is_some());
    owner.chat_delivered(admitted[0].game_session_id).await;
    // A regular member can speak too; no leader, Premium, vocation or level gate.
    command(owner, &mut admitted[1], ChatDisposition::Ok).await?;
    let notification = tokio::time::timeout(Duration::from_secs(5), listener.recv()).await??;
    assert_ne!(notification.payload(), "barrier");
    Ok(line)
}

pub(super) async fn check_departed(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut [AdmittedSession],
    line: &RelayLine,
) -> TestResult {
    owner.states.chat.deliver(line);
    assert!(
        owner.chat_next_bound(admitted[0], &mut 64).await.is_none(),
        "departed member gets no queued Party line"
    );
    command(owner, &mut admitted[0], ChatDisposition::NotAllowed).await?;
    for a in admitted.iter().skip(1) {
        assert!(owner.chat_next_bound(*a, &mut 64).await.is_some());
        owner.chat_delivered(a.game_session_id).await;
    }
    Ok(())
}
