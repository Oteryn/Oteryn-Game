//! chsw item 6: a player on one of two assigned Channels of a World reads the online count of
//! both, switches through the real connection (commands 6251, 6247, 6248) and enters the other
//! Channel by a fresh admission; the receipt counts the switch and starts the cooldown.
use super::chat_world_fixture as world;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::ChannelId;
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::{BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities},
    connection::{AdmittedSession, FirstEntryOutcome, IDLE_LIVENESS, serve_admitted},
};
use oteryn_protocol_oteryn::channel_switch::{
    self as wire, CAPABILITY_CHANNEL_SWITCH_V1, SwitchLock, SwitchStatus,
};
use oteryn_protocol_oteryn::{self as protocol, CommandStatus};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// One command on a new connection with the session's continuity; the command result, if any.
async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    ty: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let frame = protocol::encode_client_command(
        admitted.continuity.connection_generation,
        &protocol::ClientCommandValue {
            command_id: admitted.continuity.next_command_id,
            command_type: ty,
            payload,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.write_u32(u32::try_from(frame.len())?).await?;
    client.write_all(&frame).await?;
    client.shutdown().await?;
    use crate::gameplay_transport::connection::ConnectionEnd;
    match serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await {
        ConnectionEnd::AdmittedThenDisconnected(next)
        | ConnectionEnd::AdmittedThenClosed(next, _) => *admitted = next,
        // An accepted switch ends the session after its result, as a logout does.
        _ => {}
    }
    drop(server);
    let mut bytes = Vec::new();
    client.read_to_end(&mut bytes).await?;
    let mut cursor = bytes.as_slice();
    let mut result = None;
    while !cursor.is_empty() {
        let len = usize::try_from(u32::from_be_bytes(
            cursor.get(..4).ok_or("length")?.try_into()?,
        ))?;
        let frame = cursor.get(4..4 + len).ok_or("frame")?;
        cursor = &cursor[4 + len..];
        let envelope = protocol::decode_wire_envelope(frame).map_err(debug)?;
        if envelope.message_type() == protocol::MessageType::CommandResult {
            let reply = protocol::decode_command_result(envelope.payload()).map_err(debug)?;
            assert!(result.is_none(), "one result per command");
            result = Some((reply.status, reply.payload.to_vec()));
        }
    }
    result.ok_or_else(|| "missing command result".into())
}

/// The remembered Channel, the switch count and whether a cooldown runs.
async fn guard(h: &Harness) -> TestResult<(Vec<u8>, i64, bool)> {
    Ok(sqlx::query_as(
        "SELECT uuid_send(g.last_channel_id), g.switch_count, s.cooldown_left_ms > 0 \
         FROM game_channel_switch_guards g, \
              game_channel_switch_status(encode($1,'hex')::uuid, encode($2,'hex')::uuid) s \
         WHERE g.character_id=encode($1,'hex')::uuid AND g.world_id=encode($2,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .bind(id(42).as_slice())
    .fetch_one(&h.pool)
    .await?)
}

async fn session_state(h: &Harness, session: [u8; 16]) -> TestResult<i16> {
    Ok(sqlx::query_scalar(
        "SELECT session_state FROM game_durability_reconnect_sessions \
         WHERE game_session_id=encode($1,'hex')::uuid",
    )
    .bind(session.as_slice())
    .fetch_one(&h.pool)
    .await?)
}

#[test]
fn a_player_switches_between_two_live_channels_and_reenters_the_other_one() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .name("channel-switch-two-channels".into())
        .stack_size(64 << 20)
        .spawn(|| {
            run(async |admin| {
                reset();
                let h = Harness::create_with_admission_seed(
                    admin,
                    "channel_switch_two_channels",
                    true,
                    false,
                )
                .await?;
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                // Channel 43 on the harness node: Character 41, positioned and leased.
                let mut first = Dependencies::new()?;
                let entry = first.room.entry_start();
                let logout_cell = crate::foundation::MovementLocalPosition {
                    x: entry.x,
                    y: entry.y,
                    floor: entry.floor,
                };
                let pin = crate::content::channel_content_pin::ChannelContentPin::from_activation(
                    first.admitted.world_id,
                    1,
                    first.room.movement_cells().scope().generation_digest,
                    [2; 32],
                    first.room.frame_binding().digest(),
                    first.room.map_revision_digest(),
                    (entry.x + 10, entry.y, entry.floor),
                );
                let mut fence = super::fixture::fence(1)?.item_fence();
                fence.character_lease_generation = 2;
                first.admitted.item_fence = Some(fence);
                {
                    let mut runtime = first.runtime.lock().await;
                    runtime.replace_encounter_fixture_content(pin.clone());
                    runtime
                        .initialize_pinned_test_position(first.actor, logout_cell)
                        .map_err(debug)?;
                    runtime
                        .bind_attacker_lease(
                            first.actor,
                            first.admitted.game_session_id,
                            crate::foundation::CharacterLease::new(
                                crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                                2,
                            )
                            .map_err(debug)?,
                        )
                        .map_err(debug)?;
                }
                first.admitted.first_entry = FirstEntryOutcome::Positioned;
                // Channel 44 on a second node of the same World, with its own admitted player.
                let (node, second) = world::admit_second_node(&h).await?;
                let mut owner = first.owner(&h, &authority);
                first.initialize(&owner).await;
                let mut position_fence = super::fixture::fence(super::revision(&h).await?)?;
                position_fence.character_lease_generation = fence.character_lease_generation;
                assert!(owner.track_restored(first.admitted.game_session_id, first.actor,
                    &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin, position_fence));
                // The fixture entered long enough ago that no fresh-login fight block holds.
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(Duration::from_secs(120))
                    .ok_or("clock")?;
                let mut admitted = first.admitted;
                admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                    BUNDLE_WORLD_OFFERED_CAPABILITIES,
                    &[CAPABILITY_CHANNEL_SWITCH_V1],
                )
                .ok_or("CHANNEL_SWITCH_V1 selection")?;
                let here = id(43);
                let there = id(44);

                // 6251: both Channels of the World, each with its one current player.
                let (status, load) =
                    exchange(&owner, &mut admitted, wire::COMMAND_TYPE_CHANNEL_LOAD, &[]).await?;
                assert_eq!(status, CommandStatus::Accepted);
                let mut expected = vec![(here, 1), (there, 1)];
                expected.sort_unstable();
                assert_eq!(wire::decode_load(&load), Some(expected));

                // 6247: nothing blocks a switch and no cooldown runs before the first one.
                let (status, ready) = exchange(
                    &owner,
                    &mut admitted,
                    wire::COMMAND_TYPE_CHANNEL_SWITCH_STATUS,
                    &[],
                )
                .await?;
                assert_eq!(status, CommandStatus::Accepted);
                assert_eq!(wire::decode_status(&ready), Some(SwitchStatus::READY));

                // 6248 to the current Channel is refused and changes nothing.
                let (status, same) = exchange(
                    &owner,
                    &mut admitted,
                    wire::COMMAND_TYPE_CHANNEL_SWITCH_REQUEST,
                    &wire::encode_request(here),
                )
                .await?;
                assert_eq!(status, CommandStatus::Rejected);
                assert_eq!(
                    wire::decode_status(&same).map(|status| status.lock),
                    Some(SwitchLock::SameChannel)
                );
                assert!(matches!(session_state(&h, id(50)).await?, 1 | 2));

                let configured = sqlx::query("UPDATE game_channel_pvp_types SET player_capacity=1 WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid")
                    .bind(id(42).as_slice()).bind(there.as_slice()).execute(&h.pool).await?;
                assert_eq!(configured.rows_affected(), 1);
                let (status, full) = exchange(&owner, &mut admitted,
                    wire::COMMAND_TYPE_CHANNEL_SWITCH_REQUEST, &wire::encode_request(there)).await?;
                assert_eq!(status, CommandStatus::Rejected);
                assert_eq!(wire::decode_status(&full).map(|s| s.lock), Some(SwitchLock::ChannelFull));
                assert!(matches!(session_state(&h, id(50)).await?, 1 | 2));
                assert_eq!(guard(&h).await?, (here.to_vec(), 0, false));
                sqlx::query("SELECT game_chat_role_issue(encode($1,'hex')::uuid,0,4::smallint,4::smallint,true,game_character_uuid_v7(),'channel-capacity')")
                    .bind(id(41).as_slice()).execute(&h.pool).await?;

                // 6248 to the other Channel: the safe exit commits and the session ends.
                let (status, switched) = exchange(
                    &owner,
                    &mut admitted,
                    wire::COMMAND_TYPE_CHANNEL_SWITCH_REQUEST,
                    &wire::encode_request(there),
                )
                .await?;
                assert_eq!(status, CommandStatus::Accepted);
                assert_eq!(wire::decode_status(&switched), Some(SwitchStatus::READY));
                assert_eq!(
                    session_state(&h, id(50)).await?,
                    3,
                    "the safe exit ended it"
                );
                assert_eq!(guard(&h).await?, (here.to_vec(), 0, false));

                // The client enters Channel 44 by a fresh admission: that is the switch.
                let (entered, request) = super::full_world_support::relogin_into(
                    &h,
                    id(41),
                    id(40),
                    id(111),
                    Some(ChannelId::decode(&there).map_err(debug)?),
                )
                .await?;
                let destination = second.owner_from(&h.root, &node, &authority);
                let actor = {
                    let mut runtime = second.runtime.lock().await;
                    runtime.replace_encounter_fixture_content(pin);
                    let reservation = runtime.reserve_fresh_session(entered).map_err(debug)?;
                    runtime.commit_fresh_session(reservation).map_err(debug)?
                };
                let (placement, _, _) = destination
                    .initialize_first_entry(
                        &request,
                        entered,
                        request.operation().authorization.transport,
                        actor,
                    )
                    .await;
                assert_eq!(placement, FirstEntryOutcome::Positioned);
                assert_eq!(
                    second
                        .runtime
                        .lock()
                        .await
                        .read_actor_position(actor)
                        .map_err(debug)?
                        .position(),
                    logout_cell
                );
                assert!(matches!(
                    session_state(&h, *entered.as_bytes()).await?,
                    1 | 2
                ));
                let channel: Vec<u8> = sqlx::query_scalar(
                    "SELECT uuid_send(COALESCE(runtime_scope_channel_id, origin_channel_id)) \
                     FROM game_durability_reconnect_sessions \
                     WHERE game_session_id=encode($1,'hex')::uuid",
                )
                .bind(entered.as_bytes().as_slice())
                .fetch_one(&h.pool)
                .await?;
                assert_eq!(channel, there.to_vec(), "the new session is on Channel 44");
                assert_eq!(
                    guard(&h).await?,
                    (there.to_vec(), 1, true),
                    "the admission counted the switch and started the cooldown"
                );

                // A second switch inside the cooldown, back to 43: the durable preflight that
                // 6248 reads refuses it for the session now current on Channel 44.
                let (connection, lease, ownership): (String, String, String) = sqlx::query_as(
                    "SELECT current_generation::text, character_lease_generation::text, \
                     scope_ownership_generation::text FROM game_durability_reconnect_sessions \
                     WHERE game_session_id=encode($1,'hex')::uuid",
                )
                .bind(entered.as_bytes().as_slice())
                .fetch_one(&h.pool)
                .await?;
                let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
                let fence = crate::durability::item_transfer::CurrentCharacterItemFence {
                    character_id: crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
                    game_session_id: entered,
                    connection_generation: crate::foundation::ConnectionGeneration::new(
                        connection.parse()?,
                    )
                    .map_err(debug)?,
                    character_lease_generation: lease.parse()?,
                    runtime_scope: crate::foundation::RuntimeScopeRefV1::channel(
                        world_id,
                        ChannelId::decode(&there).map_err(debug)?,
                    ),
                    scope_ownership_generation: crate::foundation::ScopeOwnershipGeneration::new(
                        ownership.parse()?,
                    )
                    .map_err(debug)?,
                };
                let again = h
                    .root
                    .read_channel_switch_preflight(&authority, &node, fence, id(42), Some(here))
                    .await
                    .map_err(debug)?
                    .ok_or("the re-entered session is current")?;
                assert_eq!(again.lock, SwitchLock::Cooldown, "{again:?}");
                assert!(again.cooldown_left_ms > 0, "{again:?}");
                drop(owner);
                drop(authority);
                drop(seal);
                h.cleanup().await.map_err(debug)?;
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "channel switch two channels thread panicked")?
        .map_err(Into::into)
}
