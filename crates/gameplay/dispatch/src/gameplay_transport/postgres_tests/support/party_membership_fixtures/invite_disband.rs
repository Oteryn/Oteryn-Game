use super::*;

pub(super) async fn check(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut [AdmittedSession],
    target: EntityRef,
    leader_id: [u8; 16],
) -> TestResult {
    select_notices(owner, admitted)?;
    let [leader, invitee, bystander] = admitted else {
        return Err("expected three live peers".into());
    };
    let (mut leader_server, mut a) = Peer::new(*leader);
    let (mut invitee_server, mut b) = Peer::new(*invitee);
    let (mut bystander_server, mut c) = Peer::new(*bystander);
    let driving = async move {
        a.ready().await?;
        b.ready().await?;
        c.ready().await?;

        a.command(Some(PartyIntent::Invite(target))).await?;
        let inviter_handle =
            crate::gameplay_transport::party_command::party_member_handle(b.session, leader_id)
                .ok_or("inviter handle")?;
        assert_eq!(b.notice(3, "invited").await?.member_handle, inviter_handle);
        a.check_quiet().await?;
        c.check_quiet().await?;

        let invitation = b.command(None).await?;
        let [invitation] = invitation.invitations.as_slice() else {
            return Err("expected one invitation in the party query".into());
        };
        b.command(Some(PartyIntent::Accept(invitation.handle)))
            .await?;
        a.notice(1, "joined").await?;
        b.notice(1, "joined").await?;

        b.command(Some(PartyIntent::Leave)).await?;
        a.notice(2, "left").await?;
        b.notice(2, "left").await?;
        a.notice(4, "disbanded").await?;
        assert_eq!(
            b.notice(4, "disbanded").await?.member_handle,
            inviter_handle
        );
        assert!(a.command(None).await?.members.is_empty());
        assert!(b.command(None).await?.members.is_empty());
        c.check_quiet().await?;

        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (leader_end, invitee_end, bystander_end, result) =
        tokio::time::timeout(Duration::from_secs(40), async {
            tokio::join!(
                serve_admitted(&mut leader_server, *leader, owner, notice_policy()),
                serve_admitted(&mut invitee_server, *invitee, owner, notice_policy()),
                serve_admitted(&mut bystander_server, *bystander, owner, notice_policy()),
                driving,
            )
        })
        .await?;
    result?;
    for end in [leader_end, invitee_end, bystander_end] {
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
    }
    Ok(())
}

struct Peer {
    stream: tokio::io::DuplexStream,
    session: AdmittedSession,
    next_command: u64,
    notices: std::collections::VecDeque<PartyMembershipNotice>,
}

impl Peer {
    fn new(session: AdmittedSession) -> (tokio::io::DuplexStream, Self) {
        let (server, stream) = tokio::io::duplex(16_384);
        (
            server,
            Self {
                stream,
                session,
                next_command: session.continuity.next_command_id,
                notices: Default::default(),
            },
        )
    }

    fn generation(&self) -> u64 {
        self.session.continuity.connection_generation
    }

    async fn ready(&mut self) -> TestResult {
        let generation = self.generation();
        loop {
            let frame = read_live_frame(&mut self.stream, generation).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                return Ok(());
            }
        }
    }

    async fn command(&mut self, intent: Option<PartyIntent>) -> TestResult<PartyResult> {
        let generation = self.generation();
        let (command_type, payload) = match intent {
            Some(intent) => (
                COMMAND_TYPE_PARTY_INTENT,
                encode_party_intent(intent).map_err(debug)?,
            ),
            None => (COMMAND_TYPE_PARTY_QUERY, Vec::new()),
        };
        let command_id = self.next_command;
        let frame = encode_client_command(
            generation,
            &ClientCommandValue {
                command_id,
                command_type,
                payload: &payload,
            },
        )
        .map_err(debug)?;
        write_frame(&mut self.stream, &frame).await?;
        self.next_command += 1;
        loop {
            let frame = read_live_frame(&mut self.stream, generation).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, command_id);
                assert_eq!(result.status, CommandStatus::Accepted);
                let party = decode_party_result(result.payload).map_err(debug)?;
                assert_eq!(party.disposition, PartyDisposition::Ok);
                return Ok(party);
            }
            self.notices.extend(party_notice(&envelope)?);
        }
    }

    async fn notice(&mut self, kind: u8, label: &str) -> TestResult<PartyMembershipNotice> {
        let generation = self.generation();
        let notice = if let Some(pending) = self.notices.pop_front() {
            pending
        } else {
            tokio::time::timeout(
                Duration::from_secs(5),
                read_notice(&mut self.stream, generation),
            )
            .await
            .map_err(|_| format!("missing {label} party notice"))??
        };
        assert_eq!(notice.change as u8, kind, "expected {label} party notice");
        Ok(notice)
    }

    async fn check_quiet(&mut self) -> TestResult {
        // The query response is a stream barrier after the mutation. The next two
        // probes bracket a complete party mailbox refresh beyond that barrier.
        self.command(None).await?;
        assert!(self.notices.is_empty(), "unexpected party notice");
        let mut probes = 0;
        while probes < 2 {
            let frame = read_frame(&mut self.stream).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            assert!(
                party_notice(&envelope)?.is_none(),
                "unexpected party notice"
            );
            if envelope.message_type() == MessageType::LivenessProbe {
                let probe = oteryn_protocol_oteryn::decode_liveness_probe(envelope.payload())
                    .map_err(debug)?;
                let ack = oteryn_protocol_oteryn::encode_liveness_ack(self.generation(), probe, 0)
                    .map_err(debug)?;
                write_frame(&mut self.stream, &ack).await?;
                probes += 1;
            }
        }
        Ok(())
    }
}

