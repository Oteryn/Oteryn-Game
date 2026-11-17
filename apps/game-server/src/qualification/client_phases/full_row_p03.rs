//! P03 full-row player scenario, derived from the P03 audit/matrix row and the
//! durable Prey command/checkpoint owner:
//! 1. Admit the provisioned character and send `Query`; require an unlocked
//!    slot and the complete typed slot projection.
//! 2. Send `ListReroll`; require an accepted receipt plus a current-state
//!    delta containing a nonempty draw and an advanced free-reroll deadline.
//! 3. Send `Select` for a drawn creature that has a compiled-world spawn;
//!    require an active selection, bonus/grade/percent, 7,200 bonus seconds,
//!    and a new durable selection epoch.
//! 4. Spend wildcards on accepted bonus rerolls until the selected creature has an XP bonus.
//!    A refusal cannot qualify the reroll. Then send an out-of-range `Select`, require
//!    rejection, and prove by `Query` that the active selection was unchanged.
//! 5. Walk to the selected creature and use native ATTACK/corpse/loot actions.
//!    After the server hunting cadence, attack it again; require real combat
//!    and attributed XP award deltas plus a `Query` checkpoint with fewer bonus seconds while the
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

fn valid_bonus_reroll(accepted: bool, unchanged_on_refusal: bool) -> bool {
    accepted && unchanged_on_refusal
}

fn prey_effect_observed(timer_advanced: bool, awards_match: bool) -> bool {
    timer_advanced && awards_match
}

async fn hunt_with_xp_bonus(
    client: &mut oteryn_dev_client::DevClientSession,
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    selected: &wire::Slot,
    name: &str,
) -> TestResult<bool> {
    use sqlx::Connection;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    travel_to_spawn(client, world, spawns, &selected.selected).await?;
    let since: String = sqlx::query_scalar("SELECT clock_timestamp()::text").fetch_one(&mut database).await?;
    client_hunt::hunt_named(client, name).await?;
    type Award = (i16, i64, i16, i16, i64, i16, i32, i64, i64);
    let awards: Vec<Award> = sqlx::query_as(
        "SELECT m.prey_slot,m.prey_epoch,m.prey_grade,m.prey_percent,m.raw_share,
                m.xp_boost_percent,m.stamina_minutes,m.xp_awarded,x.level_before
         FROM game_party_kill_xp_receipts p JOIN game_party_kill_xp_members m USING(death_key)
         JOIN game_character_xp_receipts x ON x.reward_occurrence_id=m.reward_occurrence_id
         WHERE m.character_id=encode($1,'hex')::uuid AND p.creature_key=$2
           AND p.committed_at >= $3::text::timestamptz AND m.prey_slot IS NOT NULL
         ORDER BY p.committed_at,p.death_key LIMIT 2",
    ).bind(clients.characters[1].as_slice()).bind(&selected.selected).bind(since).fetch_all(&mut database).await?;
    let [(number, epoch, grade, percent, raw, boost, stamina, awarded, level)] = awards.as_slice() else {
        return Err(format!("P03 step=hunt-xp observed=awards:{awards:?};needed=one-selected-creature-award").into());
    };
    let level = u32::try_from(*level)?;
    let expected = |prey, premium| oteryn_gameplay_characters::hunting_experience::hunting_experience(
        *raw, prey, *boost, *stamina, premium, level,
    ).ok();
    Ok(*number == i16::from(selected.slot) && u64::try_from(*epoch)? == selected.selection_epoch
        && *grade == i16::from(selected.grade) && *percent == i16::from(selected.percent)
        && [false,true].into_iter().any(|premium| expected(*percent, premium) == Some(*awarded)
            && expected(0, premium).is_some_and(|base| base < *awarded)))
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
    let mut client = connect_world(clients, 0xa1).await?;
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

    let mut selected = selected;
    let mut balance = active.wildcard_balance;
    for _ in 0..8 {
        let response = command(&mut client, wire::Request::BonusReroll { slot: number }).await?;
        if !matches!(response.status, CommandStatus::Accepted | CommandStatus::DuplicateReplay) {
            let after = query(&mut client, "bonus-reroll-refusal-check").await?;
            return Err(format!("P03 step=bonus-reroll observed={:?},state-unchanged:{};needed=accepted-wildcard-reroll",
                response.status, same_checkpoint(&selected, slot(&after, number)?)).into());
        }
        let receipt = decoded(&response.payload, "bonus-reroll")?;
        let current = decoded(response.current.as_deref().ok_or("P03 step=bonus-reroll observed=missing-current-state")?,
            "bonus-reroll-current")?;
        let rerolled = slot(&current, number)?.clone();
        if !valid_bonus_reroll(receipt.status == 0 && receipt.wildcard_balance.checked_add(1) == Some(balance),
            rerolled.selected == selected.selected && rerolled.selection_epoch == selected.selection_epoch
                && (rerolled.bonus, rerolled.grade, rerolled.percent) != (selected.bonus, selected.grade, selected.percent))
        {
            return Err(format!("P03 step=bonus-reroll observed=receipt:{receipt:?}-state:{rerolled:?}").into());
        }
        balance = receipt.wildcard_balance;
        selected = rerolled;
        // XP has a per-kill durable award that identifies the exact selection and applied percentage.
        if selected.bonus == 3 {
            break;
        }
    }
    if selected.bonus != 3 {
        return Err("P03 step=bonus-reroll needed=experience-bonus-for-attributed-kill-proof".into());
    }

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

    let name = super::full_row_p02::creature_display_name(&selected.selected)?;
    let first_award = hunt_with_xp_bonus(&mut client, clients, world, &metadata.spawns, &selected, &name).await?;
    let until = tokio::time::Instant::now() + Duration::from_secs(61);
    while tokio::time::Instant::now() < until {
        client.service_liveness(Duration::from_millis(500)).await?;
        client.take_events();
    }
    let second_award = hunt_with_xp_bonus(&mut client, clients, world, &metadata.spawns, &selected, &name).await?;
    let checkpoint = query(&mut client, "effect-checkpoint").await?;
    let checkpoint = slot(&checkpoint, number)?.clone();
    if checkpoint.state != 2
        || checkpoint.selected != selected.selected
        || checkpoint.bonus != selected.bonus
        || checkpoint.percent != selected.percent
        || checkpoint.selection_epoch != selected.selection_epoch
        || !prey_effect_observed(checkpoint.hunting_seconds < selected.hunting_seconds, first_award && second_award)
    {
        return Err(format!("P03 step=effect-checkpoint observed={checkpoint:?}").into());
    }

    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("P03 step=logout observed=refused".into());
    }
    drop(client);
    let mut client = connect_world(clients, 0xa2).await?;
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
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("P03 step=final-logout observed=refused".into());
    }
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(360), verdict::observe(journey(clients))).await;
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
    fn rejected_prey_reroll_and_timer_only_effect_are_insufficient() {
        assert!(!valid_bonus_reroll(false, true));
        assert!(valid_bonus_reroll(true, true));
        assert!(!prey_effect_observed(true, false));
        assert!(prey_effect_observed(true, true));
    }

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
