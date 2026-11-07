use super::Case;
use crate::bestiary_postgres_harness::{CHANNEL, Harness, TestResult, WORLD, debug, id};
use crate::foundation::{ChannelId, ChannelRuntimeV1, MessageType, WorldId, decode_wire_envelope};
use crate::gameplay_transport::{
    capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities},
    connection::{
        AdmittedSession, ConnectionEnd, ControllerBinding, FirstEntryOutcome, IDLE_LIVENESS,
        SessionContinuity, serve_admitted,
    },
    last_position::AdmissionPosition,
    tcp_tls::{read_frame, write_frame},
};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
    team_finder_members::{self as wire, TeamFinderQuery, TeamFinderResult},
};
use tokio::{
    io::{AsyncWriteExt, DuplexStream},
    sync::Mutex,
};

pub(super) struct Peer {
    stream: DuplexStream,
    pub(super) admitted: AdmittedSession,
    next: u64,
}

impl Peer {
    pub(super) fn new(stream: DuplexStream, admitted: AdmittedSession) -> Self {
        Self {
            stream,
            admitted,
            next: admitted.continuity.next_command_id,
        }
    }

    pub(super) async fn snapshot(&mut self) -> TestResult {
        loop {
            let frame = read_frame(&mut self.stream).await.map_err(|error| {
                format!(
                    "session {:?}, initial snapshot read: {error:?}",
                    self.admitted.game_session_id
                )
            })?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                return Ok(());
            }
        }
    }

    pub(super) async fn command(
        &mut self,
        command_type: u32,
        payload: &[u8],
    ) -> TestResult<(u64, CommandStatus, Vec<u8>)> {
        let id = self.next;
        self.next = self.next.checked_add(1).ok_or("test command id overflow")?;
        let (status, bytes) = self.replay(id, command_type, payload).await?;
        Ok((id, status, bytes))
    }

    pub(super) async fn replay(
        &mut self,
        id: u64,
        command_type: u32,
        payload: &[u8],
    ) -> TestResult<(CommandStatus, Vec<u8>)> {
        write_frame(
            &mut self.stream,
            &encode_client_command(
                self.admitted.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: id,
                    command_type,
                    payload,
                },
            )
            .map_err(debug)?,
        )
        .await
        .map_err(|error| {
            format!(
                "session {:?}, command {command_type}/{id}, request write: {error:?}",
                self.admitted.game_session_id
            )
        })?;
        loop {
            let frame = read_frame(&mut self.stream).await.map_err(|error| {
                format!(
                    "session {:?}, command {command_type}/{id}, result read: {error:?}",
                    self.admitted.game_session_id
                )
            })?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, id);
                return Ok((result.status, result.payload.to_vec()));
            }
        }
    }

    pub(super) async fn action(
        &mut self,
        command_type: u32,
        payload: &[u8],
        expected: CommandStatus,
    ) -> TestResult<u64> {
        let (id, status, bytes) = self.command(command_type, payload).await?;
        assert_eq!(status, expected, "command {command_type}, id {id}");
        assert!(bytes.is_empty());
        Ok(id)
    }

    pub(super) async fn query(&mut self, query: TeamFinderQuery) -> TestResult<TeamFinderResult> {
        let (_, status, payload) = self
            .command(
                wire::COMMAND_TYPE_TEAM_FINDER_QUERY,
                &wire::encode_query(query).map_err(debug)?,
            )
            .await?;
        assert_eq!(status, CommandStatus::Accepted);
        Ok(wire::decode_result(&payload).map_err(debug)?)
    }

    pub(super) async fn shutdown(&mut self) -> TestResult {
        self.stream.shutdown().await?;
        Ok(())
    }
}

