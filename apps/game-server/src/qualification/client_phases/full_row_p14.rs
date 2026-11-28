//! P14: offline statue/book training, exercised through the native player protocol.
//!
//! Player scenario derived from the P14 matrix/audit row and the dispatch owner:
//! 1. Log in and verify that the server offered training capability 54.
//! 2. Send OFFLINE_TRAINING with a nonexistent handle; expect an accepted command frame carrying
//!    the explicit refused training result and no logout or offline-training state delta.
//! 3. Walk through the compiled world to an authored offline-training statue, obtain its live
//!    server-issued handle, and send OFFLINE_TRAINING; expect disposition 3, a nonzero durable
//!    character revision and pool, followed by the server ending the session.
//! 4. Make a fresh admission after that logout and query General Stats; expect a new game session
//!    and the durable offline-training pool (with the first ten minutes excluded from accrual) to
//!    remain present.
//! 5. Find and use the donor training book (11441); require its instructional dialog.
use super::*;
use oteryn_dev_client::{DevClientError, DevClientSession};
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::loop_training::{
    CAPABILITY_TRAINING_V1, COMMAND_TYPE_OFFLINE_TRAINING, TrainingResult,
};
use oteryn_protocol_oteryn::world_map::{MapDefinition, MapOrigin};
use std::num::{NonZeroU32, NonZeroU64};

const BOOK: &str = "oteryn:item.tibia.i11441";

const STATUES: [&str; 6] = [
    "oteryn:item.tibia.i16198",
    "oteryn:item.tibia.i16199",
    "oteryn:item.tibia.i16200",
    "oteryn:item.tibia.i16201",
    "oteryn:item.tibia.i16202",
    "oteryn:item.tibia.i50296",
];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients)))
            .await;
    let result = match result {
        Ok(result) => result,
        Err(_) => {
            Err("step=journey observed=timeout-180s need=reachable-statue-and-replies".into())
        }
    };
    if let Err(error) = &result {
        evidence(&format!("P14_GAP {error}"));
    }
    evidence(&format!(
        "e2e_all scenario=P14.full status={} seconds={:.3}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64()
    ));
}

