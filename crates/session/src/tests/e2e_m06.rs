//! M06 replay owns domain-2 client state. The raw server payload is consumed by
//! `Session::apply_state_delta` (`domain_dispatch.rs`) and observed through
//! `Session::world_object_overlay` (`session_access.rs`); `DevClientSession::world_object_overlay`
//! (`tools/dev-client/src/lib.rs`) delegates to that store.

use super::*;
use oteryn_protocol_oteryn::world_object::{self, WorldObjectOverlayEntry};

#[test]
fn server_key_door_overlay_replays_into_client_store() -> Result<(), BoxError> {
    block_on(async {
        let (payload, metadata, placement, state) =
            if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
                let root = std::path::Path::new(&root);
                let payload = std::fs::read(root.join("m06-overlay.bin"))?;
                let placement = std::fs::read(root.join("m06-placement.bin"))?;
                let expected = std::fs::read_to_string(root.join("m06-expected.txt"))?;
                let mut lines = expected.lines();
                let metadata = lines.next().ok_or("M06 missing metadata")?.to_owned();
                let state = lines.next().ok_or("M06 missing state")?.as_bytes().to_vec();
                (payload, metadata, placement, state)
            } else {
                let entry = WorldObjectOverlayEntry {
                    content_generation: CONTENT_GENERATION,
                    placement: b"m06:key-door".to_vec(),
                    state: b"oteryn:reference.state.open".to_vec(),
                    revision: 3,
                };
                (
                    world_object::encode_world_object_overlay_delta(&entry)
                        .map_err(|error| format!("{error:?}"))?,
                    "42 2 3 3".into(),
                    entry.placement,
                    entry.state,
                )
            };
        let numbers = metadata
            .split_whitespace()
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()?;
        let [sequence, base, new, entry_revision] = numbers.as_slice() else {
            return Err("M06 invalid metadata".into());
        };
        let entry = world_object::decode_world_object_overlay_delta(&payload)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(entry.revision, *entry_revision);

        let (client, peer) = tokio::io::duplex(64 * 1024);
        let server = tokio::spawn(scripted_server(peer));
        let mut session = Session::admit(client, admission()?).await?;
        session.world_spatial.content_generation = entry.content_generation;
        session.overlay_revision = *base;
        session.world_object_overlay.clear();
        let wire = encode_state_delta(
            1,
            *sequence,
            world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            *base,
            *new,
            world_object::DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
            &payload,
        )?;
        let envelope = decode_wire_envelope(&wire)?;
        let event = session.apply_state_delta(*sequence, envelope.payload())?;
        assert!(matches!(event, SessionEvent::WorldObjectOverlay(_)));
        assert_eq!(session.world_object_overlay().len(), 1);
        assert_eq!(session.world_object_overlay()[0].placement, placement);
        assert_eq!(session.world_object_overlay()[0].state, state);

        let before = session.world_object_overlay().to_vec();
        let invalid = encode_state_delta(
            1,
            sequence.saturating_add(1),
            world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            *new,
            new.saturating_add(1),
            world_object::DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
            &[0xff],
        )?;
        let invalid = decode_wire_envelope(&invalid)?;
        assert!(
            session
                .apply_state_delta(sequence.saturating_add(1), invalid.payload())
                .is_err()
        );
        assert_eq!(session.world_object_overlay(), before);
        server.abort();
        Ok(())
    })?
}
