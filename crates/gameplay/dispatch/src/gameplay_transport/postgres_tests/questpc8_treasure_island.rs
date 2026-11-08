//! Chosen exploration-only journey drives both paid ship routes and the native sand/rope cells.
use super::fixture::{Harness, debug, id};
use super::questpc8_authored_programmes::{object_target, step, use_target};
use super::{TestResult, quest_world_callbacks};
use crate::foundation::{
    MovementFacing, MovementLocalPosition, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FreshAdmissionAuthority},
};
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

#[path = "questpc8_treasure_island/world.rs"]
mod fixture_world;
pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    fixture_world::world()
}

#[test]
fn questpc8_treasure_island_paid_ship_map_shovel_rope_explore_and_return() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc8-treasure-island", || {
        quest_world_callbacks::scenario(
            "authored-treasure-island",
            Position {
                x: 32131,
                y: 32913,
                floor: -7,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_treasure_island_paid_ship_map_shovel_rope_explore_and_return duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn at(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position())
}
async fn walk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: &mut u64,
    x: i32,
    y: i32,
) -> TestResult {
    for _ in 0..64 {
        let p = at(owner, admitted).await?;
        let direction = if p.x < x {
            StepDirection::East
        } else if p.x > x {
            StepDirection::West
        } else if p.y < y {
            StepDirection::South
        } else if p.y > y {
            StepDirection::North
        } else {
            return Ok(());
        };
        step(owner, admitted, *command, direction).await?;
        *command += 1;
    }
    Err("treasure route did not arrive".into())
}
async fn progress(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/chosen/treasure-island/stage'),-1)").bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
async fn owned(h: &Harness, item: u32) -> TestResult<Vec<String>> {
    Ok(sqlx::query_scalar("SELECT i.item_instance_id::text FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid)c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_production_key=$3 ORDER BY i.item_instance_id").bind(id(41).as_slice()).bind(id(42).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).fetch_all(&h.pool).await?)
}
async fn give(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u32,
    count: u16,
    n: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let p = at(owner, admitted).await?;
    quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: u16::try_from(p.x)?,
                y: u16::try_from(p.y)?,
                floor: -i8::try_from(p.floor)?,
            },
            item: format!("oteryn:item.tibia.i{item}"),
            count,
        },
        n,
        true,
    )
    .await
}
async fn install(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<[EntityRef; 2]> {
    let catalogue = crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        crate::gameplay_transport::npc_talk::NpcTalkBook::from_model(&model),
    ));
    owner.states.chat.admit(
        admitted.game_session_id,
        "Treasure explorer",
        owner.owner_now(),
    );
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut runtime = owner.runtime.lock().await;
    runtime
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let keys = [
        "oteryn:npc.captain_waverider",
        "oteryn:npc.captain_waverider_island",
    ];
    runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: keys
                    .iter()
                    .zip([(32346, 32859), (32131, 32914)])
                    .map(|(key, (x, y))| NpcPlacements {
                        key: (*key).into(),
                        placements: vec![NpcPlacement {
                            x,
                            y,
                            floor: -7,
                            direction: MovementFacing::North,
                        }],
                    })
                    .collect(),
            },
            |_| true,
            true,
        )
        .map_err(debug)?;
    let mut found = Vec::new();
    for key in keys {
        let p = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|npc| runtime.npc_key(npc.actor) == Some(key))
            .ok_or("captain placement")?;
        found.push(EntityRef {
            identity: p.actor.placement_identity(),
            generation: p.generation,
        });
    }
    found.try_into().map_err(|_| "captains".into())
}
async fn conversation(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    command: &mut u64,
    keyword: &str,
) -> TestResult<Vec<String>> {
    use crate::foundation::{MessageType, decode_wire_envelope};
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    async fn read_frame(client: &mut tokio::io::DuplexStream) -> std::io::Result<Vec<u8>> {
        let mut prefix = [0; 4];
        client.read_exact(&mut prefix).await?;
        let length = usize::try_from(u32::from_be_bytes(prefix)).map_err(std::io::Error::other)?;
        if length > 1 << 20 {
            return Err(std::io::Error::other("NPC frame exceeds fixture bound"));
        }
        let mut frame = vec![0; length];
        client.read_exact(&mut frame).await?;
        Ok(frame)
    }
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let before = *admitted;
    let first = *command;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async move {
        // Reconnect snapshots intentionally clear NPC focus. One real connection owns
        // the full greeting, exact quote and confirmation sequence.
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        let mut replies = Vec::new();
        for (offset, text) in ["hi", keyword, "yes"].into_iter().enumerate() {
            let id = first
                .checked_add(u64::try_from(offset)?)
                .ok_or("command exhaustion")?;
            let last = offset == 2;
            let payload =
                oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
                    npc_actor: npc,
                    text: text.into(),
                })
                .map_err(debug)?;
            let frame = oteryn_protocol_oteryn::encode_client_command(
                before.continuity.connection_generation,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id: id,
                    command_type: 7,
                    payload: &payload,
                },
            )
            .map_err(debug)?;
            client
                .write_all(&u32::try_from(frame.len())?.to_be_bytes())
                .await?;
            client.write_all(&frame).await?;
            let mut result = false;
            let mut lines = Vec::new();
            loop {
                let frame = match read_frame(&mut client).await {
                    Err(error)
                        if last && result && error.kind() == std::io::ErrorKind::UnexpectedEof =>
                    {
                        break;
                    }
                    read => read?,
                };
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                match envelope.message_type() {
                    MessageType::CommandResult => {
                        let outcome =
                            oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                                .map_err(debug)?;
                        assert_eq!(outcome.command_id, id);
                        assert_eq!(
                            outcome.status,
                            oteryn_protocol_oteryn::CommandStatus::Accepted,
                            "NPC {text}: error_code={}, payload_bytes={}",
                            outcome.error_code,
                            outcome.payload.len()
                        );
                        assert_eq!(
                            oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(
                                outcome.payload
                            )
                            .map_err(debug)?,
                            NpcIntentDisposition::Ok,
                            "{text}"
                        );
                        result = true;
                        if last {
                            client.shutdown().await?;
                        }
                    }
                    MessageType::StateDelta => {
                        let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                            .map_err(debug)?;
                        if delta.domain_id == 7 {
                            let value =
                                oteryn_protocol_oteryn::npc_service::decode_npc_conversation(
                                    delta.payload,
                                )
                                .map_err(debug)?;
                            lines.extend(value.lines);
                        }
                    }
                    _ => {}
                }
                // Wait for the actual new reply as well as the result before sending
                // another utterance; snapshot text never satisfies a quote assertion.
                if !last && result && !lines.is_empty() {
                    break;
                }
            }
            replies.push(lines.join(" "));
        }
        Ok(replies)
    };
    let (ended, result): (_, TestResult<Vec<String>>) =
        super::fixture::join_two(serving, driving).await;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &ended
    {
        *admitted = *current;
    }
    let replies =
        result.map_err(|error| format!("Treasure NPC conversation ended {ended:?}: {error}"))?;
    *command = first.checked_add(3).ok_or("command exhaustion")?;
    Ok(replies)
}

