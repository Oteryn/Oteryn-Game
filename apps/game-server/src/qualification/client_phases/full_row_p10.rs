//! P10 full-row journey: daily-streak resting-area bonuses as the client sees them.
//!
//! Concrete player scenario, derived from the P10 matrix row and the read owner
//! `gameplay_transport/rest_status.rs` (`rest_status_page`):
//! 1. Admit the node-boot character, require capability 277 and build the query with the
//!    production client-ui builder (`rest_status::request`); send it and apply the reply with
//!    the production reducer (`RestStatusPanel::finish`). The applied snapshot must agree with
//!    the client's own map (inside a resting area exactly when the actor's tile is a protection
//!    zone) and with the owner rule: outside, no bonus; inside and alive, tier 7 gives doubled
//!    health and mana regeneration plus stamina and soul for premium, and the free-account cap
//!    (streak tier 3) gives single health and mana regeneration only.
//! 2. Send a malformed query (one payload byte); expect a rejected empty reply, and the reducer
//!    must drop the presented bonuses instead of keeping stale ones.
//! 3. Walk through the compiled world across the protection-zone boundary (out of it when the
//!    character starts inside, into it otherwise) and query again; the snapshot must flip with
//!    the tile. Then end the walk inside a resting area and query once more.
//! 4. Log out, make a fresh admission and query again; streak, area and bonuses must equal the
//!    last snapshot (food may only run down), proving the projection is rebuilt from durable
//!    state at the durable position.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_ui::rest_status::{RestStatus, RestStatusPanel, request};
use oteryn_protocol_oteryn::rest_status::{
    CAPABILITY_REST_STATUS_V1, COMMAND_TYPE_REST_STATUS_QUERY,
};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout-150s".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=P10.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

/// One query through the production client builder and reducer.
async fn query(
    client: &mut DevClientSession,
    world: &BundleWorld,
    panel: &mut RestStatusPanel,
    step: &str,
) -> TestResult<RestStatus> {
    let payload = request(panel)
        .ok_or_else(|| format!("step={step} observed=client-builder-refused need=query"))?;
    let reply = client
        .session_mut()
        .gated_command(
            CAPABILITY_REST_STATUS_V1,
            COMMAND_TYPE_REST_STATUS_QUERY,
            &payload,
        )
        .await?;
    let accepted = matches!(
        reply.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    );
    if !panel.finish(accepted, &reply.payload) {
        return Err(format!(
            "step={step} observed=status-{:?}-{}-bytes-not-applied need=applied-rest-status",
            reply.status,
            reply.payload.len()
        )
        .into());
    }
    let snapshot = panel
        .snapshot
        .ok_or_else(|| format!("step={step} observed=applied-without-snapshot"))?;
    check(client, tile_is_pz(client, world)?, snapshot, step)?;
    evidence(&format!("P10_CHECK {step} PASS {snapshot:?}"));
    Ok(snapshot)
}

fn tile_is_pz(client: &DevClientSession, world: &BundleWorld) -> TestResult<bool> {
    let at = tile(client.world_spatial().actor_position)?;
    Ok(world
        .base()
        .tile(at.x, at.y, at.floor)
        .is_some_and(|tile| tile.protection_zone()))
}

/// The owner rule of `rest_status_page`, checked against what the reducer applied.
fn check(client: &DevClientSession, pz: bool, value: RestStatus, step: &str) -> TestResult {
    let alive = client
        .actor_vitals()
        .ok_or_else(|| format!("step={step} observed=no-actor-vitals"))?
        .health
        != 0;
    let premium = value.stamina;
    let expected = if !value.inside_resting_area || !alive {
        (0, 0, false, false)
    } else if premium {
        (2, 2, true, true)
    } else {
        (1, 1, false, false)
    };
    let observed = (
        value.health_multiplier,
        value.mana_multiplier,
        value.stamina,
        value.soul,
    );
    if value.inside_resting_area != pz || observed != expected {
        return Err(format!(
            "step={step} observed=inside-{}-tile-pz-{pz}-alive-{alive}-bonus-{observed:?} need=bonus-{expected:?}-and-area-matching-tile",
            value.inside_resting_area
        )
        .into());
    }
    Ok(())
}

