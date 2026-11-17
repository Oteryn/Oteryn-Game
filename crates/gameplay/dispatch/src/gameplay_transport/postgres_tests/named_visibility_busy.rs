//! NBPROFILE: node boot disconnected every observer in range (`serve_end phase=visibility`)
//! while a peer joined: the named visibility read of the peer's uncached name ran out of its
//! holder-contention budget, and that transient miss ended the observer's session. A busy
//! holder now skips the tick like the appearance read; a hard failure still disconnects.
use super::appearance_change_postgres::observer_visibility_tests;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::native_gameplay::test_support::cosmetics_fixture::{
    activated_with_optional_cosmetics, cosmetics_test_packet,
};
use crate::foundation::{CharacterLease, ExactActorRef, MessageType, decode_wire_envelope};
use crate::gameplay_transport::appearance_change::observation::ObservationError;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmissionRefusal, AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAttempt,
    FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::world_spatial::{ChannelEntities, WorldSpatialObservation};
use crate::gameplay_transport::{COMMAND_HOLDER_WINDOW, ComposedFreshAdmission, item_fence_of};
use oteryn_protocol_oteryn::entity_names::CAPABILITY_WORLD_SPATIAL_ENTITIES_V2;
use std::cell::Cell;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};
use tokio::sync::Notify;

#[derive(Clone, Copy, PartialEq)]
enum Case {
    Busy,
    Hard,
}

#[test]
fn named_visibility_busy_name_read_postgres_keeps_observer_connected() -> TestResult {
    run_case(Case::Busy)
}

#[test]
fn named_visibility_hard_failure_postgres_still_disconnects() -> TestResult {
    run_case(Case::Hard)
}

fn run_case(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "named visibility test thread panicked")??;
    Ok(())
}

// The real named read (real holder, real durable name read) is served through
// serve_admitted; `hard` replaces one read with a non-transient failure.
struct NamedOwner<'o, 'a, 'f, 's> {
    owner: &'o ComposedFreshAdmission<'a, 'f, 's>,
    baseline: WorldSpatialObservation,
    peer: ExactActorRef,
    hard: Cell<bool>,
    released: Cell<bool>,
    busy: Cell<u32>,
    unavailable: Cell<u32>,
    named_after_release: Cell<u32>,
    failed: Notify,
}

impl FreshAdmissionAuthority for NamedOwner<'_, '_, '_, '_> {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(self.baseline)
    }

    async fn observe_visible_entities(&self, actor: ExactActorRef) -> Option<ChannelEntities> {
        self.owner.observe_visible_entities(actor).await
    }

    async fn observe_named_visible_entities(
        &self,
        actor: ExactActorRef,
    ) -> Result<ChannelEntities, ObservationError> {
        let result = if self.hard.replace(false) {
            Err(ObservationError::Unavailable)
        } else {
            self.owner.observe_named_visible_entities(actor).await
        };
        match &result {
            Ok(channel) => {
                let peer = channel
                    .entities
                    .iter()
                    .find(|entity| entity.identity == self.peer.placement_identity());
                if self.released.get() && peer.is_some_and(|peer| peer.display_name.is_some()) {
                    self.named_after_release
                        .set(self.named_after_release.get() + 1);
                }
            }
            Err(ObservationError::HolderBusy) => {
                self.busy.set(self.busy.get() + 1);
                self.failed.notify_one();
            }
            Err(ObservationError::Unavailable | ObservationError::RespawnPending) => {
                self.unavailable.set(self.unavailable.get() + 1);
                self.failed.notify_one();
            }
        }
        result
    }
}

fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        let tag = match case {
            Case::Busy => "named_visibility_busy",
            Case::Hard => "named_visibility_hard",
        };
        let h = Harness::create_appearance(admin, tag).await?;
        prepare_current_session(&h).await?;
        h.seed_appearance_observer().await?;
        observer_visibility_tests::admit(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let mut packet = cosmetics_test_packet();
        packet["authoring"]["document"]["content_revision"] = "content-1".into();
        let (controller, pin) =
            activated_with_optional_cosmetics(world, None, Some(&serde_json::to_vec(&packet)?));
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let deps = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    actor,
                    session,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        deps.initialize(&owner).await;
        // The second player joins; nothing has cached its chat name.
        let peer = observer_visibility_tests::initialize(&deps, &owner).await?;
        let peer_session = crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?;
        assert!(owner.states.chat.speaker_name(peer_session).is_none());

        // The named read itself classifies an exhausted holder budget as transient.
        if case == Case::Busy {
            let held = owner.root.try_issue_semantic_pass().map_err(debug)?;
            let held_at = tokio::time::Instant::now();
            let busy = owner.named_channel_entities(actor).await;
            assert!(held_at.elapsed() >= COMMAND_HOLDER_WINDOW / 2);
            assert_eq!(busy.err(), Some(ObservationError::HolderBusy));
            drop(held);
            let named = owner
                .named_channel_entities(actor)
                .await
                .map_err(|error| format!("named read after the holder returned: {error:?}"))?;
            let entry = named
                .entities
                .iter()
                .find(|entity| entity.identity == peer.placement_identity())
                .ok_or("peer not visible")?;
            assert!(entry.display_name.is_some());
            assert!(owner.states.chat.speaker_name(peer_session).is_none());
        }

        let (current, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = item_fence_of(current);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[6, CAPABILITY_WORLD_SPATIAL_ENTITIES_V2],
        )
        .ok_or("named visibility capabilities")?;
        let named = NamedOwner {
            owner: &owner,
            baseline: owner.observe(actor).await.ok_or("baseline")?,
            peer,
            hard: Cell::new(false),
            released: Cell::new(false),
            busy: Cell::new(0),
            unavailable: Cell::new(0),
            named_after_release: Cell::new(0),
            failed: Notify::new(),
        };

        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let serving = async {
            let end = serve_admitted(&mut server, admitted, &named, IDLE_LIVENESS).await;
            drop(server);
            end
        };
        let (peer_named, peer_owner) = (&named, &owner);
        let observer = async move {
            let (named, owner) = (peer_named, peer_owner);
            read_snapshot(&mut client).await?;
            match case {
                Case::Busy => {
                    // Hold the real holder past the whole foreground budget of one tick.
                    let held = owner.root.try_issue_semantic_pass().map_err(debug)?;
                    named.failed.notified().await;
                    assert!(named.busy.get() > 0);
                    drop(held);
                    named.released.set(true);
                    // The observer is still served, and a later tick names the peer.
                    let (mut probes, mut sequence) = (0, 0);
                    while probes == 0 || named.named_after_release.get() == 0 {
                        let bytes = read_frame(&mut client)
                            .await
                            .map_err(|error| format!("observer disconnected: {error}"))?;
                        let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
                        match envelope.message_type() {
                            MessageType::LivenessProbe => {
                                acknowledge(&mut client, envelope.payload(), sequence).await?;
                                probes += 1;
                            }
                            MessageType::StateDelta => sequence = envelope.server_sequence(),
                            _ => {}
                        }
                    }
                }
                Case::Hard => {
                    named.hard.set(true);
                    named.failed.notified().await;
                    assert!(
                        read_frame(&mut client).await.is_err(),
                        "hard named visibility failure kept serving"
                    );
                }
            }
            client.shutdown().await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        };
        let (end, observed) = tokio::time::timeout(
            IDLE_LIVENESS.interval * 4,
            super::fixture::join_two(serving, observer),
        )
        .await?;
        observed?;
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
        match case {
            Case::Busy => {
                assert!(named.busy.get() > 0);
                assert_eq!(named.unavailable.get(), 0);
                assert!(named.named_after_release.get() > 0);
            }
            Case::Hard => {
                assert_eq!(named.busy.get(), 0);
                assert_eq!(named.unavailable.get(), 1);
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn read_snapshot(client: &mut DuplexStream) -> TestResult {
    loop {
        let bytes = read_frame(client).await?;
        let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
        if envelope.message_type() == MessageType::SnapshotCommit {
            return Ok(());
        }
    }
}

async fn read_frame(client: &mut DuplexStream) -> TestResult<Vec<u8>> {
    let length = usize::try_from(client.read_u32().await?)?;
    if length > 1 << 20 {
        return Err("oversized test frame".into());
    }
    let mut bytes = vec![0; length];
    client.read_exact(&mut bytes).await?;
    Ok(bytes)
}

async fn acknowledge(client: &mut DuplexStream, payload: &[u8], sequence: u64) -> TestResult {
    let probe = oteryn_protocol_oteryn::decode_liveness_probe(payload).map_err(debug)?;
    let ack = oteryn_protocol_oteryn::encode_liveness_ack(1, probe, sequence).map_err(debug)?;
    client.write_u32(u32::try_from(ack.len())?).await?;
    client.write_all(&ack).await?;
    Ok(())
}