async fn fixture_move(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    destination: MovementLocalPosition,
) -> TestResult {
    // Negative probe only: landing without a fresh paid journey must earn no progress.
    let mut runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let expected = runtime.read_actor_position(actor).map_err(debug)?;
    let map = owner.bundle_map().ok_or("map")?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        &owner.movement_cells,
        actor,
        admitted.game_session_id,
        expected,
        destination,
        false,
        &doors,
    )
    .map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}
async fn fees(h: &Harness) -> TestResult<Vec<i64>> {
    Ok(sqlx::query_scalar("SELECT fee_gold_units FROM game_item_fee_burns WHERE character_id=encode($1,'hex')::uuid AND cause_kind=8 ORDER BY fee_gold_units DESC").bind(id(41).as_slice()).fetch_all(&h.pool).await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let [outbound, inbound] = install(owner, admitted).await?;
    // Select the native offered NPC capability through the same bounded admission
    // selector as the production handshake and the existing Grizzly wire fixture.
    // Command ids and connection generation continue solely through actual wire results.
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[3],
        )
        .ok_or("native NPC capability")?;
    assert_eq!(admitted.continuity.selected_capabilities.as_slice(), &[3]);
    let premium = owner.premium_coordinator.take();
    let mut command = 1;
    // Neither a landing tile nor tools alone manufacture the required completed ship receipt.
    give(owner, admitted, 3457, 1, 801).await?;
    give(owner, admitted, 3003, 1, 802).await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(progress(h).await?, -1);
    let refused = conversation(owner, admitted, inbound, &mut command, "liberty_bay").await?;
    assert!(refused[1].contains("50 gold"));
    assert!(refused[2].contains("enough money"));
    assert!(fees(h).await?.is_empty());
    give(owner, admitted, 3035, 4, 803).await?;
    // A real paid return made before this programme must not later satisfy its final return.
    let early = conversation(owner, admitted, inbound, &mut command, "liberty_bay").await?;
    assert!(early[1].contains("50 gold"));
    assert_eq!(
        at(owner, admitted).await?,
        MovementLocalPosition {
            x: 32349,
            y: 32856,
            floor: 7
        }
    );
    assert_eq!(fees(h).await?, vec![50]);
    walk(owner, admitted, &mut command, 32346, 32858).await?;
    tokio::time::sleep(std::time::Duration::from_millis(3100)).await;
    let passage = conversation(owner, admitted, outbound, &mut command, "passage").await?;
    assert!(passage[1].contains("200 gold"));
    assert_eq!(
        at(owner, admitted).await?,
        MovementLocalPosition {
            x: 32131,
            y: 32913,
            floor: 7
        }
    );
    assert_eq!(fees(h).await?, vec![200, 50]);
    step(owner, admitted, command, StepDirection::West).await?;
    command += 1;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(
        progress(h).await?,
        -1,
        "current Premium still required after paid arrival"
    );
    owner.premium_coordinator = premium;
    owner.admit_premium(id(40), admitted.game_session_id);
    step(owner, admitted, command, StepDirection::West).await?;
    command += 1;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(
        progress(h).await?,
        -1,
        "a pirate map must be carried with both tools"
    );
    give(owner, admitted, 9205, 1, 804).await?;
    let shovel = owned(h, 3457).await?;
    let rope = owned(h, 3003).await?;
    let map = owned(h, 9205).await?;
    assert_eq!(shovel.len(), 1);
    assert_eq!(rope.len(), 1);
    assert_eq!(map.len(), 1);
    step(owner, admitted, command, StepDirection::West).await?;
    command += 1;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(progress(h).await?, 1);
    walk(owner, admitted, &mut command, 32126, 32933).await?;
    let sand = TilePos {
        x: 32126,
        y: 32934,
        floor: -7,
    };
    let target = object_target(owner, sand, 0)?;
    let dig_command = command;
    assert_eq!(
        use_target(owner, admitted, command, &target).await?,
        UseDisposition::Committed
    );
    command += 1;
    assert_eq!(
        use_target(owner, admitted, dig_command, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(progress(h).await?, 2);
    step(owner, admitted, command, StepDirection::South).await?;
    command += 1;
    assert_eq!(
        at(owner, admitted).await?,
        MovementLocalPosition {
            x: 32126,
            y: 32934,
            floor: 8
        },
        "source615 must descend onto native386"
    );
    step(owner, admitted, command, StepDirection::South).await?;
    command += 1;
    assert_eq!(progress(h).await?, 3);
    let target = object_target(owner, TilePos { floor: -8, ..sand }, 0)?;
    assert_eq!(
        use_target(owner, admitted, command, &target).await?,
        UseDisposition::Committed
    );
    command += 1;
    assert_eq!(
        at(owner, admitted).await?,
        MovementLocalPosition {
            x: 32126,
            y: 32933,
            floor: 7
        }
    );
    assert_eq!(progress(h).await?, 4);
    // A source-qualified fixture relocation deliberately attempts to bypass the final
    // ship. The only paid return receipt is older than the rope objective, so refuse.
    fixture_move(
        owner,
        admitted,
        MovementLocalPosition {
            x: 32348,
            y: 32856,
            floor: 7,
        },
    )
    .await?;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(
        progress(h).await?,
        4,
        "a historical paid return cannot complete this journey"
    );
    fixture_move(
        owner,
        admitted,
        MovementLocalPosition {
            x: 32130,
            y: 32913,
            floor: 7,
        },
    )
    .await?;
    let return_trip = conversation(owner, admitted, inbound, &mut command, "liberty_bay").await?;
    assert!(return_trip[1].contains("50 gold"));
    assert_eq!(
        at(owner, admitted).await?,
        MovementLocalPosition {
            x: 32349,
            y: 32856,
            floor: 7
        }
    );
    assert_eq!(fees(h).await?, vec![200, 50, 50]);
    step(owner, admitted, command, StepDirection::West).await?;
    command += 1;
    step(owner, admitted, command, StepDirection::East).await?;
    command += 1;
    assert_eq!(progress(h).await?, 5);
    let complete:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.treasure_island_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(complete);
    let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.treasure_island_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        receipts, 5,
        "one receipt per selected stage, with no item reward callback"
    );
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(|_| "quest cache")?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    step(owner, admitted, command, StepDirection::West).await?;
    command += 1;
    step(owner, admitted, command, StepDirection::East).await?;
    assert_eq!(progress(h).await?, 5);
    assert_eq!(owned(h, 3457).await?, shovel);
    assert_eq!(owned(h, 3003).await?, rope);
    assert_eq!(owned(h, 9205).await?, map);
    assert_eq!(fees(h).await?, vec![200, 50, 50]);
    let remaining:i64=sqlx::query_scalar("SELECT coalesce(sum(i.quantity * CASE i.definition_production_key WHEN 'oteryn:item.tibia.i3031' THEN 1 WHEN 'oteryn:item.tibia.i3035' THEN 100 WHEN 'oteryn:item.tibia.i3043' THEN 10000 END),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid)c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1").bind(id(41).as_slice()).bind(id(42).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        remaining, 100,
        "400 carried gold minus50 historical return,200 outbound and50 current return"
    );
    Ok(())
}
