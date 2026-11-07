//! M05 replay: `domain_dispatch.rs::Session::apply_state_delta` installs door overlay deltas,
//! and `session_access.rs::Session::world_object_overlay` exposes the resulting client state.

use super::*;
use oteryn_protocol_oteryn::world_object::{
    DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1, WorldObjectOverlayEntry, encode_world_object_overlay_delta,
};

#[test]
fn real_m05_door_payloads_replay_through_session_store() -> Result<(), BoxError> {
    block_on(async {
        let (payloads, expected) = if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let path = std::path::Path::new(&path);
            (
                ["basic-open", "basic-closed", "key-open", "key-closed"]
                    .map(|name| std::fs::read(path.join(format!("m05-{name}.bin"))))
                    .into_iter()
                    .collect::<Result<Vec<_>, _>>()?,
                std::fs::read_to_string(path.join("m05-expected.txt"))?,
            )
        } else {
            let entry = |placement: &[u8], state: &[u8], revision| WorldObjectOverlayEntry {
                content_generation: CONTENT_GENERATION,
                placement: placement.to_vec(),
                state: state.to_vec(),
                revision,
            };
            (
                [
                    entry(b"m05:basic", b"open", 1),
                    entry(b"m05:basic", b"closed", 2),
                    entry(b"m05:key", b"open", 3),
                    entry(b"m05:key", b"closed", 4),
                ]
                .iter()
                .map(encode_world_object_overlay_delta)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("M05 fixture: {error:?}"))?,
                "open\nclosed\nopen\nclosed\n".into(),
            )
        };
        let entries = payloads
            .iter()
            .map(|payload| world_object::decode_world_object_overlay_delta(payload))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("M05 payload: {error:?}"))?;
        let initial_revision = entries[0]
            .revision
            .checked_sub(1)
            .ok_or("zero M05 open revision")?;

        let (client, server) = tokio::io::duplex(64 * 1024);
        let server_task = tokio::spawn(async move {
            let mut server = server;
            let _ = read_frame(&mut server).await?;
            write_frame(
                &mut server,
                &encode_server_accepted(&ServerAcceptedValue {
                    game_session_id: GameSessionId::decode(&uuid_v7(21))?,
                    world_id: WorldId::decode(&uuid_v7(22))?,
                    channel_id: ChannelId::decode(&uuid_v7(23))?,
                    connection_generation: 1,
                    current_server_sequence: 0,
                    next_command_id: 1,
                    schema_revision: 1,
                    selected_capabilities: &[],
                })?,
            )
            .await?;
            let spatial_payload = spatial(0);
            let overlay_payload = world_object::encode_world_object_overlay_snapshot(&[])
                .map_err(|error| format!("overlay fixture: {error:?}"))?;
            for frame in encode_single_chunk_snapshot(
                1,
                1,
                40,
                &[
                    DomainSnapshot {
                        domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                        revision: 0,
                        snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                        payload: &spatial_payload,
                    },
                    DomainSnapshot {
                        domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                        revision: initial_revision,
                        snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                        payload: &overlay_payload,
                    },
                ],
            )? {
                write_frame(&mut server, &frame).await?;
            }
            Ok::<_, BoxError>(())
        });
        let mut session = Session::admit(client, admission()?).await?;
        server_task.await??;

        let frame = |base, new, payload: &[u8]| {
            encode_state_delta(
                1,
                41 + new,
                STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                base,
                new,
                DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload,
            )
        };
        let mut states = String::new();
        let mut base = initial_revision;
        for (index, (entry, payload)) in entries.iter().zip(&payloads).enumerate() {
            let delta = frame(base, entry.revision, payload)?;
            session.apply_state_delta(
                41 + u64::try_from(index)?,
                decode_wire_envelope(&delta)?.payload(),
            )?;
            let visible = session
                .world_object_overlay()
                .iter()
                .find(|visible| visible.placement == entry.placement)
                .ok_or("M05 entry not visible")?;
            states.push_str(&format!("{}\n", String::from_utf8_lossy(&visible.state)));
            base = entry.revision;
        }
        assert_eq!(states, expected);
        assert_eq!(session.world_object_overlay().len(), 2);

        let before = session.world_object_overlay().to_vec();
        let invalid = frame(initial_revision, base + 1, &payloads[3])?;
        assert!(
            session
                .apply_state_delta(43, decode_wire_envelope(&invalid)?.payload())
                .is_err()
        );
        assert_eq!(session.world_object_overlay(), before);
        Ok(())
    })?
}
