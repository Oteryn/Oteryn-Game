//! Replay the genuine source-Hare join; native popup positions use the map plane.
use super::*;
use oteryn_session::{Admission, CharacterId, EntityKind};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
type TestError = Box<dyn std::error::Error + Send + Sync>;
fn hex(value: &str) -> Result<Vec<u8>, TestError> {
    if !value.len().is_multiple_of(2) {
        return Err("odd captured hex".into());
    }
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
        .collect()
}
#[test]
fn srv7_actual_hare_native_selection_keeps_server_identity_and_map_plane() -> Result<(), TestError>
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run())
}
async fn run() -> Result<(), TestError> {
    let capture: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../apps/client/src/features/local-world-view/tests/halloween-hare-body-wire.json"
    )))?;
    let initial = hex(capture["initial_frames_hex"]
        .as_str()
        .ok_or("actual captured frames")?)?;
    let character = CharacterId::decode(&hex(capture["source"]["character_id_hex"]
        .as_str()
        .ok_or("actual observer character")?)?)?;
    let (client, mut server) = tokio::io::duplex(1 << 20);
    let (done, wait) = tokio::sync::oneshot::channel::<()>();
    let peer = tokio::spawn(async move {
        let n = usize::try_from(server.read_u32().await?)?;
        if n > 1 << 20 {
            return Err("bounded actual bootstrap".into());
        }
        let mut bootstrap = vec![0; n];
        server.read_exact(&mut bootstrap).await?;
        server.write_all(&initial).await?;
        let _ = wait.await;
        Ok::<(), TestError>(())
    });
    let session = Session::admit(
        client,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: character,
            admission_material: b"srv7-read-only-source-frame-play",
            client_build_id: "srv7-native-play",
            supported_capabilities: oteryn_session::CLIENT_SUPPORTED_CAPABILITIES,
            deadline: std::time::Duration::from_secs(3),
        },
    )
    .await?;
    assert!(session.selected_capabilities().contains(&25));
    let map = session.world_map_snapshot().ok_or("captured native map")?;
    assert_eq!(map.header.origin.floor, -7);
    let hare = *session
        .world_entities()
        .ok_or("captured world entities")?
        .others()
        .find(|e| e.kind == EntityKind::Creature && e.position.x == 32624 && e.position.y == 32018)
        .ok_or("actual source Hare")?;
    assert_eq!(
        hare.position.floor, 7,
        "server observation remains unchanged"
    );
    let mut position = hare.position;
    position.floor = map.header.origin.floor;
    let binding = (
        map.header.content_generation,
        map.header.bundle_digest,
        map.header.reset_epoch,
    );
    let command = Command {
        target: Target::Creature(hare.entity),
        action: Action::Look,
        selection: Some(Selection {
            position,
            binding: Some(binding),
        }),
    };
    let encoded = look_payload(&command.target)?;
    let decoded =
        item_look::decode_item_look_intent_for(&encoded, ItemLookSelection { extended: true })
            .map_err(|_| "actual creature LOOK encoding")?;
    assert_eq!(
        decoded.target,
        ItemLookTarget::Creature(hare.entity),
        "server receives its exact identity and generation, without rewriting a wire floor"
    );
    let mut wrong_floor = command.clone();
    wrong_floor
        .selection
        .as_mut()
        .ok_or("selection")?
        .position
        .floor = -8;
    assert!(!super::state::current(&session, &wrong_floor));
    let mut wrong_generation = command.clone();
    wrong_generation.target = Target::Creature(EntityRef {
        generation: hare.entity.generation + 1,
        ..hare.entity
    });
    assert!(!super::state::current(&session, &wrong_generation));
    let mut wrong_binding = command.clone();
    wrong_binding.selection.as_mut().ok_or("selection")?.binding =
        Some((binding.0, binding.1, binding.2 + 1));
    assert!(!super::state::current(&session, &wrong_binding));
    assert!(
        super::state::current(&session, &command),
        "current native Creature LOOK must accept the real projected map plane"
    );
    let _ = done.send(());
    peer.await??;
    Ok(())
}