pub(super) async fn scenario(h: &Harness, case: Case) -> TestResult {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let identities = [(60, 63, 70), (61, 64, 71), (62, 65, 72)];
    let mut currents = Vec::new();
    for (character, account, session) in identities {
        currents.push(
            crate::world_party_runtime_postgres_cases::committed_fresh_player(
                h, character, account, session, now,
            )
            .await?,
        );
    }
    let characters: Vec<_> = currents
        .iter()
        .map(|current| *current.current_character_lease().character_id().as_bytes())
        .collect();
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
    for (index, current) in currents.iter().enumerate() {
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
            controller: Some(ControllerBinding {
                transport: current.current_transport().ok_or("fresh controller")?,
                account_id: id(identities[index].1),
            }),
            continuity: SessionContinuity::FRESH,
            item_fence: crate::gameplay_transport::item_fence_of(*current),
        });
    }
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
    let descriptor = crate::native_admission_source::descriptor::ProducerDescriptor::new(
        "platform-test".into(), ("127.0.0.1".into(), 1), "source.test".into(), "source.test".into(),
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/ca.der").to_vec())],
        vec![rustls::pki_types::CertificateDer::from(include_bytes!("../../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec())],
        rustls::pki_types::PrivateKeyDer::try_from(include_bytes!("../../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der").to_vec())?,
    ).map_err(debug)?;
    let evidence = crate::gameplay_transport::fresh_evidence::FreshEvidenceSource::new(descriptor);
    let spells = crate::spell::cast::v1_spell_book().map_err(debug)?;
    let achievements =
        crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?;
    let charms =
        crate::content::charm_source::CanonicalCharmCatalogue::embedded().map_err(debug)?;
    let runtime = Mutex::new(runtime);
    // full_world_e2e uses this qualified generation to align the native training
    // formula with the bootstrap fixture's durable content-1 interpretation.
    let (controller, _pin) =
        super::super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    let active = controller.active().ok_or("active native content")?;
    let mut owner = crate::gameplay_transport::composed_test_owner::ComposedTestOwner {
        root: &h.root, character: &recovery, holder: &h.node, evidence: &evidence,
        world_id: world, channel_id: channel,
        required_capabilities: crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
            &[], currents[0].current_runtime_scope(), 1, "policy-1",
            crate::gameplay_transport::world_required_capabilities::offered_for_scope(false, None),
        ).map_err(debug)?,
        runtime: &runtime, door: &door, spells: &spells, achievements: &achievements,
        imported_charms: &charms, room: &room,
    }.build();
    owner.active_generation = Some(active);
    for session in &mut admitted {
        session.continuity.selected_capabilities = SelectedCapabilities::select(
            // As in the existing S14a fixture, the synthetic entry room has no
            // bundled map. Inject the actual bundle offer into admitted sessions.
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[6, 7, 21, 42, 200, 232],
        )
        .filter(|selected| selected.contains(wire::CAPABILITY_TEAM_FINDER_MEMBERS_V1))
        .ok_or("bundle-world Team Finder offer")?;
        // Native admission initializes the source progression policy and its
        // displayed level; synthetic revisions cannot qualify a real logout.
        owner.admit_progression_session(session).await;
        owner.admit_loop_training(session).await;
        assert_eq!(session.first_entry, FirstEntryOutcome::Positioned);
        assert!(
            owner.progression_binding(session.game_session_id).is_some(),
            "native progression binding"
        );
        let fence = owner
            .current_quest_fence(session.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("initialized gameplay fence")?;
        assert!(owner.track_restored(
            session.game_session_id,
            session.runtime_actor.ok_or("actor")?,
            &AdmissionPosition::FirstLogin,
            fence,
        ));
    }
    let (mut server0, client0) = tokio::io::duplex(65_536);
    let (mut server1, client1) = tokio::io::duplex(65_536);
    let (mut server2, client2) = tokio::io::duplex(65_536);
    let owner_ref = &owner;
    let (first, second, third) = (admitted[0], admitted[1], admitted[2]);
    let serve0 = async move { serve_admitted(&mut server0, first, owner_ref, IDLE_LIVENESS).await };
    let serve1 =
        async move { serve_admitted(&mut server1, second, owner_ref, IDLE_LIVENESS).await };
    let serve2 = async move { serve_admitted(&mut server2, third, owner_ref, IDLE_LIVENESS).await };
    let peers = [
        Peer::new(client0, admitted[0]),
        Peer::new(client1, admitted[1]),
        Peer::new(client2, admitted[2]),
    ];
    let serving =
        super::super::fixture::join_two(super::super::fixture::join_two(serve0, serve1), serve2);
    let (ends, result) = tokio::time::timeout(
        std::time::Duration::from_secs(60),
        super::super::fixture::join_two(
            serving,
            super::cases::drive(h, &owner, &characters, case, peers),
        ),
    )
    .await
    .map_err(|error| format!("case {case:?}, initial connections deadline: {error}"))?;
    result.map_err(|error| format!("case {case:?}, driver: {error}; connection ends: {ends:?}"))?;
    let ((end0, end1), end2) = ends;
    if case == Case::Logout {
        assert!(
            matches!(end0, ConnectionEnd::AdmittedThenLoggedOut(_)),
            "{end0:?}"
        );
    } else {
        assert!(
            matches!(end0, ConnectionEnd::AdmittedThenDisconnected(_)),
            "{end0:?}"
        );
    }
    assert!(
        matches!(end1, ConnectionEnd::AdmittedThenDisconnected(_)),
        "{end1:?}"
    );
    assert!(
        matches!(end2, ConnectionEnd::AdmittedThenDisconnected(_)),
        "{end2:?}"
    );
    if case == Case::Logout {
        eprintln!("S14BC tf_logout stage=successor_admission");
        super::relog::check(h, &owner, admitted[0], characters[0], id(identities[0].1))
            .await
            .map_err(|error| format!("case Logout, successor admission/relog: {error}"))?;
    }
    Ok(())
}
