//! P22: Cyclopedia tracker commands 383-384 through admitted connection bytes, the domain 98
//! join snapshot from the durable lists, and the live domain 98 refresh after a real rat kill
//! and a Bosstiary kill settle through `drain_kill_rewards`. A rebuilt owner (fresh Channel
//! state, as after a restart) restores both lists from PostgreSQL alone through the join
//! snapshot's authority method.
//! The setup is the Task Board end-to-end fixture's (production generation, Thalom open cells).
#![allow(clippy::expect_used)]

use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::cyclopedia_tracker::{
    COMMAND_TYPE_BESTIARY_TRACKER as BESTIARY, COMMAND_TYPE_BOSSTIARY_TRACKER as BOSSTIARY,
    DELTA_TYPE_CYCLOPEDIA_TRACKER_V1, SNAPSHOT_TYPE_CYCLOPEDIA_TRACKER_V1,
    STATE_DOMAIN_CYCLOPEDIA_TRACKER, TrackerAction, TrackerRequest, TrackerSnapshot,
    decode_domain_snapshot, decode_snapshot, encode_request,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::time::Duration;
use tokio::io::{AsyncWriteExt, DuplexStream};
use tokio::sync::Mutex;

use super::fixture::{self, Harness, debug, id, run};
use super::{TestResult, prepare_current_session};
use crate::foundation::{
    ChannelId, ChannelRuntimeV1, CommandStatus, GameSessionId, MessageType, MovementLocalPosition,
    NodeId, WorldId, decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, ControllerBinding, FirstEntryOutcome, FreshAdmissionAuthority,
    IDLE_LIVENESS, SessionContinuity, serve_admitted,
};
use crate::gameplay_transport::kill_reward::DrainOutcome;
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, character_gameplay_fence};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;

const OPEN_PLAYER: MovementLocalPosition = MovementLocalPosition {
    x: 2,
    y: 11,
    floor: -7,
};
const OPEN_RAT: MovementLocalPosition = MovementLocalPosition {
    x: 1,
    y: 11,
    floor: -7,
};
const PASS: Duration = Duration::from_secs(3);
const RAT: &str = "oteryn:creature.rat";
const BOSS: &str = "oteryn:creature.ferumbras";

#[test]
fn cyclopedia_tracker_postgres_dispatch_kill_refresh_and_restart() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("cyclopedia-tracker", scenario)
}

fn change(action: TrackerAction, key: &str) -> Vec<u8> {
    encode_request(&TrackerRequest::Change {
        action,
        creature_key: key.to_owned(),
    })
    .expect("request")
}

/// The client side of one admitted connection.
struct Client {
    stream: DuplexStream,
    generation: u64,
    next_id: u64,
    /// Domain 98 deltas read while waiting for something else.
    deltas: Vec<(u64, TrackerSnapshot)>,
}