async fn walk_until(
    client: &mut DevClientSession,
    world: &BundleWorld,
    pz: bool,
    step: &str,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.protection_zone() == pz)
    })
    .ok_or_else(|| format!("step={step} observed=no-route need=tile-with-pz-{pz}"))?;
    walk(client, world, &path).await
}

async fn logout(client: &mut DevClientSession, step: &str) -> TestResult {
    let (result, observed, attempts, _) = logout_retrying_busy(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!(
            "step={step} observed={observed}-after-{attempts}-attempts need=accepted-logout"
        )
        .into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0x6a).await?;
    let selected = client
        .selected_capabilities()
        .contains(&CAPABILITY_REST_STATUS_V1);
    if !selected {
        return Err("step=admit observed=capability-277-not-selected".into());
    }
    let initial_session = client.join_snapshot().game_session_id;
    let mut panel = RestStatusPanel {
        selected,
        ..Default::default()
    };
    let baseline = query(&mut client, world, &mut panel, "baseline").await?;

    let refused = client
        .session_mut()
        .gated_command(
            CAPABILITY_REST_STATUS_V1,
            COMMAND_TYPE_REST_STATUS_QUERY,
            &[0],
        )
        .await?;
    if refused.status != CommandStatus::Rejected || !refused.payload.is_empty() {
        return Err(format!(
            "step=malformed observed=status-{:?}-{}-bytes need=empty-rejection",
            refused.status,
            refused.payload.len()
        )
        .into());
    }
    if panel.finish(false, &refused.payload) || panel.snapshot.is_some() || panel.error.is_none() {
        return Err("step=malformed observed=reducer-kept-stale-bonus need=cleared-panel".into());
    }

    walk_until(
        &mut client,
        world,
        !baseline.inside_resting_area,
        "cross-boundary",
    )
    .await?;
    let crossed = query(&mut client, world, &mut panel, "cross-boundary").await?;
    if crossed.inside_resting_area == baseline.inside_resting_area
        || crossed.streak != baseline.streak
    {
        return Err(format!(
            "step=cross-boundary observed={crossed:?}-from-{baseline:?} need=flipped-area-same-streak"
        )
        .into());
    }
    walk_until(&mut client, world, true, "return-to-rest").await?;
    let rested = query(&mut client, world, &mut panel, "return-to-rest").await?;
    if !rested.inside_resting_area || rested.streak != baseline.streak {
        return Err(
            format!("step=return-to-rest observed={rested:?} need=inside-same-streak").into(),
        );
    }

    logout(&mut client, "logout").await?;
    drop(client);
    let mut fresh = connect_world(clients, 0x6b).await?;
    if fresh.join_snapshot().game_session_id == initial_session {
        return Err("step=fresh-admission observed=reused-game-session need=new-admission".into());
    }
    let mut fresh_panel = RestStatusPanel {
        selected: fresh
            .selected_capabilities()
            .contains(&CAPABILITY_REST_STATUS_V1),
        ..Default::default()
    };
    let durable = query(&mut fresh, world, &mut fresh_panel, "fresh-admission").await;
    let cleanup = logout(&mut fresh, "final-logout").await;
    let durable = durable?;
    let food_ran_down = match (rested.food_remaining_ms, durable.food_remaining_ms) {
        (-1, after) => after == -1,
        (before, after) => (0..=before).contains(&after),
    };
    let comparable = RestStatus {
        food_remaining_ms: rested.food_remaining_ms,
        ..durable
    };
    if comparable != rested || !food_ran_down {
        return Err(format!(
            "step=fresh-admission observed={durable:?} need=same-as-{rested:?}-food-not-increased"
        )
        .into());
    }
    cleanup?;
    Ok(format!(
        "streak={};baseline_inside={};crossed_inside={};rest_bonus=({},{},{},{});durable-after-relog",
        baseline.streak,
        baseline.inside_resting_area,
        crossed.inside_resting_area,
        rested.health_multiplier,
        rested.mana_multiplier,
        rested.stamina,
        rested.soul
    ))
}