fn select_notices(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut [AdmittedSession],
) -> TestResult {
    let selected = SelectedCapabilities::select(
        owner.offered_capabilities(),
        &[6, 21, 42, 92, CAPABILITY_PARTY_MEMBERSHIP_NOTICES_V1],
    )
    .filter(|caps| caps.contains(CAPABILITY_PARTY_MEMBERSHIP_NOTICES_V1))
    .ok_or("party membership notice offer")?;
    for peer in &mut *admitted {
        peer.continuity.selected_capabilities = selected;
    }
    Ok(())
}

pub(super) async fn check_fanout(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut [AdmittedSession],
    target: EntityRef,
    [leader_id, member_id]: [[u8; 16]; 2],
    h: &Harness,
) -> TestResult {
    select_notices(owner, admitted)?;
    let [leader, member, ..] = admitted else {
        return Err("expected two live peers".into());
    };
    let (mut leader_server, mut a) = Peer::new(*leader);
    let (mut member_server, mut b) = Peer::new(*member);
    // No liveness probe may drive delivery during the scenario.
    let policy = crate::gameplay_transport::connection::LivenessPolicy {
        interval: Duration::from_secs(60),
        missed_limit: 3,
    };
    let driving = async move {
        a.ready().await?;
        b.ready().await?;
        a.command(Some(PartyIntent::Invite(target))).await?;
        let invitations = b.command(None).await?.invitations;
        let [invitation] = invitations.as_slice() else {
            return Err("expected one invitation".into());
        };
        b.command(Some(PartyIntent::Accept(invitation.handle))).await?;

        for (peer, character, change) in [
            (&mut a, member_id, PartyMembershipChange::Joined),
            (&mut b, leader_id, PartyMembershipChange::Invited),
        ] {
            assert_notice(peer, character, change).await?;
        }
        assert_notice(&mut b, member_id, PartyMembershipChange::Joined).await?;

        // Keep the actor's foreground turn through the leave result so the
        // test can acquire the holder before the background drains.
        let foreground = h.root.foreground_demand()?;
        b.command(Some(PartyIntent::Leave)).await?;
        let held = h.root.try_issue_semantic_pass()?;
        drop(foreground);
        wait_for_busy_drain(h, member_id).await?;
        drop(held);

        for peer in [&mut a, &mut b] {
            assert_notice(peer, member_id, PartyMembershipChange::Left).await?;
            assert_notice(peer, leader_id, PartyMembershipChange::Disbanded).await?;
            assert!(peer.command(None).await?.members.is_empty());
            assert!(peer.notices.is_empty(), "duplicate Party notice");
        }
        // Any second copy left in the mailboxes would be emitted on the next
        // delivery turn. Check the durable queues in addition to wire order.
        let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM game_party_membership_notices")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(remaining, 0);
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (result, leader_end, member_end) = tokio::time::timeout(Duration::from_secs(45), async {
        tokio::join!(
            // Once the leave result arrives, the driver can claim the holder
            // before polling either serving future for its next dispatch turn.
            biased;
            driving,
            serve_admitted(&mut leader_server, *leader, owner, policy),
            serve_admitted(&mut member_server, *member, owner, policy),
        )
    }).await?;
    result?;
    for end in [leader_end, member_end] {
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
    }
    Ok(())
}

async fn assert_notice(
    peer: &mut Peer,
    character: [u8; 16],
    change: PartyMembershipChange,
) -> TestResult {
    let handle = crate::gameplay_transport::party_command::party_member_handle(peer.session, character)
        .ok_or("party notice handle")?;
    let notice = peer.notice(change as u8, &format!("{change:?}")).await?;
    assert_eq!(notice.member_handle, handle, "{change:?} subject");
    Ok(())
}

async fn wait_for_busy_drain(h: &Harness, member_id: [u8; 16]) -> TestResult {
    let pending: Option<i32> = sqlx::query_scalar(
        "SELECT cardinality(events) FROM game_party_membership_notices
         WHERE character_id = encode($1, 'hex')::uuid",
    )
    .bind(member_id.as_slice())
    .fetch_optional(&h.pool)
    .await?;
    assert_eq!(pending, Some(2), "committed Left and Disbanded remain queued");
    tokio::time::timeout(Duration::from_secs(4), async {
        while !h.root.has_ready_demand() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| "Party delivery did not attempt checkout during the holder outage")?;
    Ok(())
}
