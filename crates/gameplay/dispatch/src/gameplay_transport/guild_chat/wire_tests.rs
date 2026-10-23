use super::*;

async fn login(client: &mut tokio::io::DuplexStream, extended: bool) -> TestResult<GuildRoomState> {
    let mut guild = None;
    loop {
        let frame = read_frame(client).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == MessageType::SnapshotChunk {
            let (_, domains) = decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
            for domain in domains
                .into_iter()
                .filter(|d| d.domain_id == chat::STATE_DOMAIN_CHAT)
            {
                if extended {
                    assert_eq!(
                        domain.snapshot_type,
                        guild_room::SNAPSHOT_TYPE_GUILD_CHAT_V1
                    );
                    guild = Some(
                        guild_room::decode_snapshot(domain.payload)
                            .map_err(debug)?
                            .guild,
                    );
                } else {
                    assert_eq!(domain.snapshot_type, chat::SNAPSHOT_TYPE_CHAT_V1);
                    assert_eq!(
                        chat::decode_chat_rooms(domain.payload).map_err(debug)?,
                        chat::ChatRoomSet::default()
                    );
                    guild = Some(GuildRoomState::Absent);
                }
            }
        }
        if envelope.message_type() == MessageType::SnapshotCommit {
            return guild.ok_or_else(|| "CHAT snapshot missing".into());
        }
    }
}

pub(super) async fn send(client: &mut tokio::io::DuplexStream, id: u64, text: &str) -> TestResult {
    let payload = chat::encode_chat_intent(&ChatIntent::Guild {
        text: text.to_owned(),
    })
    .map_err(debug)?;
    let frame = encode_client_command(
        1,
        &ClientCommandValue {
            command_id: id,
            command_type: chat::COMMAND_TYPE_CHAT_INTENT,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    write_frame(client, &frame).await?;
    Ok(())
}

pub(super) async fn roundtrip(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    listener: &mut sqlx::postgres::PgListener,
    ring: &RelayKeyRing,
    channel: &str,
) -> TestResult<RelayLine> {
    let (mut server, mut client) = tokio::io::duplex(64);
    let serving = async { serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await };
    let driving = async {
        assert_eq!(
            login(&mut client, true).await?,
            GuildRoomState::Member {
                guild_id: *gid(90)?.as_bytes(),
                rank: 3,
                motd: "Welcome members".into()
            }
        );
        send(&mut client, 1, "hello guild").await?;
        loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == MessageType::CommandResult {
                let result = decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(result.status, CommandStatus::Accepted);
                assert_eq!(
                    chat::decode_chat_intent_result(result.payload)
                        .map_err(debug)?
                        .disposition,
                    ChatDisposition::Ok
                );
                break;
            }
        }
        let notification = tokio::time::timeout(Duration::from_secs(5), listener.recv()).await??;
        let mut receiving = ReplayWindow::starting_at(0);
        let now: i64 = sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint",
        )
        .fetch_one(&h.pool)
        .await?;
        let line = receiving
            .receive(ring, channel, notification.payload(), u64::try_from(now)?)
            .map_err(debug)?;
        assert_eq!(line.sender_name.as_str(), "Fixture Hero");
        assert_eq!(
            line.destination,
            RelayDestination::Guild {
                guild_id: gid(90)?,
                rank: GuildRank::new(3).map_err(debug)?
            }
        );
        owner.chat.deliver(&line);
        loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == MessageType::StateDelta {
                let delta = decode_state_delta(env.payload()).map_err(debug)?;
                if delta.domain_id == chat::STATE_DOMAIN_CHAT {
                    let ChatLine::Guild(guild) =
                        chat::decode_chat_line(delta.payload).map_err(debug)?
                    else {
                        return Err("guild kind lost".into());
                    };
                    assert_eq!(guild.guild_id, *gid(90)?.as_bytes());
                    assert_eq!(guild.rank, 3);
                    assert_eq!(guild.speaker_name, "Fixture Hero");
                    assert_eq!(guild.text, "hello guild");
                    break;
                }
            }
        }
        client.shutdown().await?;
        Ok(line)
    };
    let (_, result): (_, TestResult<RelayLine>) =
        tokio::time::timeout(Duration::from_secs(20), fixture::join_two(serving, driving)).await?;
    let accepted = result?;
    let mut old = admitted;
    old.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[6, 7])
            .ok_or("old CHAT selection")?;
    let (mut server, mut client) = tokio::io::duplex(256);
    let serving = async { serve_admitted(&mut server, old, owner, IDLE_LIVENESS).await };
    let driving = async {
        login(&mut client, false).await?;
        owner
            .chat
            .deliver(&payload(gid(90)?, "unoffered peers receive none")?);
        send(&mut client, 1, "forged unnegotiated guild intent").await?;
        loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            assert_ne!(env.message_type(), MessageType::StateDelta);
            if env.message_type() == MessageType::CommandResult {
                let result = decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(result.status, CommandStatus::Rejected);
                assert!(result.payload.is_empty());
                break;
            }
        }
        client.shutdown().await?;
        Ok(())
    };
    let (_, result): (_, TestResult) =
        tokio::time::timeout(Duration::from_secs(20), fixture::join_two(serving, driving)).await?;
    result?;
    Ok(accepted)
}

pub(super) async fn member_login(client: &mut tokio::io::DuplexStream) -> TestResult {
    assert!(matches!(
        login(client, true).await?,
        GuildRoomState::Member { .. }
    ));
    Ok(())
}