fn matches_short_offline_pool(seconds: u32, fresh_minutes: u16) -> bool {
    fresh_minutes != 0 && u32::from(fresh_minutes) == seconds / 60
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let index = provisioning::item_index()?;
    let references: Vec<NonZeroU32> = STATUES
        .iter()
        .map(|key| {
            index
                .definition_ref("Item", key, "definition-r1")
                .ok_or_else(|| {
                    format!("step=locate-statue observed=missing-definition:{key}").into()
                })
        })
        .collect::<TestResult<_>>()?;
    let positions = placement_positions(&bytes, &references)?;
    let mut client = connect_world(clients, 0xb4).await?;
    let offered = client
        .selected_capabilities()
        .contains(&CAPABILITY_TRAINING_V1);

    let before_session = client.join_snapshot().game_session_id;
    let before_pool = general_stats(&mut client).await?;
    let refused = training(&mut client, NonZeroU64::MAX).await?;
    if !offered
        || refused != TrainingResult::REFUSED
        || client.join_snapshot().game_session_id != before_session
        || general_stats(&mut client).await? != before_pool
    {
        return Err(format!(
            "step=refusal observed=capability-offered:{offered},reply:{refused:?},pool-before:{before_pool},pool-after:{:?} need=capability-54-and-refused-result-without-state-delta",
            general_stats(&mut client).await.ok()
        )
        .into());
    }
    evidence("P14_CHECK step=1 status=PASS observed=capability-54");
    evidence("P14_CHECK step=2 status=PASS observed=rejected-frame,disposition-4,no-pool-delta");

    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        positions.iter().any(|statue| adjacent(at, *statue))
    })
    .ok_or("step=walk-to-statue observed=no-reachable-authored-statue need=statue-population-near-provisioned-character")?;
    walk(&mut client, world, &path).await?;
    let statue = adjacent_map_handle(&mut client, &references).ok_or(
        "step=statue-handle observed=authored-statue-without-live-handle need=map-item-handle",
    )?;
    let selected = training(&mut client, statue).await?;
    if selected == TrainingResult::REFUSED {
        let current_pool = general_stats(&mut client).await?;
        if current_pool != before_pool || client.join_snapshot().game_session_id != before_session {
            return Err(format!(
                "step=statue-refusal changed state; pool={before_pool}->{current_pool}"
            )
            .into());
        }
        let reply = client.session_mut().logout().await?;
        if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("step=statue-refusal logout={reply:?}").into());
        }
        return Err("step=offline-statue-training observed=authored-statue-refused-without-state-change need=current-Premium-and-unlocked-character; source:crates/gameplay/dispatch/src/gameplay_transport/loop_training/mod.rs:481".into());
    }
    if selected.disposition != 3 || selected.revision == 0 || selected.pool_seconds == 0 {
        return Err(format!(
            "step=select-statue observed={selected:?} need=disposition-3-nonzero-revision-and-pool"
        )
        .into());
    }
    let ended = match client.service_liveness(Duration::from_secs(5)).await {
        Err(DevClientError::Session(error)) => {
            let observed = error.to_string();
            if !observed.starts_with("transport I/O failed: ") {
                return Err(format!(
                    "step=3-server-ended-session observed={error:?} need=server-closed-connection-after-disposition-3"
                )
                .into());
            }
            observed
        }
        observed => {
            return Err(format!(
                "step=3-server-ended-session observed={observed:?} need=server-closed-connection-after-disposition-3"
            )
            .into());
        }
    };
    evidence(&format!(
        "P14_CHECK step=3 status=PASS observed=walked,statue-handle-{},disposition-3,revision-{},pool-seconds-{},server-ended-session-{ended}",
        statue.get(),
        selected.revision,
        selected.pool_seconds,
    ));

    let mut fresh = connect_world(clients, 0xb5).await?;
    let fresh_pool = general_stats(&mut fresh).await?;
    if fresh.join_snapshot().game_session_id == before_session
        || !matches_short_offline_pool(selected.pool_seconds, fresh_pool)
    {
        return Err(format!(
            "step=fresh-admission observed=session={:?},pool-minutes={fresh_pool},selected-pool-seconds={} need=new-session-with-durable-pool",
            fresh.join_snapshot().game_session_id,
            selected.pool_seconds
        )
        .into());
    }
    evidence(&format!(
        "P14_CHECK step=4 status=PASS observed=fresh-session,durable-pool-minutes-{fresh_pool}"
    ));
    let book = probe_training_book(&mut fresh, world, &bytes, &index).await;
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=final-logout observed=refused need=accepted-ordinary-logout".into());
    }
    book
}

async fn probe_training_book(
    client: &mut DevClientSession,
    world: &BundleWorld,
    bytes: &[u8],
    index: &crate::content::item_ref::ItemDefinitionIndex,
) -> TestResult {
    let reference = index.definition_ref("Item", BOOK, "definition-r1")
        .ok_or("step=offline-training-book observed=unindexed-i11441; source:content/items/definitions/items-01000-01499.json:1")?;
    let positions = placement_positions(bytes, &[reference])?;
    let path = route(world, tile(client.world_spatial().actor_position)?, |at| {
        positions.iter().any(|book| adjacent(at, *book))
    }).ok_or("step=offline-training-book observed=no-reachable-i11441-placement; source:content/items/definitions/items-01000-01499.json:1")?;
    walk(client, world, &path).await?;
    let handle = adjacent_map_handle(client, &[reference])
        .ok_or("step=offline-training-book observed=no-live-i11441-handle")?;
    use oteryn_protocol_oteryn::{server_modal as modal, world_object::UseDisposition};
    if !client
        .selected_capabilities()
        .contains(&modal::CAPABILITY_SERVER_MODAL_V1)
    {
        return Err("step=offline-training-book need=server-modal-capability; source:crates/protocol-oteryn/src/server_modal.rs:2".into());
    }
    if book_modal(client).await? != modal::Reply::None {
        return Err("step=offline-training-book modal already open before use".into());
    }
    let observed = client.use_item(handle).await?;
    if observed.status != CommandStatus::Accepted
        || observed.disposition != UseDisposition::Committed
    {
        return Err(format!("step=offline-training-book observed=use-status-{:?},disposition-{:?} need=committed-map-book-use; source:crates/gameplay/dispatch/src/gameplay_transport/item_inscription.rs:64", observed.status, observed.disposition).into());
    }
    let reply = book_modal(client).await?;
    if !instructional_dialog(&reply) {
        return Err(format!("step=offline-training-book observed={reply:?} need=map-book-instruction-dialog; source:crates/gameplay/dispatch/src/gameplay_transport/item_inscription.rs:64").into());
    }
    evidence("P14_CHECK step=5 status=PASS observed=book-training-instructions");
    Ok(())
}

