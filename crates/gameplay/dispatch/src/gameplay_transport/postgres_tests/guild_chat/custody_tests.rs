use super::*;

pub(super) async fn accept_before_exclude(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    leader: CurrentCharacterItemFence,
) -> TestResult {
    let (mut server, mut client) = tokio::io::duplex(64);
    let serving = async { serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await };
    let driving = async {
        wire_tests::member_login(&mut client).await?;
        owner
            .states
            .chat
            .deliver(&payload(gid(90)?, &"x".repeat(255))?);
        // A frame prefix proves connection-owner acceptance; 64-byte socket capacity
        // blocks the remaining 255-character line. Exclusion must commit before we drain it.
        let length = client.read_u32().await?;
        let exclusion = request(
            1,
            gid(90)?,
            leader,
            GuildCommand::Exclude(admitted.item_fence.ok_or("fence")?.character_id),
        )?;
        let excluded = tokio::time::timeout(
            Duration::from_secs(5),
            h.root
                .commit_guild_membership(owner.character, &h.node, leader, exclusion),
        )
        .await?
        .map_err(debug)?;
        assert!(
            matches!(excluded,GuildOutcome::Committed(receipt) if receipt.result==GuildResult::Ok)
        );
        let mut frame = vec![0; usize::try_from(length)?];
        client.read_exact(&mut frame).await?;
        let env = decode_wire_envelope(&frame).map_err(debug)?;
        assert_eq!(env.message_type(), MessageType::StateDelta);
        let delta = decode_state_delta(env.payload()).map_err(debug)?;
        assert!(
            matches!(chat::decode_chat_line(delta.payload).map_err(debug)?,ChatLine::Guild(line) if line.text.len()==255)
        );
        // A subsequent line still has the old payload guild, and is refused at final dequeue.
        owner.states.chat.deliver(&payload(gid(90)?, "excluded")?);
        assert!(owner.chat_next_bound(admitted, &mut 64).await.is_none());
        send_not_allowed(&mut client).await?;
        client.shutdown().await?;
        Ok(())
    };
    let (_, result): (_, TestResult) =
        tokio::time::timeout(Duration::from_secs(20), fixture::join_two(serving, driving)).await?;
    result?;
    assert_eq!(
        owner.chat_guild_admit(admitted).await,
        GuildRoomState::Absent
    );
    Ok(())
}

pub(super) async fn send_not_allowed(client: &mut tokio::io::DuplexStream) -> TestResult {
    wire_tests::send(client, 1, "no longer a member").await?;
    loop {
        let frame = read_frame(client).await?;
        let env = decode_wire_envelope(&frame).map_err(debug)?;
        assert_ne!(env.message_type(), MessageType::StateDelta);
        if env.message_type() == MessageType::CommandResult {
            let result = decode_command_result(env.payload()).map_err(debug)?;
            assert_eq!(result.status, CommandStatus::Accepted);
            assert_eq!(
                chat::decode_chat_intent_result(result.payload)
                    .map_err(debug)?
                    .disposition,
                ChatDisposition::NotAllowed
            );
            break;
        }
    }
    Ok(())
}
