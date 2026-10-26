//! IDLEKICK: real router, composed logout, generation fence and PostgreSQL final position.
use super::autowalk::{at, place, read};
use super::fixture::{Harness, debug, id, join_two, run};
use super::{Dependencies, TestResult, prepare_current_session, track};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ChannelRuntimeV1, GameSessionState};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmissionRefusal, AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAttempt,
    FreshAdmissionAuthority, GameplayIdlePolicy, LivenessPolicy, LogoutReply, serve_admitted,
};
use crate::map::boot::{BootPins, boot};
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::{MessageType, decode_wire_envelope, encode_liveness_ack};
use std::cell::{Cell, RefCell};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::time::Instant;

struct ClockOwner<'a, 'f, 's> {
    owner: &'a ComposedFreshAdmission<'a, 'f, 's>,
    now: Cell<Instant>,
    replies: RefCell<Vec<LogoutReply>>,
    relocate_on_logout: Cell<bool>,
}

impl FreshAdmissionAuthority for ClockOwner<'_, '_, '_> {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }
    fn gameplay_idle_now(&self) -> Instant {
        self.now.get()
    }
    fn gameplay_idle_policy(&self) -> GameplayIdlePolicy {
        self.owner.gameplay_idle_policy()
    }
    async fn gameplay_idle_exemptions(&self, admitted: AdmittedSession) -> Option<(bool, bool)> {
        self.owner.gameplay_idle_exemptions(admitted).await
    }
    async fn observe(
        &self,
        actor: crate::foundation::ExactActorRef,
    ) -> Option<crate::gameplay_transport::world_spatial::WorldSpatialObservation> {
        self.owner.observe(actor).await
    }
    async fn logout(&self, admitted: AdmittedSession) -> LogoutReply {
        if self.relocate_on_logout.get() && place(self.owner, admitted, at(0, 0)).await.is_err() {
            return LogoutReply::Unknown;
        }
        let reply = self.owner.logout(admitted).await;
        self.replies.borrow_mut().push(reply);
        reply
    }
}

fn bundle() -> TestResult<(Vec<u8>, crate::map::BundlePins)> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    use oteryn_world_bundle::writer::{
        self, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector, Terrain,
        TerrainKind,
    };
    let manifest = Manifest {
        format: writer::FORMAT.into(),
        min_reader_version: writer::VERSION,
        projection_class: "server".into(),
        compiler_version: "engineering".into(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            ..Default::default()
        },
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 32,
            max_y: 32,
            floors: vec![-7],
        },
        palette: vec![PaletteEntry {
            key: "oteryn:terrain.tibia.i4526".into(),
            family: Family::Terrain,
            id: 0,
            terrain: Some(Terrain {
                kind: TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(150),
            }),
        }],
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
        subareas: Default::default(),
    };
    let bytes = writer::write(
        &manifest,
        &[Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles: (0..2)
                .map(|x| Tile {
                    x,
                    y: 0,
                    flags: if x == 0 { 1 << 3 } else { 0 },
                    house: 0,
                    zones: vec![],
                    items: vec![Item {
                        palette: 0,
                        depth: 0,
                        attrs: Attrs::default(),
                    }],
                })
                .collect(),
        }],
        &Default::default(),
    )
    .map_err(debug)?;
    let digest = writer::read(&bytes).map_err(debug)?.digest;
    Ok((
        bytes,
        crate::map::BundlePins {
            digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            production: true,
        },
    ))
}