fn adjacent(a: TilePos, b: TilePos) -> bool {
    a.floor == b.floor && a.x.abs_diff(b.x).max(a.y.abs_diff(b.y)) <= 1
}

fn placement_positions(bytes: &[u8], references: &[NonZeroU32]) -> TestResult<Vec<TilePos>> {
    let mut positions = Vec::new();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        for tile in sector.tiles {
            if tile.items.iter().any(|item| {
                references.iter().any(|reference| {
                    reference.get() == manifest.palette[item.palette as usize].id + 1
                })
            }) {
                positions.push(TilePos {
                    x: tile.x,
                    y: tile.y,
                    floor: sector.floor,
                });
            }
        }
        Ok(())
    })?;
    Ok(positions)
}

fn adjacent_map_handle(
    client: &mut DevClientSession,
    references: &[NonZeroU32],
) -> Option<ItemHandle> {
    let snapshot = client.session_mut().world_map_snapshot()?;
    let player = tile(client.world_spatial().actor_position).ok()?;
    snapshot
        .tiles
        .iter()
        .filter(|map_tile| tile(map_tile.position).is_ok_and(|at| adjacent(player, at)))
        .flat_map(|map_tile| &map_tile.items)
        .find_map(|item| match (item.definition, item.origin) {
            (MapDefinition::Item(reference), MapOrigin::Handle(handle))
                if references.contains(&reference) =>
            {
                Some(handle)
            }
            _ => None,
        })
}

