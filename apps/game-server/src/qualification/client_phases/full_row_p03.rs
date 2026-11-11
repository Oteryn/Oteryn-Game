//! P03 full-row player scenario, derived from the P03 audit/matrix row and the
//! durable Prey command/checkpoint owner:
//! 1. Admit the provisioned character and send `Query`; require an unlocked
//!    slot and the complete typed slot projection.
//! 2. Send `ListReroll`; require an accepted receipt plus a current-state
//!    delta containing a nonempty draw and an advanced free-reroll deadline.
//! 3. Send `Select` for a drawn creature that has a compiled-world spawn;
//!    require an active selection, bonus/grade/percent, 7,200 bonus seconds,
//!    and a new durable selection epoch.
//! 4. Send `BonusReroll`; if the ordinary character owns a wildcard, require
//!    its debit and a changed bonus receipt; otherwise require the explicit
//!    refusal and unchanged state. Then send an out-of-range `Select`, require
//!    rejection, and prove by `Query` that the active selection was unchanged.
//! 5. Walk to the selected creature and use native ATTACK/corpse/loot actions.
//!    After the server hunting cadence, attack it again; require real combat
//!    deltas and a `Query` checkpoint with fewer bonus seconds while the
//!    selected bonus and occurrence fence remain active.
//! 6. Logout, obtain a fresh admission, and send `Query`; require the selected
//!    creature, bonus, remaining time, reroll deadline, and selection epoch to
//!    equal the last acknowledged checkpoint.
use super::*;
use oteryn_dev_client::client_runtime::prey::{self, CommandStatus};
use prey::prey as wire;

type Reply = wire::Reply;

async fn command(
    client: &mut oteryn_dev_client::DevClientSession,
    request: wire::Request,
) -> TestResult<prey::PreyResponse> {
    Ok(prey::send(client.session_mut(), &request).await?)
}

fn decoded(bytes: &[u8], step: &str) -> TestResult<Reply> {
    wire::decode_reply(bytes).map_err(|error| format!("P03 {step} malformed reply: {error:?}").into())
}

async fn accepted(
    client: &mut oteryn_dev_client::DevClientSession,
    request: wire::Request,
    step: &str,
) -> TestResult<Reply> {
    let response = command(client, request).await?;
    if !matches!(response.status, CommandStatus::Accepted | CommandStatus::DuplicateReplay) {
        return Err(format!("P03 step={step} observed-command-status={:?}", response.status).into());
    }
    let receipt = decoded(&response.payload, step)?;
    if receipt.status != 0 {
        return Err(format!("P03 step={step} observed-prey-status={}", receipt.status).into());
    }
    match response.current {
        Some(current) => decoded(&current, step),
        None => Ok(receipt),
    }
}

async fn query(client: &mut oteryn_dev_client::DevClientSession, step: &str) -> TestResult<Reply> {
    accepted(client, wire::Request::Query, step).await
}

fn slot(reply: &Reply, number: u8) -> TestResult<&wire::Slot> {
    reply
        .slots
        .iter()
        .find(|slot| slot.slot == number)
        .ok_or_else(|| format!("P03 missing slot {number}").into())
}