fn case(protected: bool, stale: bool) -> TestResult {
    run(async |admin| {
        let seed = if protected {
            "idle_kick_nologout"
        } else if stale {
            "idle_kick_stale"
        } else {
            "idle_kick_release"
        };
        let h = Harness::create_with_admission_seed(admin, seed, true, false).await?;
        eprintln!("POSTGRES_STARTED IDLEKICK protected={protected} stale={stale}");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let character = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let active = controller.active().ok_or("active")?;
        deps.room = pin.qualified_room().clone();
        let (bytes, pins) = bundle()?;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let start = TilePos {
            x: u16::from(!protected),
            y: 0,
            floor: -7,
        };
        let world = boot(
            &bytes,
            &BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start,
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |_| None,
        )
        .map_err(debug)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            world.map_revision_digest(),
            (i32::from(start.x), 0, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            content_pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, at(i32::from(start.x), 0))
            .map_err(debug)?;
        deps.admitted.runtime_actor = Some(deps.actor);
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &character);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        owner.states.gameplay_idle_policy = GameplayIdlePolicy {
            warning_after_ms: 15,
            removal_after_ms: 16,
        };
        deps.initialize(&owner).await;
        track(&owner, &deps)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[31])
                .ok_or("notice capability")?;
        if stale {
            admitted.controller.as_mut().ok_or("controller")?.transport =
                crate::foundation::AuthenticatedTransportRefV1::decode(&[10; 16]).map_err(debug)?;
        }
        // Loading the full native generation can outlast the fixture's ready observation
        // under shared-PC load. Production runs this maintenance continuously.
        assert!(h.root.maintain_ready_once().await?);
        let clock = ClockOwner {
            owner: &owner,
            now: Cell::new(Instant::now()),
            replies: RefCell::new(Vec::new()),
            relocate_on_logout: Cell::new(false),
        };
        let origin = clock.now.get();
        let (mut server, mut client) = tokio::io::duplex(1 << 16);
        let serving = async {
            let end = serve_admitted(
                &mut server,
                admitted,
                &clock,
                LivenessPolicy {
                    // Semantic thresholds use the controlled clock. Leave real PostgreSQL
                    // refusal/reconciliation time inside each transport cadence.
                    interval: Duration::from_secs(1),
                    missed_limit: 10,
                },
            )
            .await;
            drop(server);
            end
        };
        let client_flow = async {
            let mut warnings = 0;
            let mut probes = 0;
            loop {
                let frame = match read(&mut client).await {
                    Ok(frame) => frame,
                    Err(_) if !stale => break,
                    Err(error) => {
                        return Err(format!(
                            "idle read: {error}; logout replies: {:?}",
                            clock.replies.borrow()
                        )
                        .into());
                    }
                };
                let envelope = decode_wire_envelope(&frame)?;
                match envelope.message_type() {
                    MessageType::SnapshotCommit => {
                        clock.now.set(origin + Duration::from_millis(15))
                    }
                    MessageType::StateDelta => {
                        let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                        assert_eq!(delta.domain_id, 23);
                        assert_eq!(
                            oteryn_protocol_oteryn::tutorial_notices::decode_tutorial_notice(delta.payload)
                                .map_err(debug)?,
                            oteryn_protocol_oteryn::tutorial_notices::TutorialNotice::StatusText(
                                "You have been inactive. You will be disconnected soon unless you perform a gameplay action.".into()
                            ),
                            "operator thresholds do not advertise default timing"
                        );
                        warnings += 1;
                    }
                    MessageType::LivenessProbe => {
                        probes += 1;
                        let ack = encode_liveness_ack(
                            admitted.continuity.connection_generation,
                            probes,
                            0,
                        )?;
                        client
                            .write_all(&u32::try_from(ack.len())?.to_be_bytes())
                            .await?;
                        client.write_all(&ack).await?;
                        clock.now.set(origin + Duration::from_millis(16));
                        if protected && probes == 2 {
                            let active: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_reconnect_sessions WHERE session_state=2").fetch_one(&h.pool).await?;
                            assert_eq!(active, 1, "NOLOGOUT kept the expired session active");
                            // Server relocation is not a gameplay input. The next idle pass
                            // must read the new tile and release without another 16 minutes.
                            place(&owner, admitted, at(1, 0)).await?;
                            clock.relocate_on_logout.set(true);
                        }
                        if protected && probes == 3 {
                            // Race the initial exemption read: the real logout sees a
                            // relocation back onto NOLOGOUT before its release fence.
                            assert_eq!(*clock.replies.borrow(), [LogoutReply::NoLogout]);
                            let active: i64 = sqlx::query_scalar("SELECT count(*) FROM game_durability_reconnect_sessions WHERE session_state=2").fetch_one(&h.pool).await?;
                            assert_eq!(active, 1, "NOLOGOUT rechecked at the release fence");
                            clock.relocate_on_logout.set(false);
                            place(&owner, admitted, at(1, 0)).await?;
                        }
                        if stale && probes >= 2 {
                            client.shutdown().await?;
                            break;
                        }
                        if !stale && probes >= if protected { 4 } else { 3 } {
                            client.shutdown().await?;
                            return Err(format!(
                                "idle release did not complete: {:?}",
                                clock.replies.borrow()
                            )
                            .into());
                        }
                    }
                    _ => {}
                }
            }
            assert_eq!(warnings, 1);
            Ok::<(), Box<dyn std::error::Error>>(())
        };
        let (end, result) = join_two(serving, client_flow).await;
        result?;
        let current = FreshAdmissionStore::from_root(h.root.clone())
            .current_session(admitted.game_session_id)
            .await?;
        let positions: Vec<(i32, i32, i16)> = sqlx::query_as(
            "SELECT tile_x,tile_y,tile_floor FROM game_character_last_positions LIMIT 2",
        )
        .fetch_all(&h.pool)
        .await?;
        if stale {
            assert_eq!(current.session_state(), GameSessionState::Active);
            assert!(positions.is_empty());
            assert!(
                matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
                "{end:?}; replies: {:?}",
                clock.replies.borrow()
            );
        } else {
            assert_eq!(current.session_state(), GameSessionState::Terminal);
            assert_eq!(positions, [(1, 0, 7)]);
            assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)));
            assert!(
                owner
                    .runtime
                    .lock()
                    .await
                    .player_control_facts(deps.actor, admitted.game_session_id)
                    .is_err()
            );
        }
        drop(owner);
        drop(character);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn idle_kick_postgres_router_saves_final_position_and_terminal_release() -> TestResult {
    large_stack(false, false)
}
#[test]
fn idle_kick_postgres_router_no_logout_tile_prevents_release() -> TestResult {
    large_stack(true, false)
}
#[test]
fn idle_kick_postgres_router_stale_transport_cannot_release() -> TestResult {
    large_stack(false, true)
}

fn large_stack(protected: bool, stale: bool) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || case(protected, stale).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "idle kick PG panic")??;
    Ok(())
}
