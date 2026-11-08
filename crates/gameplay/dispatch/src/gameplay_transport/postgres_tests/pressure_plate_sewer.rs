//! Crystal movements_sewer.lua:1-45: uid25030 creates/removes one sewer435.
use super::TestResult;
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{local, wire};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::AdmittedSession;
use crate::map::overlay::TilePos;
use crate::map::view::{MapFacts, compose_tile};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_spatial::{
    StepDirection, StepDisposition, decode_step_result, encode_step_intent,
};

#[test]
fn devil_helmet_pressure_plate_creates_sewer_then_removes_it_on_exit_and_replay() -> TestResult {
    scenario("devil-helmet-sewer")
}

#[test]
fn devil_helmet_pressure_plate_preserves_existing_sewer_until_exit() -> TestResult {
    scenario("devil-helmet-sewer-existing")
}

#[test]
fn devil_helmet_pressure_plate_ignores_unrelated_unique_id() -> TestResult {
    scenario("devil-helmet-sewer-unrelated")
}

#[test]
fn devil_helmet_pressure_plate_created_sewer_accepts_native_use_and_replay() -> TestResult {
    scenario("devil-helmet-sewer-use")
}

#[test]
fn devil_helmet_pressure_plate_existing_sewer_accepts_native_use_and_replay() -> TestResult {
    scenario("devil-helmet-sewer-use-existing")
}

#[test]
fn devil_helmet_pressure_plate_sewer_refuses_missing_landing() -> TestResult {
    scenario("devil-helmet-sewer-use-blocked")
}

fn scenario(name: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("devil-helmet-sewer", move || {
        super::quest_world_callbacks::scenario(
            name,
            Position {
                x: if name.contains("-use") { 32481 } else { 32480 },
                y: 32170,
                floor: -14,
            },
            false,
        )
    })?;
    println!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: ExactActorRef,
    fence: CurrentCharacterGameplayFence,
    trigger: Position,
    name: &str,
) -> TestResult {
    if name.contains("-use") {
        use crate::gameplay_transport::connection::FreshAdmissionAuthority;
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[4, 6, 11, 15, 18],
            )
            .ok_or("map USE capabilities")?;
    }
    let position = TilePos {
        x: 32482,
        y: 32170,
        floor: -14,
    };
    assert_eq!(
        sewers(owner, position)?,
        usize::from(name.ends_with("existing")),
        "initial source sewer"
    );
    let before = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    for (command, direction, expected_count) in [
        (1, StepDirection::East, 1),
        (2, StepDirection::West, 0),
        (3, StepDirection::East, 1),
        (4, StepDirection::West, 0),
    ]
    .into_iter()
    .filter(|_| name != "devil-helmet-sewer-use-existing")
    {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let payload = encode_step_intent(direction);
        let reply = wire(owner, admitted, command, 1, payload.clone()).await?;
        assert_eq!(
            decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        let expected_count = if name.ends_with("unrelated") {
            0
        } else {
            expected_count
        };
        assert_eq!(
            sewers(owner, position)?,
            expected_count,
            "source sewer after command {command}"
        );
        let expected_position = if command % 2 == 1 {
            trigger
        } else {
            Position {
                x: trigger.x - 1,
                ..trigger
            }
        };
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            local(expected_position)
        );
        let revision = owner
            .bundle_map()
            .ok_or("map")?
            .facts
            .movement_items(position)
            .first()
            .map(|entry| entry.source);
        assert_eq!(wire(owner, admitted, command, 1, payload).await?, reply);
        assert_eq!(sewers(owner, position)?, expected_count);
        assert_eq!(
            owner
                .bundle_map()
                .ok_or("map")?
                .facts
                .movement_items(position)
                .first()
                .map(|entry| entry.source),
            revision,
            "replay retains the same physical object"
        );
    }
    if name.contains("-use") {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let step_id = if name.ends_with("use-existing") { 1 } else { 5 };
        let step = wire(
            owner,
            admitted,
            step_id,
            1,
            encode_step_intent(StepDirection::East),
        )
        .await?;
        assert_eq!(
            decode_step_result(&step).map_err(debug)?,
            StepDisposition::Moved
        );
        assert_eq!(sewers(owner, position)?, 1);
        let (reply, payload) = use_sewer(owner, admitted, step_id + 1, position).await?;
        let expected = if name.ends_with("blocked") {
            oteryn_protocol_oteryn::world_object::UseDisposition::NotPossible
        } else {
            oteryn_protocol_oteryn::world_object::UseDisposition::Committed
        };
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
            expected
        );
        let after = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?;
        assert_eq!(
            after.position(),
            local(if name.ends_with("blocked") {
                trigger
            } else {
                Position {
                    x: position.x,
                    y: position.y,
                    floor: -15,
                }
            })
        );
        assert_eq!(wire(owner, admitted, step_id + 1, 2, payload).await?, reply);
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?,
            after,
            "USE replay must not descend twice"
        );
        assert_eq!(
            sewers(owner, position)?,
            usize::from(name.ends_with("blocked")),
            "dropdown must dispatch the committed plate StepOut"
        );
    }
    assert_eq!(
        owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?,
        before
    );
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        after, receipts,
        "physical plate must not invent quest progress"
    );
    Ok(())
}

