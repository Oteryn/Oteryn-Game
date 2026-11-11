//! S07 full-row player scenario, derived from the S07 matrix/audit row and the
//! production owner in `gameplay_transport/bed_sleep_production.rs`:
//! 1. Walk an ordinarily admitted, Premium character into its House and beside the
//!    reciprocal bed pair; obtain the bed's session-local handle from WORLD_MAP.
//! 2. Send BED_SLEEP_INTENT (magic-level training) and require an Accepted command
//!    carrying OfflineTraining; this proves ACL approval, canonical pair reservation,
//!    saved rest/position, and the sleep-triggered session release.
//! 3. Admit the other provisioned character, walk it to the same pair, send the same
//!    player command, and require BedOccupied without a logout or reservation delta.
//! 4. Freshly admit the sleeper (the wake action), require its new session and world
//!    snapshot, then use the same bed again. OfflineTraining proves the prior durable
//!    reservation was released and that sleep/wake state survived logout/admission.
use super::*;
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

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(180), journey(clients)).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err("step-1:timeout-walking-to-authored-bed".into()),
    };
    match result {
        Ok(()) => evidence(&format!(
            "e2e_all scenario=S07.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Err(error) => {
            let detail = error.to_string().replace(' ', "-");
            evidence(&format!("S07_GAP {error}"));
            evidence(&format!(
                "e2e_all scenario=S07.full status=FAIL seconds={:.3} evidence={detail}",
                started.elapsed().as_secs_f64()
            ));
        }
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut sleeper = row_connect_character(clients, 0xb7, 1).await?;
    travel(&mut sleeper, "step-1").await?;
    let handle = bed_handle(&mut sleeper, "step-1")?;
    let saved_position = sleeper.world_spatial().actor_position;
    let first = sleep(&mut sleeper, handle, "step-2").await?;
    if first != UseDisposition::OfflineTraining {
        return Err(format!("step-2:sleep-observed-{first:?}-need-OfflineTraining").into());
    }
    let release = sleeper.session_mut().service_liveness(Duration::from_secs(2)).await;
    if !matches!(&release, Err(error)
        if error.to_string().starts_with("transport I/O failed: "))
    {
        return Err(
            format!("step-2:sleep-release-observed-{release:?}-need-transport-close").into(),
        );
    }

    sleeper.ended = true;
    let mut peer = row_connect_character(clients, 0xb8, 0).await?;
    travel(&mut peer, "step-3").await?;
    let peer_bed = bed_handle(&mut peer, "step-3")?;
    let peer_position = peer.world_spatial().actor_position;
    peer.take_events();
    let occupied = sleep(&mut peer, peer_bed, "step-3").await?;
    if occupied != UseDisposition::BedOccupied {
        return Err(
            format!("step-3:reservation-refusal-observed-{occupied:?}-need-BedOccupied").into(),
        );
    }
    let liveness = peer.service_liveness(Duration::from_millis(100)).await;
    if liveness.is_err() || peer.world_spatial().actor_position != peer_position {
        return Err(format!(
            "step-3:occupied-refusal-observed-liveness-{liveness:?}-position-{:?}-need-live-session-position-{peer_position:?}",
            peer.world_spatial().actor_position
        )
        .into());
    }
    logout(&mut peer, "step-3").await?;

    let mut woke = row_connect_character(clients, 0xb9, 1).await?;
    if woke.join_snapshot().game_session_id == sleeper.join_snapshot().game_session_id {
        return Err("step-4:fresh-admission-reused-sleep-session".into());
    }
    if woke.world_spatial().actor_position != saved_position {
        return Err(format!(
            "step-4:saved-position-observed-{:?}-need-{saved_position:?}",
            woke.world_spatial().actor_position
        )
        .into());
    }
    let woke_bed = bed_handle(&mut woke, "step-4")?;
    let again = sleep(&mut woke, woke_bed, "step-4").await?;
    if again != UseDisposition::OfflineTraining {
        return Err(format!("step-4:wake-release-observed-{again:?}-need-OfflineTraining").into());
    }
    let release = woke.session_mut().service_liveness(Duration::from_secs(2)).await;
    if !matches!(&release, Err(error)
        if error.to_string().starts_with("transport I/O failed: "))
    {
        return Err(format!("step-4:sleep-release-observed-{release:?}-need-transport-close").into());
    }
    woke.ended = true;
    Ok(())
}

async fn travel(client: &mut DevClientSession, step: &str) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    if client.world_spatial().actor_position == APPROACH {
        return Err(
            format!("{step}:walk-observed-already-at-approach-no-required-command-ran").into(),
        );
    }
    let reply = send(client.session_mut(), APPROACH).await?;
    if reply.status != CommandStatus::Accepted
        || reply.disposition != Some(WalkDisposition::Accepted)
    {
        return Err(format!(
            "{step}:walk-to-bed-observed-status-{:?}-disposition-{:?}-need-house-route-and-ACL",
            reply.status, reply.disposition
        )
        .into());
    }
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
        if client.world_spatial().actor_position == APPROACH {
            return Ok(());
        }
    }
    Err(format!(
        "{step}:walk-accepted-but-stopped-at-{:?}-need-reachable-house-route",
        client.world_spatial().actor_position
    )
    .into())
}

fn bed_handle(client: &mut DevClientSession, step: &str) -> TestResult<ItemHandle> {
    let snapshot = client
        .session_mut()
        .gated_snapshot(oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or_else(|| format!("{step}:missing-world-map-at-bed"))?;
    let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
        .map_err(|error| format!("{step}:bed-map-decode-{error:?}"))?;
    map.tiles
        .iter()
        .find(|tile| tile.position == BED)
        .and_then(|tile| {
            tile.items.iter().find_map(|item| match item.origin {
                oteryn_protocol_oteryn::world_map::MapOrigin::Handle(handle) => Some(handle),
                _ => None,
            })
        })
        .ok_or_else(|| format!("{step}:authored-bed-published-no-live-item-handle").into())
}

async fn sleep(
    client: &mut DevClientSession,
    handle: ItemHandle,
    step: &str,
) -> TestResult<UseDisposition> {
    let target = encode_item_target(handle);
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
        return Err(format!(
            "{step}:bed-command-observed-status-{:?}-payload-{:02x?}",
            reply.status, reply.payload
        )
        .into());
    }
    decode_use_result(&reply.payload)
        .map_err(|error| format!("{step}:bed-command-reply-decode-{error:?}").into())
}

async fn logout(client: &mut RowClient, step: &str) -> TestResult {
    let reply = client.row_logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("{step}:logout-observed-{:?}", reply.result).into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bed_payload_round_trips_through_the_native_decoder() -> TestResult {
        let handle = ItemHandle::new(41).ok_or("handle")?;
        let target = encode_item_target(handle);
        let mut payload = vec![0x0a, u8::try_from(target.len())?];
        payload.extend(target);
        payload.extend([0x10, 6]);
        let decoded = oteryn_protocol_oteryn::bed::decode_bed_sleep_intent(&payload)
            .map_err(|error| format!("bed payload: {error:?}"))?;
        assert_eq!(
            decoded,
            oteryn_protocol_oteryn::bed::BedSleepIntent {
                target: handle,
                skill: 6
            }
        );
        Ok(())
    }
}
