//! S07 client replay capture: the bed command results a player receives. Character one walks to
//! the authored bed pair and sleeps (OfflineTraining, which releases the session); Character zero
//! tries the same pair (BedOccupied). The raw `CommandResult` payloads the session received are
//! written byte for byte to `s07-sleep.bin` / `s07-occupied.bin`; `s07-expected.txt` records the
//! server-observed dispositions. Character one is re-admitted at the end to wake it.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::target_move::{WalkDisposition, send};
use oteryn_protocol_oteryn::bed::COMMAND_TYPE_BED_SLEEP_INTENT;
use oteryn_protocol_oteryn::item_view::{ItemHandle, encode_item_target};
use oteryn_protocol_oteryn::world_object::{UseDisposition, decode_use_result};

const BED: ActorPosition = ActorPosition {
    x: 32311,
    y: 32804,
    floor: 7,
};
const APPROACH: ActorPosition = ActorPosition {
    x: 32312,
    y: 32804,
    floor: 7,
};

fn capture(root: &Path, name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write;

    let path = root.join(name);
    let mut file = std::fs::File::create_new(&path)
        .map_err(|error| format!("step=capture file={name} create={error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("step=capture file={name} write={error}"))?;
    Ok(())
}

async fn travel(client: &mut DevClientSession) -> TestResult {
    if client.world_spatial().actor_position == APPROACH {
        return Err("step=walk already at the bed approach before the walk".into());
    }
    let reply = send(client.session_mut(), APPROACH).await?;
    if reply.status != CommandStatus::Accepted
        || reply.disposition != Some(WalkDisposition::Accepted)
    {
        return Err(format!(
            "step=walk status={:?} disposition={:?}",
            reply.status, reply.disposition
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
        if client.world_spatial().actor_position == APPROACH {
            return Ok(());
        }
    }
    Err("step=walk stopped before the bed approach".into())
}

fn bed_handle(client: &mut DevClientSession) -> TestResult<ItemHandle> {
    let snapshot = client
        .session_mut()
        .gated_snapshot(oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or("step=bed-handle missing world map")?;
    let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
        .map_err(|error| format!("step=bed-handle decode {error:?}"))?;
    map.tiles
        .iter()
        .find(|tile| tile.position == BED)
        .and_then(|tile| {
            tile.items.iter().find_map(|item| match item.origin {
                oteryn_protocol_oteryn::world_map::MapOrigin::Handle(handle) => Some(handle),
                _ => None,
            })
        })
        .ok_or_else(|| "step=bed-handle authored bed has no live item handle".into())
}

/// Sleeps on the bed with magic-level training and returns the raw reply payload plus its
/// decoded disposition.
async fn sleep(client: &mut DevClientSession) -> TestResult<(Vec<u8>, UseDisposition)> {
    let target = encode_item_target(bed_handle(client)?);
    let mut payload = vec![0x0a, u8::try_from(target.len())?];
    payload.extend(target);
    payload.extend([0x10, 6]);
    let reply = client
        .session_mut()
        .gated_command(
            oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1,
            COMMAND_TYPE_BED_SLEEP_INTENT,
            &payload,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=sleep status={:?}", reply.status).into());
    }
    let disposition = decode_use_result(&reply.payload)
        .map_err(|error| format!("step=sleep decode {error:?}"))?;
    Ok((reply.payload, disposition))
}

async fn journey(clients: &SeamClients<'_>, capture_dir: &Path) -> TestResult<String> {
    let mut sleeper = connect_character(clients, 0x60, 1).await?;
    travel(&mut sleeper).await?;
    let (sleep_payload, slept) = sleep(&mut sleeper).await?;
    if slept != UseDisposition::OfflineTraining {
        return Err(format!("step=sleep observed {slept:?}, expected OfflineTraining").into());
    }
    let mut peer = connect_character(clients, 0x61, 0).await?;
    travel(&mut peer).await?;
    let (occupied_payload, occupied) = sleep(&mut peer).await?;
    let _ = peer.session_mut().logout().await;
    if occupied != UseDisposition::BedOccupied {
        return Err(format!("step=occupied observed {occupied:?}, expected BedOccupied").into());
    }
    // Wake the sleeper through an ordinary fresh admission.
    let mut woke = connect_character(clients, 0x62, 1).await?;
    let _ = woke.session_mut().logout().await;
    capture(capture_dir, "s07-sleep.bin", &sleep_payload)?;
    capture(capture_dir, "s07-occupied.bin", &occupied_payload)?;
    capture(
        capture_dir,
        "s07-expected.txt",
        format!("sleep={slept:?}\noccupied={occupied:?}\n").as_bytes(),
    )?;
    Ok(format!("sleep={slept:?} occupied={occupied:?}"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let Some(capture_dir) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        evidence("e2e_all scenario=S07.client status=FAIL evidence=missing-capture-directory");
        return;
    };
    match verdict::indexed_timeout(
        Duration::from_secs(180),
        verdict::observe(journey(clients, Path::new(&capture_dir))),
    )
    .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=S07.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S07.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=S07.client status=FAIL evidence=timeout"),
    }
}

#[test]
fn s07_capture_refuses_to_overwrite_an_existing_file() -> TestResult {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let root = std::env::temp_dir().join(format!("oteryn-s07-{}-{stamp}", std::process::id()));
    std::fs::create_dir(&root)?;
    let result = (|| -> TestResult {
        capture(&root, "s07-sleep.bin", b"first")?;
        assert!(capture(&root, "s07-sleep.bin", b"second").is_err());
        assert_eq!(std::fs::read(root.join("s07-sleep.bin"))?, b"first");
        assert!(capture(&root, "missing/s07-sleep.bin", b"first").is_err());
        Ok(())
    })();
    std::fs::remove_dir_all(root)?;
    result
}
