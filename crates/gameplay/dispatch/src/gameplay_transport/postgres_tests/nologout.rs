//! ZONES-NOLOGOUT-1: retained bundle flags through the composed logout owner and real router.
use super::autowalk::{at, place, read, send};
use super::fixture::{Harness, debug, join_two, run};
use super::{Dependencies, TestResult, prepare_current_session, track};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use oteryn_protocol_oteryn::logout::{LogoutResult, decode_logout_result};
use oteryn_protocol_oteryn::tutorial_notices::{TutorialNotice, decode_tutorial_notice};
use oteryn_protocol_oteryn::{
    MessageType, decode_command_result, decode_snapshot_chunk, decode_state_delta,
    decode_wire_envelope,
};
use tokio::io::AsyncWriteExt;

#[test]
fn nologout_bundle_logout_and_motd_real_dispatch_postgres() -> TestResult {
    // The composed dispatch future needs the same stack as neighboring wire scenarios.
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "nologout e2e panicked")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "nologout", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let (bytes, mut pins) = crate::map::boot::tests::bundle();
        let mut data = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
        data.manifest.palette[0].key = "oteryn:terrain.tibia.i100".into();
        data.sectors[0].tiles[0]
            .items
            .push(oteryn_world_bundle::sector::Item {
                palette: 4,
                depth: 0,
                attrs: Default::default(),
            });
        data.sectors[0].tiles[0].flags = 8 | 1; // NOLOGOUT also applies inside PZ.
        data.sectors[0].tiles[1].flags = 1; // PZ permits logout during a fight.
        let bytes = oteryn_world_bundle::writer::write(&data.manifest, &data.sectors, &data.spawns)
            .map_err(debug)?;
        pins.digest = oteryn_world_bundle::bundle::read(&bytes)
            .map_err(debug)?
            .digest;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let world = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 1,
                    y: 0,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| match key {
                "item:box" => Some(crate::map::facts::ItemDefinition {
                    reference: std::num::NonZeroU32::new(1)?,
                    solid: Some(true),
                    pickupable: false,
                    blocks_projectile: true,
                    door: false,
                }),
                "item:coin" => Some(crate::map::facts::ItemDefinition {
                    reference: std::num::NonZeroU32::new(2)?,
                    solid: Some(false),
                    pickupable: true,
                    blocks_projectile: false,
                    door: false,
                }),
                _ => None,
            },
        )
        .map_err(debug)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let deps = Dependencies::with_content(
            ChannelContentPin::from_activation(
                deps.admitted.world_id,
                1,
                cells.scope().generation_digest,
                [2; 32],
                world.frame_binding_digest(deps.admitted.world_id),
                world.map_revision_digest(),
                (1, 0, 7),
            ),
            deps.room,
            deps.spells,
        )?;
        // A loose coin makes spell semantics unknown without hiding the zone flags.
        assert!(
            cells
                .spell_tiles()
                .lookup(
                    cells.scope(),
                    crate::content::LogicalCell { x: 1, y: 0, z: 7 }
                )
                .is_err()
        );
        let mut owner = deps.owner(&h, &authority);
        owner.movement_cells = &cells;
        owner.server_motd = "Welcome to Oteryn!";
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_pinned_test_position(deps.actor, at(1, 0))
            .map_err(debug)?;
        track(&owner, &deps)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[21, 92])
                .ok_or("selection")?;
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let flow = async {
            let mut motds = 0;
            loop {
                let frame = read(&mut client).await?;
                let envelope = decode_wire_envelope(&frame)?;
                if envelope.message_type() == MessageType::SnapshotChunk {
                    for domain in decode_snapshot_chunk(envelope.payload())?.1 {
                        if domain.domain_id == 57 {
                            assert_eq!(
                                decode_tutorial_notice(domain.payload).map_err(debug)?,
                                TutorialNotice::StatusText("Welcome to Oteryn!".into())
                            );
                            motds += 1;
                        }
                    }
                }
                if envelope.message_type() == MessageType::SnapshotCommit {
                    break;
                }
            }
            assert_eq!(motds, 1);
            // A hit cannot supersede NOLOGOUT's earlier refusal, even in a protection zone.
            {
                let runtime = owner.runtime.lock().await;
                owner.states.attack.lock().await.record_hit_taken(
                    &runtime,
                    deps.actor,
                    admitted.game_session_id,
                    owner.owner_now(),
                );
            }
            send(&mut client, 1, 23, &[]).await?;
            let mut refused = false;
            loop {
                let frame = read(&mut client).await?;
                let envelope = decode_wire_envelope(&frame)?;
                match envelope.message_type() {
                    MessageType::CommandResult => {
                        let result = decode_command_result(envelope.payload())?;
                        assert_eq!(result.status, crate::foundation::CommandStatus::Rejected);
                        assert_eq!(
                            decode_logout_result(result.payload).map_err(debug)?,
                            LogoutResult::Busy
                        );
                        refused = true;
                    }
                    MessageType::StateDelta => {
                        let delta = decode_state_delta(envelope.payload())?;
                        if delta.domain_id == 57 {
                            assert!(refused, "result precedes its refusal message");
                            assert_eq!((delta.base_revision, delta.new_revision), (0, 1));
                            assert_eq!(
                                decode_tutorial_notice(delta.payload).map_err(debug)?,
                                TutorialNotice::StatusText("You can not logout here.".into())
                            );
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let state =
                crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                    .current_session(admitted.game_session_id)
                    .await
                    .map_err(debug)?;
            assert_eq!(
                state.session_state(),
                crate::foundation::GameSessionState::Active
            );
            let saves: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_last_positions")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(saves, 0);
            place(&owner, admitted, at(3, 0)).await?;
            // The live fight deadline still blocks outside PZ.
            send(&mut client, 2, 23, &[]).await?;
            let mut in_fight = false;
            loop {
                let frame = read(&mut client).await?;
                let envelope = decode_wire_envelope(&frame)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload())?;
                    assert!(matches!(
                        decode_logout_result(result.payload).map_err(debug)?,
                        LogoutResult::InFight { .. }
                    ));
                    in_fight = true;
                }
                if envelope.message_type() == MessageType::StateDelta {
                    let delta = decode_state_delta(envelope.payload())?;
                    if delta.domain_id == 57 {
                        assert!(in_fight, "result precedes its fight message");
                        assert_eq!((delta.base_revision, delta.new_revision), (1, 2));
                        assert_eq!(
                            decode_tutorial_notice(delta.payload).map_err(debug)?,
                            TutorialNotice::StatusText(
                                "You may not logout during or immediately after a fight!".into()
                            )
                        );
                        break;
                    }
                }
            }
            // A qualified PZ keeps the already-existing fight exception.
            place(&owner, admitted, at(2, 0)).await?;
            send(&mut client, 3, 23, &[]).await?;
            loop {
                let frame = read(&mut client).await?;
                let envelope = decode_wire_envelope(&frame)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload())?;
                    assert_eq!(result.command_id, 3);
                    assert_eq!(
                        decode_logout_result(result.payload).map_err(debug)?,
                        LogoutResult::Accepted
                    );
                    break;
                }
            }
            client.shutdown().await?;
            Ok::<_, Box<dyn std::error::Error>>(())
        };
        let (end, result) = join_two(
            serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS),
            flow,
        )
        .await;
        result?;
        assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)));
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