fn sewers(owner: &ComposedFreshAdmission<'_, '_, '_>, position: TilePos) -> TestResult<usize> {
    let map = owner.bundle_map().ok_or("map")?;
    let tile = compose_tile(&map.overlay, &map.facts, position)
        .map_err(debug)?
        .ok_or("sewer tile")?;
    Ok(tile
        .entries
        .iter()
        .filter(|entry| entry.facts.appearance_id == 435)
        .count())
}

// Resolve the same handle / base ordinal that the real client receives. A
// movement-created sewer is handle-bearing, unlike its immutable base version.
async fn use_sewer(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command_id: u64,
    position: TilePos,
) -> TestResult<(Vec<u8>, Vec<u8>)> {
    use crate::foundation::{MessageType, decode_wire_envelope};
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use oteryn_protocol_oteryn::world_map::{
        MapOrigin, STATE_DOMAIN_WORLD_MAP_VIEW, decode_world_map_snapshot,
    };
    use oteryn_protocol_oteryn::world_object::{
        UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
    };
    use tokio::io::AsyncWriteExt;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let receiving = async move {
        let mut target = None;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&raw).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                let (_, domains) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())
                        .map_err(debug)?;
                for domain in domains {
                    if domain.domain_id != STATE_DOMAIN_WORLD_MAP_VIEW {
                        continue;
                    }
                    let snapshot = decode_world_map_snapshot(domain.payload).map_err(debug)?;
                    for tile in snapshot.tiles {
                        if (tile.position.x, tile.position.y, tile.position.floor)
                            != (
                                i32::from(position.x),
                                i32::from(position.y),
                                i16::from(position.floor),
                            )
                        {
                            continue;
                        }
                        for item in tile.items {
                            if item.appearance_id != 435 {
                                continue;
                            }
                            target = Some(match item.origin {
                                MapOrigin::Handle(handle) => UseTarget::Item(handle),
                                MapOrigin::BaseOrdinal {
                                    ordinal,
                                    object_revision,
                                } => {
                                    let mut placement = snapshot.header.bundle_digest.to_vec();
                                    placement.extend_from_slice(
                                        &crate::map::view::placement_key(position, ordinal)
                                            .ok_or("placement")?
                                            .to_be_bytes(),
                                    );
                                    UseTarget::WorldObject(WorldObjectTarget {
                                        placement,
                                        expected_revision: object_revision,
                                    })
                                }
                                MapOrigin::DisplayOnly => {
                                    return Err("sewer is display-only".into());
                                }
                            });
                        }
                    }
                }
            }
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        let payload = encode_item_use_intent(&UseIntent {
            target: target.ok_or("visible sewer")?,
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)?;
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: 2,
                payload: &payload,
            },
        )
        .map_err(debug)?;
        client.write_u32(u32::try_from(frame.len())?).await?;
        client.write_all(&frame).await?;
        loop {
            let raw = super::item_look::read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&raw).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                    .map_err(debug)?;
                assert_eq!(result.command_id, command_id);
                let reply = result.payload.to_vec();
                client.shutdown().await?;
                return Ok((reply, payload));
            }
        }
    };
    let reader = tokio::spawn(async move {
        let result: TestResult<(Vec<u8>, Vec<u8>)> = receiving.await;
        result.map_err(|error| error.to_string())
    });
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    let result = reader.await?.map_err(Into::into);
    if let ConnectionEnd::AdmittedThenDisconnected(a) | ConnectionEnd::AdmittedThenClosed(a, _) =
        end
    {
        *admitted = a;
    }
    result
}
