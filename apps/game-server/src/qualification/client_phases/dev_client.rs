use super::*;

pub(super) async fn dev_client<'a>(
    clients: &SeamClients<'a>,
    next_grant: &mut impl FnMut(&str, [u8; 16], u64) -> OwnedGrant<'a>,
) -> TestResult<()> {
    let &SeamClients {
        address,
        certificate,
        accounts,
        characters,
        world,
        descriptor,
        url,
        ..
    } = clients;
    evidence("stage=dev_client");
    // #162 C1b (owner decision A6-b; coordinator #162 comment 5875470550, option 3 for the door
    // assertion) and C2: the dev/qualification-only native client (`oteryn-dev-client`, a dev-
    // dependency of this crate — `workspace-boundaries.toml` `[dev_edges]`, never walked by the
    // production-closure check) drives one full admission end-to-end over the real loopback
    // TCP+TLS listener, admitted with the same WP5 fixture grant mechanism
    // (`next_grant`/`sign_grant`) every other admission in this file uses. It reads the join
    // snapshot (state domains 1 `WORLD_SPATIAL` and 2 `WORLD_OBJECT_OVERLAY`) and then itself
    // issues the door scenario one command at a time (`step`, `use_object`), decoding every
    // `CommandResult` disposition and server-sequenced delta, using exclusively
    // `oteryn-protocol-oteryn`'s own client-direction codecs — `oteryn-dev-client` holds no codec
    // of its own. `characters[1]` is free here (released to TERMINAL by the `use_wire` stage
    // above), so this is the 4th durable admission. Its connection then closes the same "silent"
    // way `concurrent[0]`/`use_session` above do (the dev client is dropped after its last
    // command, dropping the TLS stream), so it goes through the identical control-loss (60s)
    // then grace-release (100s) cycle before this function returns, keeping the same single
    // committed actor (character[0]'s own final re-admission below) invariant `seam_flow`'s
    // caller checks last.
    //
    // The door's expected join-snapshot entry here is the same construction `use_wire_frames`
    // used for its own final overlay delta above (open then close: revision 0 -> 1 -> 2), which
    // the `use_wire` stage's own byte-for-byte comparison
    // (`frames.get(1..) != Some(expected.as_slice())`) already proved matches what the real
    // server put on the wire — not a fresh-door assumption, and not a live runtime query either,
    // so this holds identically whether this stage is composed locally
    // (`server_seam_real_owners_over_tcp_tls`, `runtime` is `Some`) or run against an
    // externally-running node (`node_boot_seam_against_running_node`, `runtime` is `None` and
    // there is no local door object to query at all). The commands' expected values are derived
    // the way `use_wire_frames` derives its own: the same room, the same cells, the same door
    // key and state keys, CommandIds and server sequences counted from 1 (a fresh GameSession),
    // the spatial revision chain from the join snapshot's baseline, and the door's overlay
    // revision chain from the door revision the join snapshot carried.
    let dev_client_room = crate::content::qualify_native_entry_room(WorldId::decode(&world)?)
        .map_err(|e| format!("dev client native entry room: {e}"))?;
    let dev_client_content_generation = dev_client_room.compiled().client_digest();
    let door_placement = crate::content::accepted::DOOR_CELL.0.as_bytes().to_vec();
    let door_state = crate::content::accepted::DOOR_CLOSED_STATE
        .as_bytes()
        .to_vec();
    let door_revision: u64 = 2;

    let dev_client_generation = platform_generation(descriptor, &accounts[1]).await?;
    let dev_client_grant = next_grant(&accounts[1], characters[1], dev_client_generation);
    let dev_client_token = sign_grant(&dev_client_grant.borrowed(), now_seconds()?);
    let dev_client_character = crate::foundation::CharacterId::decode(&characters[1])?;
    let mut dev_client = oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
        address,
        server_name: "localhost",
        root_certificate: certificate,
        schema_revision: 1,
        character_id: dev_client_character,
        admission_material: dev_client_token.as_bytes(),
        client_build_id: "oteryn-dev-client/seam-qualification",
        deadline: Duration::from_secs(20),
    })
    .await
    .map_err(|error| format!("dev client join: {error}"))?;
    let dev_client_snapshot = dev_client.join_snapshot().clone();

    if dev_client_snapshot.world_spatial.content_generation != dev_client_content_generation
        || dev_client_snapshot.world_spatial.actor_position
            != (ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            })
    {
        return Err(format!(
            "dev client join snapshot world_spatial mismatch: {:?}",
            dev_client_snapshot.world_spatial
        )
        .into());
    }
    if dev_client_snapshot.world_object_overlay.len() != 1 {
        return Err(format!(
            "dev client join snapshot overlay count: {:?}",
            dev_client_snapshot.world_object_overlay
        )
        .into());
    }
    let dev_client_door = &dev_client_snapshot.world_object_overlay[0];
    if dev_client_door.content_generation != dev_client_content_generation
        || dev_client_door.placement != door_placement
        || dev_client_door.state != door_state
        || dev_client_door.revision != door_revision
    {
        return Err(format!(
            "dev client join snapshot native entry door overlay mismatch: {dev_client_door:?} \
             (expected placement={door_placement:?} state={door_state:?} revision={door_revision})"
        )
        .into());
    }
    // A fresh GameSession: baseline spatial revision 1, and the overlay domain revision is the
    // door's own revision (`connection.rs` `serve_admitted`).
    if dev_client_snapshot.world_spatial_revision != 1
        || dev_client_snapshot.world_object_overlay_revision != door_revision
    {
        return Err(format!(
            "dev client join snapshot domain revisions: spatial={} overlay={} (expected 1 and {door_revision})",
            dev_client_snapshot.world_spatial_revision,
            dev_client_snapshot.world_object_overlay_revision,
        )
        .into());
    }
    if committed_admissions(url).await? != 4 {
        return Err("dev client admission did not commit exactly one GameSession".into());
    }
    evidence(&format!(
        "dev_client admission=committed join_snapshot=decoded domain1=world_spatial domain2=world_object_overlay door=native_entry_door door_revision={door_revision} admissions=4",
    ));

    // C2: the dev client itself steps next to the door, USEs it open, steps through the doorway
    // and back out, and USEs it closed: `use_wire_frames`'s east / use-open / north / south /
    // use-close, without its doorway `use` and negative tail. A fresh GameSession numbers
    // CommandIds from 1 and server sequences from 0 exactly as there.
    if dev_client.next_command_id() != 1 || dev_client.last_server_sequence() != 0 {
        return Err(format!(
            "dev client fresh session numbering: next_command_id={} last_server_sequence={}",
            dev_client.next_command_id(),
            dev_client.last_server_sequence()
        )
        .into());
    }
    dev_client_door_scenario(
        &mut dev_client,
        &dev_client_snapshot,
        dev_client_content_generation,
        &door_placement,
        door_revision,
    )
    .await?;
    evidence(&format!(
        "dev_client step_east=moved use_open=committed door_open_revision={} step_through=moved step_out=moved use_close=committed door_closed_revision={} admissions=4",
        door_revision + 1,
        door_revision + 2,
    ));
    // Dropped here, closing the TLS stream the same silent way the join-only stage did.
    drop(dev_client);
    let dev_client_session = *dev_client_snapshot.game_session_id.as_bytes();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        if let Some((1, epoch, grace)) = session_loss_row(url, dev_client_session).await? {
            if epoch != "1" || !(58..=62).contains(&grace) {
                return Err(format!("dev_client loss epoch={epoch} grace={grace}").into());
            }
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("dev_client session never became reconnectable".into());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(100);
    while !matches!(
        session_loss_row(url, dev_client_session).await?,
        Some((3, _, _))
    ) {
        if tokio::time::Instant::now() >= deadline {
            return Err("dev_client session was not released after grace".into());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    evidence("dev_client released=terminal admissions=4");

    Ok(())
}

async fn dev_client_door_scenario(
    dev_client: &mut oteryn_dev_client::DevClientSession,
    dev_client_snapshot: &oteryn_dev_client::JoinSnapshot,
    dev_client_content_generation: [u8; 32],
    door_placement: &[u8],
    door_revision: u64,
) -> TestResult {
    {
        use oteryn_dev_client::{AppliedDelta, CommandOutcome, SessionEvent};
        use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectOverlayEntry};
        let spatial_baseline = dev_client_snapshot.world_spatial_revision;
        let content_generation = dev_client_content_generation;
        let moved =
            |command_id: u64, result_sequence: u64, base: u64, x: i32, y: i32| CommandOutcome {
                command_id,
                status: CommandStatus::Accepted,
                disposition: StepDisposition::Moved,
                result_server_sequence: result_sequence,
                world_spatial_delta: Some(AppliedDelta {
                    server_sequence: result_sequence + 1,
                    base_revision: base,
                    new_revision: base + 1,
                    value: WorldSpatialObservation {
                        content_generation,
                        actor_position: ActorPosition { x, y, floor: 0 },
                    },
                }),
                world_object_overlay_delta: None,
                gate_message_id: None,
            };
        let door_entry = |state: &str, revision: u64| WorldObjectOverlayEntry {
            content_generation,
            placement: door_placement.to_vec(),
            state: state.as_bytes().to_vec(),
            revision,
        };
        // A `COMMITTED` use names no domain, so the exchange returns at its result and carries no
        // delta (SESSION-PUSH-1); the door's domain 2 delta arrives through the event path.
        let committed_use = |command_id: u64, result_sequence: u64| CommandOutcome {
            command_id,
            status: CommandStatus::Accepted,
            disposition: UseDisposition::Committed,
            result_server_sequence: result_sequence,
            world_spatial_delta: None,
            world_object_overlay_delta: None,
            gate_message_id: None,
        };
        let door_event = |result_sequence: u64, base: u64, state: &str| {
            SessionEvent::WorldObjectOverlay(AppliedDelta {
                server_sequence: result_sequence + 1,
                base_revision: base,
                new_revision: base + 1,
                value: door_entry(state, base + 1),
            })
        };

        // cmd1: east (0,0) -> (1,0), adjacent to the door cell (1,-1).
        let step_east = dev_client
            .step(StepDirection::East)
            .await
            .map_err(|error| format!("dev client step east: {error}"))?;
        dev_client_expect(
            "step east",
            &step_east,
            &moved(1, 1, spatial_baseline, 1, 0),
        )?;
        dev_client_step_event(dev_client, "step east", &step_east)?;
        // cmd2: USE the door open (expected revision = the joined door revision).
        let use_open = dev_client
            .use_object(door_placement, door_revision)
            .await
            .map_err(|error| format!("dev client use open: {error}"))?;
        dev_client_expect("use open", &use_open, &committed_use(2, 3))?;
        // The door's domain 2 delta (base, new revision, entry) is read idle and asserted from the
        // event path before the next command.
        let open_events = dev_client_door_events(dev_client).await?;
        dev_client_expect(
            "use open door delta",
            &open_events,
            &vec![door_event(
                3,
                door_revision,
                crate::content::accepted::DOOR_OPEN_STATE,
            )],
        )?;
        // cmd3: north (1,0) -> (1,-1), through the now open doorway.
        let step_through = dev_client
            .step(StepDirection::North)
            .await
            .map_err(|error| format!("dev client step through: {error}"))?;
        dev_client_expect(
            "step through",
            &step_through,
            &moved(3, 5, spatial_baseline + 1, 1, -1),
        )?;
        dev_client_step_event(dev_client, "step through", &step_through)?;
        // cmd4: south (1,-1) -> (1,0), out of the doorway.
        let step_out = dev_client
            .step(StepDirection::South)
            .await
            .map_err(|error| format!("dev client step out: {error}"))?;
        dev_client_expect(
            "step out",
            &step_out,
            &moved(4, 7, spatial_baseline + 2, 1, 0),
        )?;
        dev_client_step_event(dev_client, "step out", &step_out)?;
        // cmd5: USE the door closed (expected revision = the revision the open delta reported).
        let use_close = dev_client
            .use_object(door_placement, door_revision + 1)
            .await
            .map_err(|error| format!("dev client use close: {error}"))?;
        dev_client_expect("use close", &use_close, &committed_use(5, 9))?;
        let close_events = dev_client_door_events(dev_client).await?;
        dev_client_expect(
            "use close door delta",
            &close_events,
            &vec![door_event(
                9,
                door_revision + 1,
                crate::content::accepted::DOOR_CLOSED_STATE,
            )],
        )?;

        // The session's own applied state: back at (1,0,0), door closed at revision + 2, every
        // CommandId (1..=5) and server sequence (1..=10) consumed.
        dev_client_expect(
            "final position",
            dev_client.world_spatial(),
            &WorldSpatialObservation {
                content_generation,
                actor_position: ActorPosition {
                    x: 1,
                    y: 0,
                    floor: 0,
                },
            },
        )?;
        dev_client_expect(
            "final door overlay",
            dev_client.world_object_overlay(),
            &[door_entry(
                crate::content::accepted::DOOR_CLOSED_STATE,
                door_revision + 2,
            )][..],
        )?;
        dev_client_expect(
            "final numbering",
            &(
                dev_client.next_command_id(),
                dev_client.last_server_sequence(),
            ),
            &(6, 10),
        )?;
    }
    Ok(())
}
