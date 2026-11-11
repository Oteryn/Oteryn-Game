//! W10 player scenario, derived from the matrix row, re-audit, and the native Hazard owner:
//! 1. Admit a node-boot character and send an empty Hazard command. Expect ACCEPTED and the
//!    two-byte `[unlocked, selected]` state delta, initially with level 1 selected.
//! 2. Send selection level 1. Expect ACCEPTED, the same two-byte state, and no unrelated state
//!    mutation. Then send locked level 2 and expect a REJECTED reply with no payload (the refusal).
//! 3. Walk, using compiled-world collision, to an authored Primal Menace encounter. Send ATTACK,
//!    require the accepted combat-state delta, remain alive while servicing native combat, and
//!    require the monster's removal/death frames.
//! 4. Require the kill's Hazard unlock reply to change `[1, 1]` to `[2, 1]`; select level 2 and
//!    require `[2, 2]`. Fight and loot an authored Hazard-area creature, asserting combat vitals
//!    and carried-inventory deltas rather than inspecting storage directly.
//! 5. Log out, freshly admit the same character, query Hazard again, and require `[2, 2]` plus the
//!    retained loot. Re-select locked level 3 and require the same empty REJECTED refusal.
//!
//! Every command above is a native player command. If authored encounter access prevents a later
//! command, the row reports the first observed step and reply/path failure; it never claims PASS.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{attack as attacks, hazard};

const PRIMAL: &str = "oteryn:creature.the_primal_menace";

#[derive(Debug)]
struct StepFailure {
    step: u8,
    observed: String,
    need: &'static str,
}

type StepResult<T = ()> = Result<T, StepFailure>;

fn failed(step: u8, observed: impl std::fmt::Display, need: &'static str) -> StepFailure {
    StepFailure {
        step,
        observed: observed.to_string().replace(' ', "_"),
        need,
    }
}

async fn command(
    client: &mut DevClientSession,
    selection: Option<u8>,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let payload = selection.map_or_else(Vec::new, |level| vec![level]);
    let reply = client
        .session_mut()
        .gated_command(
            hazard::CAPABILITY_HAZARD_V1,
            hazard::COMMAND_TYPE_HAZARD,
            &payload,
        )
        .await?;
    Ok((reply.status, reply.payload))
}

fn accepted(step: u8, reply: (CommandStatus, Vec<u8>), expected: [u8; 2]) -> StepResult {
    if reply.0 != CommandStatus::Accepted || reply.1 != expected {
        return Err(failed(
            step,
            format!("status={:?},payload={:?}", reply.0, reply.1),
            "an accepted Hazard reply carrying the exact unlocked/selected delta",
        ));
    }
    evidence(&format!(
        "W10_CHECK step={step} status=PASS reply=ACCEPTED hazard={}/{}",
        expected[0], expected[1]
    ));
    Ok(())
}

fn rejected(step: u8, reply: (CommandStatus, Vec<u8>)) -> StepResult {
    if reply.0 != CommandStatus::Rejected || !reply.1.is_empty() {
        return Err(failed(
            step,
            format!("status={:?},payload={:?}", reply.0, reply.1),
            "an empty REJECTED reply for a locked Hazard level",
        ));
    }
    evidence(&format!(
        "W10_CHECK step={step} status=PASS reply=REJECTED payload=empty"
    ));
    Ok(())
}

fn primal_positions(bytes: &[u8]) -> TestResult<Vec<TilePos>> {
    let metadata = oteryn_world_bundle::bundle::visit(bytes, |_, _| Ok(()))?;
    let mut positions: Vec<_> = metadata
        .spawns
        .sources
        .iter()
        .flat_map(|source| source.points.iter().map(move |point| (source.floor, point)))
        .filter(|(_, point)| {
            metadata
                .spawns
                .creatures
                .get(point.creature as usize)
                .is_some_and(|creature| creature.key == PRIMAL)
        })
        .map(|(floor, point)| TilePos {
            x: point.x,
            y: point.y,
            floor,
        })
        .collect();
    positions.sort_unstable();
    positions.dedup();
    Ok(positions)
}