fn same_checkpoint(left: &wire::Slot, right: &wire::Slot) -> bool {
    left.state == right.state
        && left.selected == right.selected
        && left.bonus == right.bonus
        && left.grade == right.grade
        && left.percent == right.percent
        && left.hunting_seconds == right.hunting_seconds
        && left.free_reroll_at == right.free_reroll_at
        && left.selection_epoch == right.selection_epoch
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = row_connect_world(clients, 0xa1).await?;
    let initial_session = client.join_snapshot().game_session_id;
    let initial = query(&mut client, "query").await?;
    let unlocked = initial
        .slots
        .iter()
        .find(|slot| slot.state != 0)
        .ok_or("P03 step=query observed=no-unlocked-slot")?;
    let number = unlocked.slot;
    let before_deadline = unlocked.free_reroll_at;

    let drawn = accepted(
        &mut client,
        wire::Request::ListReroll { slot: number },
        "draw-reroll",
    )
    .await?;
    let draw = slot(&drawn, number)?;
    if draw.state != 1 || draw.creatures.is_empty() || draw.free_reroll_at <= before_deadline {
        return Err(format!(
            "P03 step=draw-reroll observed=state:{}-candidates:{}-deadline:{}",
            draw.state,
            draw.creatures.len(),
            draw.free_reroll_at
        )
        .into());
    }
    let (index, creature) = draw
        .creatures
        .iter()
        .enumerate()
        .find(|(_, creature)| {
            metadata.spawns.sources.iter().any(|source| {
                source.points.iter().any(|point| {
                    metadata
                        .spawns
                        .creatures
                        .get(point.creature as usize)
                        .is_some_and(|entry| entry.key == creature.as_str())
                })
            })
        })
        .ok_or("P03 step=select observed=no-drawn-creature-with-compiled-spawn")?;
    let active = accepted(
        &mut client,
        wire::Request::Select {
            slot: number,
            index: u8::try_from(index + 1)?,
        },
        "select",
    )
    .await?;
    let selected = slot(&active, number)?.clone();
    if selected.state != 2
        || selected.selected != *creature
        || selected.bonus == 0
        || selected.grade == 0
        || selected.percent == 0
        || selected.hunting_seconds != 7_200
        || selected.selection_epoch == 0
    {
        return Err(format!("P03 step=select observed={selected:?}").into());
    }

    let bonus_reroll = command(
        &mut client,
        wire::Request::BonusReroll { slot: number },
    )
    .await?;
    let selected = if matches!(
        bonus_reroll.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        let receipt = decoded(&bonus_reroll.payload, "bonus-reroll")?;
        let current = decoded(
            bonus_reroll
                .current
                .as_deref()
                .ok_or("P03 step=bonus-reroll observed=missing-current-state")?,
            "bonus-reroll-current",
        )?;
        let rerolled = slot(&current, number)?.clone();
        if receipt.status != 0
            || receipt.wildcard_balance.checked_add(1) != Some(active.wildcard_balance)
            || rerolled.selected != selected.selected
            || rerolled.selection_epoch != selected.selection_epoch
            || (rerolled.bonus, rerolled.grade, rerolled.percent)
                == (selected.bonus, selected.grade, selected.percent)
        {
            return Err(format!(
                "P03 step=bonus-reroll observed-receipt={receipt:?}-state={rerolled:?}"
            )
            .into());
        }
        rerolled
    } else if bonus_reroll.status == CommandStatus::Rejected {
        let after = query(&mut client, "bonus-reroll-refusal-check").await?;
        if !same_checkpoint(&selected, slot(&after, number)?) {
            return Err("P03 step=bonus-reroll-refusal observed=active-state-mutated".into());
        }
        selected
    } else {
        return Err(format!(
            "P03 step=bonus-reroll observed-command-status={:?}",
            bonus_reroll.status
        )
        .into());
    };

    let refusal = command(
        &mut client,
        wire::Request::Select {
            slot: number,
            index: wire::MAX_CREATURES as u8,
        },
    )
    .await?;
    if refusal.status != CommandStatus::Rejected {
        return Err(format!(
            "P03 step=refusal observed-command-status={:?}-prey-payload={:?}",
            refusal.status, refusal.payload
        )
        .into());
    }
    let after_refusal = query(&mut client, "refusal-check").await?;
    if !same_checkpoint(&selected, slot(&after_refusal, number)?) {
        return Err("P03 step=refusal observed=active-state-mutated".into());
    }

    travel_to_spawn(&mut client, world, &metadata.spawns, &selected.selected).await?;
    client_hunt::hunt(&mut client).await?;
    // Keep answering liveness while the interval runs out; a bare sleep lets
    // the server drop the session as unresponsive.
    let until = tokio::time::Instant::now() + Duration::from_secs(61);
    while tokio::time::Instant::now() < until {
        client.service_liveness(Duration::from_millis(500)).await?;
    }
    travel_to_spawn(&mut client, world, &metadata.spawns, &selected.selected).await?;
    client_hunt::hunt(&mut client).await?;
    let checkpoint = query(&mut client, "effect-checkpoint").await?;
    let checkpoint = slot(&checkpoint, number)?.clone();
    if checkpoint.state != 2
        || checkpoint.selected != selected.selected
        || checkpoint.bonus != selected.bonus
        || checkpoint.percent != selected.percent
        || checkpoint.selection_epoch != selected.selection_epoch
        || checkpoint.hunting_seconds >= selected.hunting_seconds
    {
        return Err(format!("P03 step=effect-checkpoint observed={checkpoint:?}").into());
    }

    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("P03 step=logout observed=refused".into());
    }
    drop(client);
    let mut client = row_connect_world(clients, 0xa2).await?;
    if client.join_snapshot().game_session_id == initial_session {
        return Err("P03 step=fresh-admission observed=reused-session".into());
    }
    let durable = query(&mut client, "fresh-admission-query").await?;
    if !same_checkpoint(&checkpoint, slot(&durable, number)?) {
        return Err(format!(
            "P03 step=fresh-admission observed={:?}-expected={checkpoint:?}",
            slot(&durable, number)?
        )
        .into());
    }
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("P03 step=final-logout observed=refused".into());
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(360), journey(clients)).await;
    let error = match result {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.to_string()),
        Err(_) => Some("step=journey observed=timeout".to_owned()),
    };
    let status = if error.is_none() { "PASS" } else { "FAIL" };
    let evidence_value = error.map(|error| {
        error
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() || "-_:./".contains(ch) { ch } else { '-' })
            .collect::<String>()
    });
    evidence(&format!(
        "e2e_all scenario=P03.full status={status} seconds={:.3}{}",
        started.elapsed().as_secs_f64(),
        evidence_value
            .as_deref()
            .map(|value| format!(" evidence={value}"))
            .unwrap_or_default()
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_comparison_includes_durable_occurrence_and_time() {
        let mut left = wire::Slot {
            slot: 1,
            state: 2,
            creatures: Vec::new(),
            selected: "oteryn:creature.rat".to_owned(),
            bonus: 1,
            grade: 3,
            percent: 11,
            hunting_seconds: 7_140,
            option: 0,
            free_reroll_at: 99,
            chosen_tier: 1,
            allowed_tier: 1,
            selection_epoch: 7,
        };
        let right = left.clone();
        assert!(same_checkpoint(&left, &right));
        left.selection_epoch += 1;
        assert!(!same_checkpoint(&left, &right));
        left.selection_epoch -= 1;
        left.hunting_seconds -= 60;
        assert!(!same_checkpoint(&left, &right));
    }
}