impl Client {
    /// Reads the join snapshot up to its commit; returns the domain 98 trackers.
    async fn join(&mut self) -> TestResult<Option<(TrackerSnapshot, TrackerSnapshot)>> {
        let mut trackers = None;
        loop {
            let frame = read_frame(&mut self.stream).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            match envelope.message_type() {
                MessageType::SnapshotCommit => return Ok(trackers),
                MessageType::SnapshotChunk => {
                    let (_, domains) =
                        oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())
                            .map_err(debug)?;
                    for domain in domains {
                        if domain.domain_id == STATE_DOMAIN_CYCLOPEDIA_TRACKER {
                            assert_eq!(domain.snapshot_type, SNAPSHOT_TYPE_CYCLOPEDIA_TRACKER_V1);
                            assert_eq!(domain.revision, 1);
                            trackers = Some(
                                decode_domain_snapshot(domain.payload).ok_or("domain snapshot")?,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Records a domain 98 delta frame; `true` when `frame` was one.
    fn delta(&mut self, frame: &[u8]) -> TestResult<bool> {
        let envelope = decode_wire_envelope(frame).map_err(debug)?;
        if envelope.message_type() != MessageType::StateDelta {
            return Ok(false);
        }
        let delta =
            oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
        if delta.domain_id != STATE_DOMAIN_CYCLOPEDIA_TRACKER {
            return Ok(false);
        }
        assert_eq!(delta.delta_type, DELTA_TYPE_CYCLOPEDIA_TRACKER_V1);
        assert_eq!(delta.new_revision, delta.base_revision + 1);
        let tracker = decode_snapshot(delta.payload).ok_or("tracker delta")?;
        self.deltas.push((delta.new_revision, tracker));
        Ok(true)
    }

    async fn command(
        &mut self,
        command_type: u32,
        payload: &[u8],
    ) -> TestResult<(CommandStatus, Vec<u8>)> {
        let command_id = self.next_id;
        self.next_id += 1;
        let bytes = oteryn_protocol_oteryn::encode_client_command(
            self.generation,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type,
                payload,
            },
        )
        .map_err(debug)?;
        write_frame(&mut self.stream, &bytes).await?;
        loop {
            let frame = read_frame(&mut self.stream).await?;
            if self.delta(&frame)? {
                continue;
            }
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let value = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                    .map_err(debug)?;
                assert_eq!(value.command_id, command_id);
                return Ok((value.status, value.payload.to_vec()));
            }
        }
    }

    /// The tracker of an accepted command.
    async fn accepted(&mut self, command_type: u32, payload: &[u8]) -> TestResult<TrackerSnapshot> {
        let (status, bytes) = self.command(command_type, payload).await?;
        assert_eq!(
            status,
            CommandStatus::Accepted,
            "{command_type} {payload:?}"
        );
        Ok(decode_snapshot(&bytes).ok_or("tracker result")?)
    }

    async fn rejected(&mut self, command_type: u32, payload: &[u8]) -> TestResult {
        assert_eq!(
            self.command(command_type, payload).await?,
            (CommandStatus::Rejected, Vec::new()),
            "{command_type} {payload:?}"
        );
        Ok(())
    }

    /// The next domain 98 delta, waiting on the server's visibility cadence.
    async fn next_delta(&mut self) -> TestResult<(u64, TrackerSnapshot)> {
        while self.deltas.is_empty() {
            let frame = read_frame(&mut self.stream).await?;
            self.delta(&frame)?;
        }
        Ok(self.deltas.remove(0))
    }
}

/// Serves `admitted` on `owner` while `drive` runs the client; keeps the continuity the
/// connection ended with.
async fn connect<F>(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    drive: impl FnOnce(Client) -> F,
) -> TestResult
where
    F: std::future::Future<Output = TestResult<Client>>,
{
    let before = *admitted;
    let (mut server, stream) = tokio::io::duplex(1 << 20);
    let serving = async move {
        let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async {
        let mut client = drive(Client {
            stream,
            generation: before.continuity.connection_generation,
            next_id: before.continuity.next_command_id,
            deltas: Vec::new(),
        })
        .await?;
        assert!(
            client.deltas.is_empty(),
            "unexpected refresh {:?}",
            client.deltas
        );
        client.stream.shutdown().await?;
        TestResult::Ok(())
    };
    let (ended, driven) =
        tokio::time::timeout(Duration::from_secs(60), fixture::join_two(serving, driving)).await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &ended
    {
        *admitted = *current;
    }
    driven.map_err(|error| format!("connection ended {ended:?}: {error}").into())
}

fn keys(tracker: &TrackerSnapshot) -> Vec<(&str, u32)> {
    tracker
        .entries
        .iter()
        .map(|entry| (entry.creature_key.as_str(), entry.kill_count))
        .collect()
}

#[allow(clippy::too_many_lines)]
fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "cyclopedia_tracker", false, false).await?;
        eprintln!(
            "POSTGRES_STARTED P22 cyclopedia_tracker_postgres_dispatch_kill_refresh_and_restart"
        );
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let session = GameSessionId::decode(&id(50)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                world,
                Some(include_bytes!(
                    "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
                )),
            );
        let active = controller.active().expect("activated generation");
        let native = active.native_gameplay().expect("native gameplay state");
        let room = pin.qualified_room().clone();
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            room.map_revision_digest(),
            (OPEN_PLAYER.x, OPEN_PLAYER.y, OPEN_PLAYER.floor),
        );
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3::text::numeric(20,0), NULL, $4, $5, $6)")
            .bind(id(42).as_slice())
            .bind(id(43).as_slice())
            .bind(pin.activation_sequence().to_string())
            .bind(pin.identity().server_artifact_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice())
            .bind(pin.frame_binding().digest().as_slice())
            .execute(&harness.pool)
            .await?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            // The player, the rat and the two boss deaths.
            4,
            channel_pin.clone(),
        )
        .map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        runtime
            .realize_native_qualification_spawn(channel_pin, RAT, OPEN_RAT)
            .map_err(debug)?;
        let rat = {
            let creature = runtime
                .visible_entities()
                .creatures
                .first()
                .copied()
                .expect("the rat is visible");
            EntityRef {
                identity: creature.actor.placement_identity(),
                generation: creature.generation,
            }
        };
        let door_content =
            crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
                .map_err(debug)?;
        let fence = super::fixture::fence(1)?;
        let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_test(
            fence.runtime_scope,
            fence.scope_ownership_generation,
            door_content,
        );
        let door = crate::world_runtime::bind_native_entry_door(
            room.door(),
            &door_fence,
            fence.runtime_scope,
            fence.scope_ownership_generation,
        )
        .map_err(debug)?;
        let descriptor = crate::native_admission_source::descriptor::ProducerDescriptor::new(
            "platform-test".into(),
            ("127.0.0.1".into(), 1),
            "source.test".into(),
            "source.test".into(),
            vec![rustls::pki_types::CertificateDer::from(
                include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/ca.der").to_vec(),
            )],
            vec![rustls::pki_types::CertificateDer::from(
                include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec(),
            )],
            rustls::pki_types::PrivateKeyDer::try_from(
                include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der")
                    .to_vec(),
            )?,
        )
        .map_err(debug)?;
        let evidence =
            crate::gameplay_transport::fresh_evidence::FreshEvidenceSource::new(descriptor);
        let spells = crate::spell::cast::v1_spell_book().map_err(debug)?;
        let achievements =
            crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?;
        let charms =
            crate::content::charm_source::CanonicalCharmCatalogue::embedded().map_err(debug)?;
        let runtime = Mutex::new(runtime);
        let door = Mutex::new(door);
        let mut admitted = AdmittedSession {
            game_session_id: session,
            world_id: world,
            channel_id: channel,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::NotApplicable,
            controller: Some(ControllerBinding {
                transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[9; 16])
                    .map_err(debug)?,
                account_id: id(40),
            }),
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        };
        macro_rules! owner {
            () => {{
                let mut owner = crate::gameplay_transport::composed_test_owner::ComposedTestOwner {
                    root: &harness.root,
                    character: &authority,
                    holder: &harness.node,
                    evidence: &evidence,
                    world_id: world,
                    channel_id: channel,
                    required_capabilities: crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                        &[],
                        crate::foundation::RuntimeScopeRefV1::channel(world, channel),
                        1,
                        "policy-1",
                        crate::gameplay_transport::world_required_capabilities::offered_for_scope(false, None),
                    )
                    .expect("an empty declared set binds"),
                    runtime: &runtime,
                    door: &door,
                    spells: &spells,
                    achievements: &achievements,
                    imported_charms: &charms,
                    room: &room,
                }.build();
                owner.active_generation = Some(active);
                owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
            owner.clock_origin = std::time::Instant::now()
                .checked_sub(Duration::from_secs(60))
                .expect("a monotonic clock older than a minute");
            {
                let runtime = owner.runtime.lock().await;
                assert!(
                    owner
                        .states
                        .spell_states
                        .lock()
                        .await
                        .initialize(
                            &runtime,
                            actor,
                            session,
                            crate::spell::cast::CharacterCastFacts {
                                vocation: crate::spell::Vocation::Monk,
                                ..crate::gameplay_transport::actor_spell::tests::FACTS
                            },
                            (3, 7_000_000),
                            owner.owner_now(),
                        )
                        .is_some()
                );
                // A trained fist makes the first real swing lethal (as the Task Board case).
                let mut skills = [(10, 0); 7];
                skills[0] = (1_000, 0);
                let build = crate::durability::character_build::DurableBuildState::new(
                    "monk",
                    (crate::gameplay_transport::actor_spell::tests::FACTS.magic_level.try_into()?, 0),
                    skills,
                )
                .map_err(debug)?;
                assert!(owner.states.spell_states.lock().await.load_owned_build(
                    &runtime, actor, session, &build,
                ));
            }
            owner.admit_progression_session(&mut admitted).await;
            assert!(owner.progression_binding(session).is_some());
                owner
            }};
        }
        let mut owner = owner!();
        let current = character_gameplay_fence::current(&owner, session, false)
            .await
            .ok_or("current fence")?;
        admitted.item_fence = Some(current.item_fence());
        admitted.first_entry = FirstEntryOutcome::Positioned;
        assert!(
            owner.reward_table().row(RAT).map_err(debug)?.race.is_some(),
            "the rat is a Bestiary race"
        );
        let boss = owner
            .reward_table()
            .boss(BOSS)
            .cloned()
            .ok_or("ferumbras is a boss")?;
        assert_eq!(boss.key, BOSS);

        // Without capability 197 neither command is a tracker command and no domain 98 is sent.
        connect(&owner, &mut admitted, async |mut client| {
            assert_eq!(client.join().await?, None);
            for command in [BESTIARY, BOSSTIARY] {
                let (status, _) = client.command(command, &[]).await?;
                assert_ne!(status, CommandStatus::Accepted);
            }
            Ok(client)
        })
        .await?;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            // Capability 6 runs the visibility cadence that carries the live refresh.
            &[
                oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES,
                oteryn_protocol_oteryn::cyclopedia_tracker::CAPABILITY_CYCLOPEDIA_TRACKER_V1,
            ],
        )
        .ok_or("selection")?;

        // P22a/P22b: add, refuse, remove through the real command stream.
        connect(&owner, &mut admitted, async |mut client| {
            let (bestiary, bosstiary) = client.join().await?.ok_or("domain 98 snapshot")?;
            assert!(!bestiary.boss && bestiary.entries.is_empty());
            assert!(bosstiary.boss && bosstiary.entries.is_empty());
            let added = client
                .accepted(BESTIARY, &change(TrackerAction::Add, RAT))
                .await?;
            assert!(!added.boss);
            assert_eq!(keys(&added), [(RAT, 0)]);
            let race = owner
                .reward_table()
                .row(RAT)
                .map_err(debug)?
                .race
                .clone()
                .ok_or("race")?;
            assert_eq!(
                added.entries[0].thresholds.map(u64::from).to_vec(),
                race.kill_thresholds()
                    .iter()
                    .map(|&t| u64::from(t))
                    .collect::<Vec<_>>()
            );
            assert!(!added.entries[0].completed);
            // Adding again changes nothing.
            assert_eq!(
                client
                    .accepted(BESTIARY, &change(TrackerAction::Add, RAT))
                    .await?,
                added
            );
            // Not a Bestiary race of the loaded content; a boss on the Bestiary command.
            client
                .rejected(
                    BESTIARY,
                    &change(TrackerAction::Add, "oteryn:creature.nosuch"),
                )
                .await?;
            client
                .rejected(BOSSTIARY, &change(TrackerAction::Add, RAT))
                .await?;
            // Crystal protocolgame.cpp:2790: no boss change before its first credited kill.
            client
                .rejected(BOSSTIARY, &change(TrackerAction::Add, BOSS))
                .await?;
            client
                .rejected(BOSSTIARY, &change(TrackerAction::Remove, BOSS))
                .await?;
            client.rejected(BESTIARY, &[2, b'x']).await?;
            assert!(client.accepted(BOSSTIARY, &[]).await?.entries.is_empty());
            assert_eq!(client.accepted(BESTIARY, &[]).await?, added);
            Ok(client)
        })
        .await?;
        let rows: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_cyclopedia_tracker")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(rows, 1);

        // P22d: a real rat kill settled by `drain_kill_rewards` while disconnected; the next
        // connection's snapshot carries it and the visibility cadence pushes the refresh.
        {
            let runtime = owner.runtime.lock().await;
            let now = owner.owner_now();
            assert_eq!(
                owner.states.attack.lock().await.set_target(
                    &runtime,
                    Some(&room),
                    actor,
                    session,
                    now,
                    Some(rat),
                    1
                ),
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
            );
        }
        let mut swings = 0;
        loop {
            owner.refresh_attack_weapon(actor, session).await;
            owner.drain_auto_attacks().await;
            if owner
                .states
                .attack
                .lock()
                .await
                .kills()
                .has_session(session)
            {
                break;
            }
            swings += 1;
            assert!(swings < 64, "the held rat dies to auto-attack swings");
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(PASS)
                .expect("a monotonic clock older than the passes");
        }
        let mut drains = 0;
        while owner.drain_kill_rewards(session).await != DrainOutcome::Drained {
            drains += 1;
            assert!(drains < 8, "the queued kill settles");
        }
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence read")?
            .ok_or("live session")?;
        // An untracked boss kill credits the boss and pushes nothing.
        let facts = {
            let mut runtime = owner.runtime.lock().await;
            super::bosstiary::death(&mut runtime, fence)?
        };
        super::bosstiary::settle(&owner, facts, &boss).await?;
        connect(&owner, &mut admitted, async |mut client| {
            let (bestiary, bosstiary) = client.join().await?.ok_or("domain 98 snapshot")?;
            assert_eq!(keys(&bestiary), [(RAT, 1)]);
            assert!(bosstiary.entries.is_empty());
            let (revision, refreshed) = client.next_delta().await?;
            assert_eq!(revision, 2);
            assert_eq!(refreshed, bestiary);
            // The credited kill admits a Bosstiary change.
            let tracked = client
                .accepted(BOSSTIARY, &change(TrackerAction::Add, BOSS))
                .await?;
            assert!(tracked.boss);
            assert_eq!(keys(&tracked), [(BOSS, 1)]);
            // Nemesis: prowess 1, expertise 3, mastery 5 (rulesets/progression/bosstiary).
            assert_eq!(tracked.entries[0].thresholds, [1, 3, 5]);
            // A Bosstiary kill while connected is pushed on the cadence.
            let facts = {
                let mut runtime = owner.runtime.lock().await;
                super::bosstiary::death(&mut runtime, fence)?
            };
            super::bosstiary::settle(&owner, facts, &boss).await?;
            let (revision, refreshed) = client.next_delta().await?;
            assert_eq!(revision, 3);
            assert!(refreshed.boss);
            assert_eq!(keys(&refreshed), [(BOSS, 2)]);
            Ok(client)
        })
        .await?;

        // P22c: a rebuilt owner holds no tracker state; the lists come from PostgreSQL.
        drop(owner);
        let reopened = owner!();
        // The join (`serve/snapshot.rs`) and the command route call these authority methods.
        let (bestiary, bosstiary) = decode_domain_snapshot(
            &reopened
                .cyclopedia_tracker_snapshot(admitted)
                .await
                .ok_or("domain 98 snapshot")?,
        )
        .ok_or("domain snapshot")?;
        assert_eq!(keys(&bestiary), [(RAT, 1)]);
        assert_eq!(keys(&bosstiary), [(BOSS, 2)]);
        for (command, key) in [(BOSSTIARY, BOSS), (BESTIARY, RAT)] {
            let removed = reopened
                .cyclopedia_tracker(
                    admitted,
                    command,
                    TrackerRequest::Change {
                        action: TrackerAction::Remove,
                        creature_key: key.to_owned(),
                    },
                )
                .await
                .ok_or("remove")?;
            assert!(removed.entries.is_empty());
        }
        let rows: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_cyclopedia_tracker")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(rows, 0);

        // The migration grants the runtime only what the owner uses.
        let access: (bool, bool, bool) = sqlx::query_as("SELECT has_table_privilege('oteryn_game_runtime','game_character_cyclopedia_tracker','INSERT'),has_table_privilege('oteryn_game_runtime','game_character_cyclopedia_tracker','DELETE'),has_table_privilege('oteryn_game_runtime','game_character_cyclopedia_tracker','UPDATE')")
            .fetch_one(&harness.pool)
            .await?;
        assert_eq!(access, (true, true, false));
        eprintln!(
            "CYCLOPEDIA TRACKER PostgreSQL: dispatch, kill gate, live refresh and restart verified"
        );
        drop(reopened);
        drop(authority);
        drop(seal);
        harness.cleanup().await?;
        Ok(())
    })
}