async fn training(client: &mut DevClientSession, target: ItemHandle) -> TestResult<TrainingResult> {
    let payload = offline_payload(target);
    oteryn_protocol_oteryn::loop_training::decode_offline_training(&payload)
        .map_err(|error| format!("offline training encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_TRAINING_V1,
            COMMAND_TYPE_OFFLINE_TRAINING,
            &payload,
        )
        .await?;
    let decoded = decode_training_result(&reply.payload).ok_or_else(|| {
        format!(
            "step=offline-training-reply observed=malformed:{:?} need=bounded-training-result",
            reply.payload
        )
    })?;
    let expected = if decoded == TrainingResult::REFUSED {
        CommandStatus::Rejected
    } else {
        CommandStatus::Accepted
    };
    if reply.status != expected {
        return Err(format!(
            "step=offline-training-command observed=status:{:?} expected={expected:?}",
            reply.status,
        )
        .into());
    }
    Ok(decoded)
}

fn offline_payload(target: ItemHandle) -> Vec<u8> {
    let mut payload = vec![8];
    push_varint(&mut payload, target.get());
    payload
}

fn push_varint(output: &mut Vec<u8>, mut value: u64) {
    while value >= 128 {
        output.push((value as u8 & 127) | 128);
        value >>= 7;
    }
    output.push(value as u8);
}

fn decode_training_result(bytes: &[u8]) -> Option<TrainingResult> {
    if bytes.len() > oteryn_protocol_oteryn::loop_training::MAX_TRAINING_RESULT_BYTES {
        return None;
    }
    let mut values = [None; 4];
    let mut cursor = 0;
    while cursor < bytes.len() {
        let tag = *bytes.get(cursor)?;
        cursor += 1;
        if !matches!(tag, 8 | 16 | 24 | 32) {
            return None;
        }
        let slot = &mut values[usize::from(tag / 8 - 1)];
        if slot.is_some() {
            return None;
        }
        let mut value = 0u64;
        let mut complete = false;
        for shift in (0..70).step_by(7) {
            let byte = *bytes.get(cursor)?;
            cursor += 1;
            if shift == 63 && byte > 1 {
                return None;
            }
            value |= u64::from(byte & 127) << shift;
            if byte & 128 == 0 {
                complete = true;
                break;
            }
        }
        if !complete {
            return None;
        }
        *slot = Some(value);
    }
    let result = TrainingResult {
        disposition: u8::try_from(values[0]?).ok()?,
        revision: values[1]?,
        pool_seconds: u32::try_from(values[2]?).ok()?,
        charges: u32::try_from(values[3]?).ok()?,
    };
    (oteryn_protocol_oteryn::loop_training::encode_training_result(result).ok()? == bytes)
        .then_some(result)
}

async fn general_stats(client: &mut DevClientSession) -> TestResult<u16> {
    use oteryn_protocol_oteryn::cyclopedia_character::{self as wire, CharacterTab};
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            388,
            &[CharacterTab::GeneralStats as u8, 0, 0, 1],
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || general_stats::validate(&reply.payload, general_stats::selected(client)).is_err()
        || reply.payload[0] != CharacterTab::GeneralStats as u8
    {
        return Err(format!(
            "step=general-stats observed=status:{:?},payload-len:{} need=accepted-negotiated-general-stats",
            reply.status,
            reply.payload.len()
        )
        .into());
    }
    Ok(u16::from_le_bytes(reply.payload[31..33].try_into()?))
}

async fn book_modal(
    client: &mut DevClientSession,
) -> TestResult<oteryn_protocol_oteryn::server_modal::Reply> {
    use oteryn_protocol_oteryn::server_modal as modal;
    let reply = client
        .session_mut()
        .gated_command(
            modal::CAPABILITY_SERVER_MODAL_V1,
            modal::COMMAND_TYPE_SERVER_MODAL_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "step=offline-training-book modal query status={:?}",
            reply.status
        )
        .into());
    }
    modal::Reply::decode(&reply.payload).map_err(Into::into)
}

fn instructional_dialog(reply: &oteryn_protocol_oteryn::server_modal::Reply) -> bool {
    use oteryn_protocol_oteryn::server_modal::Reply;
    let Reply::Dialog(dialog) = reply else {
        return false;
    };
    let text = dialog.message.to_ascii_lowercase();
    (dialog
        .title
        .to_ascii_lowercase()
        .contains("offline training")
        || text.contains("offline training"))
        && text.contains("statue")
        && (text.contains("log out") || text.contains("logout"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn accepts_training_instructions_but_not_an_unrelated_dialog() {
        use oteryn_protocol_oteryn::server_modal::{Dialog, Reply};
        let mut dialog = Dialog {
            id: 1,
            title: "Information on Offline Training".into(),
            message: "Premium characters can use a training statue and log out to train offline."
                .into(),
            choices: vec![],
            buttons: vec![(1, "OK".into())],
            default_button: None,
            cancel_button: None,
            image: None,
        };
        assert!(super::instructional_dialog(&Reply::Dialog(dialog.clone())));
        dialog.title = "Market".into();
        dialog.message = "Sell your items.".into();
        assert!(!super::instructional_dialog(&Reply::Dialog(dialog)));
        assert!(!super::instructional_dialog(&Reply::None));
    }

    #[test]
    fn quick_relog_must_not_spend_training_pool() {
        assert!(super::matches_short_offline_pool(3_616, 60));
        assert!(!super::matches_short_offline_pool(3_616, 59));
    }

    use super::*;

    #[test]
    fn offline_payload_round_trips_maximum_handle() -> TestResult {
        let handle = NonZeroU64::MAX;
        let payload = offline_payload(handle);
        let decoded = oteryn_protocol_oteryn::loop_training::decode_offline_training(&payload)
            .map_err(|error| format!("decode: {error:?}"))?;
        assert_eq!(decoded, handle);
        Ok(())
    }
}