async fn journey(clients: &SeamClients<'_>) -> StepResult {
    let mut active_client = None;
    let result: StepResult = tokio::time::timeout(Duration::from_secs(150), async {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE").map_err(|error| {
        failed(1, error, "the node-boot compiled world bundle")
    })?)
    .map_err(|error| failed(1, error, "the node-boot compiled world bundle"))?;
    let pin: serde_json::Value = serde_json::from_slice(
        &std::fs::read(required("NODE_BOOT_WORLD_PIN").map_err(|error| {
            failed(1, error, "the node-boot world pin")
        })?)
        .map_err(|error| failed(1, error, "the node-boot world pin"))?,
    )
    .map_err(|error| failed(1, error, "a valid node-boot world pin"))?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world).map_err(|error| failed(1, error, "a valid world id"))?,
        ChannelId::decode(&clients.channel)
            .map_err(|error| failed(1, error, "a valid channel id"))?,
    )
    .map_err(|error| failed(1, error, "the production world loader to accept the bundle"))?;
    active_client = Some(connect_world(clients, 0xd0)
        .await
        .map_err(|error| failed(1, error, "fresh native admission"))?);
    let client = active_client.as_mut().ok_or_else(|| failed(1, "admission absent", "native admission"))?;

    accepted(
        1,
        command(&mut *client, None)
            .await
            .map_err(|error| failed(1, error, "a Hazard query reply"))?,
        [1, 1],
    )?;
    accepted(
        2,
        command(&mut *client, Some(1))
            .await
            .map_err(|error| failed(2, error, "a Hazard selection reply"))?,
        [1, 1],
    )?;
    rejected(
        2,
        command(&mut *client, Some(2))
            .await
            .map_err(|error| failed(2, error, "a locked-level refusal"))?,
    )?;

    let positions = primal_positions(&bytes)
        .map_err(|error| failed(3, error, "an authored Primal Menace spawn"))?;
    let from = tile(client.world_spatial().actor_position)
        .map_err(|error| failed(3, error, "a valid actor position"))?;
    let path = route(world, from, |at| {
        positions.iter().any(|position| {
            position.floor == at.floor
                && position.x.abs_diff(at.x).max(position.y.abs_diff(at.y)) <= 1
        })
    })
    .ok_or_else(|| {
        failed(
            3,
            format!("no_reachable_primal_spawn;authored={};from={from:?}", positions.len()),
            "an ordinary walkable route from admission to the authored Primal Ordeal encounter",
        )
    })?;
    walk(&mut *client, world, &path)
        .await
        .map_err(|error| failed(3, error, "accepted STEP replies to the Primal encounter"))?;
    let at = client.world_spatial().actor_position;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities
                .iter()
                .find(|entity| {
                    entity.kind == oteryn_dev_client::EntityKind::Creature
                        && entity.position.floor == at.floor
                        && entity.position.x.abs_diff(at.x).max(entity.position.y.abs_diff(at.y)) <= 1
                })
                .copied()
        })
        .ok_or_else(|| failed(3, "spawn_reached_but_no_creature_frame", "a visible Primal Menace entity"))?;
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await
        .map_err(|error| failed(3, error, "an ATTACK reply"))?;
    if attack.disposition != attacks::AttackIntentDisposition::Ok {
        return Err(failed(
            3,
            format!("attack={attack:?}"),
            "an accepted Primal Menace ATTACK with combat delta",
        ));
    }
    evidence("W10_CHECK step=3 status=PASS reply=ATTACK_OK combat-delta=present");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    while client
        .world_entities()
        .is_some_and(|entities| entities.iter().any(|entity| entity.entity == target.entity))
    {
        if tokio::time::Instant::now() >= deadline {
            let _ = client.session_mut().attack_target(None).await;
            return Err(failed(
                3,
                format!("combat_timeout;vitals={:?}", client.actor_vitals()),
                "the provisioned player build or legitimate encounter support needed to complete the Primal fight",
            ));
        }
        client
            .service_liveness(Duration::from_millis(100))
            .await
            .map_err(|error| failed(3, error, "continued native combat frames"))?;
        client.take_events();
    }
    client
        .session_mut()
        .attack_target(None)
        .await
        .map_err(|error| failed(3, error, "an attack-stop reply"))?;

    accepted(
        4,
        command(&mut *client, None)
            .await
            .map_err(|error| failed(4, error, "the post-kill Hazard unlock delta"))?,
        [2, 1],
    )?;
    accepted(
        4,
        command(&mut *client, Some(2))
            .await
            .map_err(|error| failed(4, error, "the level-2 Hazard selection delta"))?,
        [2, 2],
    )?;
    let inventory_before = client
        .inventory()
        .ok_or_else(|| failed(4, "inventory_snapshot_missing", "the pre-loot inventory frame"))?
        .clone();
    let old_corpses: Vec<_> = client.world_entities().into_iter().flat_map(|entities| entities.iter()).filter(|entity| entity.kind == oteryn_dev_client::EntityKind::Corpse).map(|entity| entity.entity).collect();
    let second = client
        .world_entities()
        .and_then(|entities| {
            entities
                .iter()
                .find(|entity| {
                    entity.kind == oteryn_dev_client::EntityKind::Creature
                        && entity.entity != target.entity
                        && entity.position.floor == at.floor
                        && entity.position.x.abs_diff(at.x).max(entity.position.y.abs_diff(at.y))
                            <= 7
                })
                .copied()
        })
        .ok_or_else(|| {
            failed(
                4,
                "no_visible_hazard_area_creature_after_unlock",
                "an authored reachable Hazard-area combat and loot target for the scaling comparison",
            )
        })?;
    let second_attack = client
        .session_mut()
        .attack_target(Some(&second.entity))
        .await
        .map_err(|error| failed(4, error, "a scaled-combat ATTACK reply"))?;
    if second_attack.disposition != attacks::AttackIntentDisposition::Ok {
        return Err(failed(
            4,
            format!("scaled_attack={second_attack:?}"),
            "an accepted Hazard-area ATTACK and combat delta",
        ));
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    let mut corpse_position = second.position;
    while client
        .world_entities()
        .is_some_and(|entities| entities.iter().any(|entity| entity.entity == second.entity))
    {
        if tokio::time::Instant::now() >= deadline {
            let _ = client.session_mut().attack_target(None).await;
            return Err(failed(
                4,
                format!("scaled_combat_timeout;vitals={:?}", client.actor_vitals()),
                "a completable authored Hazard-area combat target",
            ));
        }
        if let Some(entity) = client.world_entities().and_then(|entities| entities.get(&second.entity)) { corpse_position = entity.position; }
        client
            .service_liveness(Duration::from_millis(100))
            .await
            .map_err(|error| failed(4, error, "scaled combat/vitals frames"))?;
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::WorldSpatialEntities(delta) = event
                && let Some(entity) = delta.value.update.iter().find(|entity| entity.entity == second.entity) {
                corpse_position = entity.position;
            }
        }
    }
    client
        .session_mut()
        .attack_target(None)
        .await
        .map_err(|error| failed(4, error, "the scaled attack-stop reply"))?;
    let corpse = client.world_entities().and_then(|entities| entities.iter().find(|entity| entity.kind == oteryn_dev_client::EntityKind::Corpse && entity.position == corpse_position && !old_corpses.contains(&entity.entity))).copied().ok_or_else(|| failed(4, "hazard_corpse_missing", "the killed creature's corpse"))?;
    let oteryn_dev_client::EntityDetail::Object { item_handle: Some(handle), .. } = corpse.detail else { return Err(failed(4, "corpse_handle_missing", "the server-issued corpse handle")); };
    let used = client.use_item(handle).await.map_err(|error| failed(4, error, "accepted corpse use"))?;
    if used.status != CommandStatus::Accepted || used.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed { return Err(failed(4, format!("{used:?}"), "committed corpse use")); }
    let loot_deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while client.open_container().is_none_or(|container| container.container_handle != Some(handle)) {
        if tokio::time::Instant::now() >= loot_deadline { return Err(failed(4, "corpse_view_timeout", "opened corpse delta")); }
        client.service_liveness(Duration::from_millis(50)).await.map_err(|error| failed(4, error, "corpse view"))?;
    }
    let loot = client.open_container().ok_or_else(|| failed(4, "corpse_view_missing", "corpse inventory"))?.entries.clone();
    for item in loot {
        let moved = client.move_item(&oteryn_protocol_oteryn::item_view::ItemMoveIntent { source: item.handle, destination: oteryn_protocol_oteryn::item_view::ItemMoveDestination::MainBackpack }).await.map_err(|error| failed(4, error, "loot move"))?;
        if moved.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved { return Err(failed(4, format!("{moved:?}"), "committed loot move")); }
    }
    let inventory_after = client
        .inventory()
        .ok_or_else(|| failed(4, "post_kill_inventory_missing", "the loot inventory delta"))?
        .clone();
    if inventory_after == inventory_before {
        return Err(failed(
            4,
            "hazard_kill_had_no_carried_loot_delta",
            "a native corpse-loot action/result that exposes the scaled loot through the dev client",
        ));
    }
    evidence("W10_CHECK step=4 status=PASS scaled-combat=kill loot-delta=present");

    let old_session = client.join_snapshot().game_session_id;
    if client
        .session_mut()
        .logout()
        .await
        .map_err(|error| failed(5, error, "an accepted logout reply"))?
        .result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err(failed(5, "logout_refused", "an accepted logout reply"));
    }
    *client = connect_world(clients, 0xd1)
        .await
        .map_err(|error| failed(5, error, "fresh native readmission"))?;
    if client.join_snapshot().game_session_id == old_session {
        return Err(failed(5, "session_not_fresh", "a new admitted game session"));
    }
    accepted(
        5,
        command(&mut *client, None)
            .await
            .map_err(|error| failed(5, error, "the reloaded Hazard state"))?,
        [2, 2],
    )?;
    if client.inventory().is_none_or(|inventory| row_inventory_signature(inventory) != row_inventory_signature(&inventory_after)) {
        return Err(failed(
            5,
            "loot_inventory_changed_after_fresh_admission",
            "the durable carried loot snapshot",
        ));
    }
    rejected(
        5,
        command(&mut *client, Some(3))
            .await
            .map_err(|error| failed(5, error, "the reloaded locked-level refusal"))?,
    )?;
    Ok(())

    }).await.unwrap_or_else(|error| Err(failed(5, error, "journey deadline")));
    if let Some(client) = active_client.as_mut() {
        row_logout(client)
            .await
            .map_err(|error| failed(5, error, "accepted terminal logout"))?;
    }
    result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete(journey(clients)).await;
    match result {
        Ok(Ok(())) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W10.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => {
            evidence(&format!(
                "W10_GAP step={} observed={} need={}",
                error.step,
                error.observed,
                error.need.replace(' ', "_")
            ));
            evidence(&format!(
                "SEAM_EVIDENCE e2e_all scenario=W10.full status=FAIL seconds={:.3} evidence=step{}:{}",
                started.elapsed().as_secs_f64(),
                error.step,
                error.observed
            ));
        }
        Err(_) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W10.full status=FAIL seconds={:.3} evidence=step3:journey_timeout",
            started.elapsed().as_secs_f64()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hazard_reply_checks_require_exact_status_and_delta() {
        assert!(accepted(1, (CommandStatus::Accepted, vec![1, 1]), [1, 1]).is_ok());
        assert!(accepted(1, (CommandStatus::Accepted, vec![2, 1]), [1, 1]).is_err());
        assert!(rejected(2, (CommandStatus::Rejected, Vec::new())).is_ok());
        assert!(rejected(2, (CommandStatus::Accepted, Vec::new())).is_err());
    }
}
