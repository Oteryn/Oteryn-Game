//! S07 client replay capture: the bed command results a player receives. Character one walks to
//! the authored bed pair and sleeps (OfflineTraining, which releases the session); Character zero
//! tries the same pair (BedOccupied). The raw `CommandResult` payloads the session received are
//! written byte for byte to `s07-sleep.bin` / `s07-occupied.bin`; `s07-expected.txt` records the
//! server-observed dispositions. Character one is re-admitted at the end to wake it.
use super::*;
use super::{
    row_connect_character as connect_character,
    row_finish as row_logout,
};
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::target_move::{WalkDisposition, send};
use oteryn_protocol_oteryn::bed::COMMAND_TYPE_BED_SLEEP_INTENT;
use oteryn_protocol_oteryn::item_view::{ItemHandle, encode_item_target};
use oteryn_protocol_oteryn::world_object::{
    CAPABILITY_ITEM_USE_V1, UseDisposition, decode_use_result,
};

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

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    std::fs::write(Path::new(&root).join(name), bytes)?;

    Ok(())
}

async fn travel(client: &mut DevClientSession) -> TestResult {
    if client.world_spatial().actor_position == APPROACH {
        return Ok(());
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
    bed_handle_from(&snapshot.payload)
}

fn bed_handle_from(payload: &[u8]) -> TestResult<ItemHandle> {
    let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(payload)
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
            CAPABILITY_ITEM_USE_V1,
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

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    row_complete(async {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    let mut sleeper = connect_character(clients, 0x60, 1).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        travel(&mut sleeper).await?;
        let (sleep_payload, slept) = sleep(&mut sleeper).await?;
        if slept != UseDisposition::OfflineTraining {
            return Err(format!("step=sleep observed {slept:?}, expected OfflineTraining").into());
        }
        // OfflineTraining is a terminal server-owned session transition.
        sleeper.ended = true;
        row_needs_logout_0 = false;
        let mut peer = connect_character(clients, 0x61, 0).await?;
        let mut row_needs_logout_1 = true;
        let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
            travel(&mut peer).await?;
            let (occupied_payload, occupied) = sleep(&mut peer).await?;
            tokio::time::timeout(Duration::from_secs(10), row_logout(&mut peer)).await??;
            row_needs_logout_1 = false;
            if occupied != UseDisposition::BedOccupied {
                return Err(
                    format!("step=occupied observed {occupied:?}, expected BedOccupied").into(),
                );
            }
            // Wake the sleeper through an ordinary fresh admission.
            let mut woke = connect_character(clients, 0x62, 1).await?;
            let mut row_needs_logout_2 = true;
            let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
                tokio::time::timeout(Duration::from_secs(10), row_logout(&mut woke)).await??;
                row_needs_logout_2 = false;
                capture("s07-sleep.bin", &sleep_payload)?;
                capture("s07-occupied.bin", &occupied_payload)?;
                capture(
                    "s07-expected.txt",
                    format!("sleep={slept:?}\noccupied={occupied:?}\n").as_bytes(),
                )?;
                Ok(format!("sleep={slept:?} occupied={occupied:?}"))
            })
            .await
            .unwrap_or_else(|error| Err(error.into()));
            if row_needs_logout_2 {
                tokio::time::timeout(Duration::from_secs(10), row_logout(&mut woke)).await??;
            }
            row_result
        })
    .await
    .unwrap_or_else(|error| Err(error.into()));
        if row_needs_logout_1 {
            tokio::time::timeout(Duration::from_secs(10), row_logout(&mut peer)).await??;
        }
        row_result
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_0 {
        tokio::time::timeout(Duration::from_secs(10), row_logout(&mut sleeper)).await??;
    }
    row_result

    }).await?
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_none() {
        return;
    }
    match row_complete(journey(clients)).await {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_map_snapshot_has_no_bed_handle() {
        assert!(bed_handle_from(&[]).is_err());
        assert!(bed_handle_from(&[255]).is_err());
    }
}
