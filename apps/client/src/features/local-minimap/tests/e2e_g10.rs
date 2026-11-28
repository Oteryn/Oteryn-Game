use super::*;
use oteryn_protocol_oteryn::{CharacterId, decode_server_accepted, decode_wire_envelope};
use oteryn_session::{Admission, CLIENT_SUPPORTED_CAPABILITIES, Session, read_frame};
use tokio::io::AsyncWriteExt;

type Error = Box<dyn std::error::Error + Send + Sync>;

fn uuid(text: &str) -> Result<[u8; 16], Error> {
    Ok(u128::from_str_radix(&text.replace('-', ""), 16)?.to_be_bytes())
}

async fn admission(
    path: &Path,
    character: CharacterId,
    world: [u8; 16],
    session_id: [u8; 16],
    sequence: u64,
) -> Result<WorldMapViewSnapshot, Error> {
    let capture = std::fs::read(path)?;
    let accepted = read_frame(&mut capture.as_slice()).await?;
    let envelope = decode_wire_envelope(&accepted)?;
    assert_eq!(*decode_server_accepted(envelope.payload())?.world_id.as_bytes(), world);

    let (client, mut peer) = tokio::io::duplex(64 * 1024);
    let transfer = tokio::spawn(async move {
        read_frame(&mut peer).await?;
        peer.write_all(&capture).await?;
        peer.shutdown().await?;
        Ok::<_, Error>(())
    });
    let session = Session::admit(
        client,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: character,
            admission_material: b"offline-replay",
            client_build_id: "oteryn-client/g10-replay",
            supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
            deadline: Duration::from_secs(5),
        },
    ).await?;
    tokio::time::timeout(Duration::from_secs(5), transfer).await???;
    assert_eq!(*session.join_snapshot().game_session_id.as_bytes(), session_id);
    assert_eq!(session.last_server_sequence(), sequence);
    let map = session.world_map_snapshot().ok_or("missing admitted map")?;
    assert_eq!(map.header.origin, session.world_spatial().actor_position);
    assert!(map.tiles.iter().any(|tile| tile.position == map.header.origin && tile.ground_speed > 0));
    Ok(map)
}

async fn replay() -> Result<(), Error> {
    let capture = PathBuf::from(std::env::var_os("G10_REPLAY_DIR").ok_or("missing run capture")?);
    let character = CharacterId::decode(&uuid(&std::env::var("G10_REPLAY_CHARACTER")?)?)?;
    let world = std::env::var("G10_REPLAY_WORLD")?;
    let entry_session = uuid(&std::env::var("G10_REPLAY_ENTRY_SESSION")?)?;
    let relog_session = uuid(&std::env::var("G10_REPLAY_RELOG_SESSION")?)?;
    assert_ne!(entry_session, relog_session);
    let entry = admission(
        &capture.join("entry.frames"), character, uuid(&world)?, entry_session,
        std::env::var("G10_REPLAY_ENTRY_SEQUENCE")?.parse()?,
    ).await?;
    let cache = tempfile::tempdir_in(&capture)?;
    let origin = entry.header.origin;
    let marker = Marker {
        x: origin.x,
        y: origin.y,
        floor: origin.floor,
        label: "Cyclopedia waypoint".into(),
        color: 2,
    };
    {
        let mut minimap = Minimap::default();
        minimap.root = Some(cache.path().to_owned());
        minimap.bind_world(&world);
        minimap.observe(&entry);
        assert!(minimap.markers.is_empty());
        minimap.edit_marker(marker.x, marker.y, marker.floor, marker.label.clone(), marker.color)?;
        assert_eq!(minimap.markers.as_slice(), std::slice::from_ref(&marker));
    }

    let relog = admission(
        &capture.join("relog.frames"), character, uuid(&world)?, relog_session,
        std::env::var("G10_REPLAY_RELOG_SEQUENCE")?.parse()?,
    ).await?;
    let mut minimap = Minimap::default();
    minimap.root = Some(cache.path().to_owned());
    minimap.bind_world(&world);
    minimap.observe(&relog);
    assert_eq!(minimap.markers.as_slice(), std::slice::from_ref(&marker));
    minimap.remove_marker(marker.x, marker.y, marker.floor)?;
    assert!(minimap.markers.is_empty());
    drop(minimap);
    let mut reopened = Minimap::default();
    reopened.root = Some(cache.path().to_owned());
    reopened.bind_world(&world);
    reopened.observe(&relog);
    assert!(reopened.markers.is_empty());
    println!("G10_MINIMAP_REPLAY position={},{},{} icon={} description={:?} persisted=true removed=true",
        marker.x, marker.y, marker.floor, marker.color, marker.label);
    Ok(())
}

#[test]
#[ignore = "requires both original G10 live admission recordings"]
fn g10_replays_live_admissions_and_persists_exact_marker() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(replay()))?
        .join()
        .map_err(|_| "G10 minimap replay panicked")?
}
