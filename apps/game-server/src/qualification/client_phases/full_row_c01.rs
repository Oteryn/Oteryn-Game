//! C01 — melee attacks, block/armor, fight modes, and chase.
//!
//! Concrete player scenario derived from the C01 audit/matrix row and the
//! ATTACK owner (`gameplay_transport/attack.rs`):
//! 1. Admit the provisioned node-boot character, walk through the compiled
//!    world to an authored rat, and require a genuinely equipped hand weapon.
//! 2. For offensive, balanced, and defensive mode, send SET_FIGHT_MODES with
//!    STAND and then CHASE; after every accepted reply require the exact combat
//!    state delta, send ATTACK_TARGET, and observe a lower target-HP delta.
//! 3. During those encounters require server-attributed incoming damage and an
//!    equal ActorVitals HP delta while the character wears its ordinary armor;
//!    this is the observed armor/block result, never a hard-coded damage pass.
//! 4. Send ATTACK_TARGET naming the character itself; require the server's
//!    TargetNotACreature refusal and unchanged combat target/vitals.
//! 5. Kill a rat by native melee, require XP to increase, open its corpse, and
//!    move real rolled loot into the backpack (retrying only an empty loot roll).
//! 6. Clear the target (monster-target/follow disengage), logout, freshly admit
//!    with a different game session, and re-check the earned XP and exact loot
//!    inventory as durable state.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityDetail, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::attack::{AttackIntentDisposition, ChaseMode, FightMode, FightModes};
use std::collections::BTreeSet;

const CASES: &[&str] = &[
    "offensive-stand", "offensive-chase", "balanced-stand", "balanced-chase",
    "defensive-stand", "defensive-chase", "armor-defence", "kill-xp",
    "corpse-loot", "relog-xp-loot", "monster-target-follow-disengage",
];

struct Row { passed: BTreeSet<&'static str>, started: std::time::Instant }
impl Row {
    fn new() -> Self { Self { passed: BTreeSet::new(), started: std::time::Instant::now() } }
    fn pass(&mut self, case: &'static str, detail: impl std::fmt::Display) {
        if CASES.contains(&case) && self.passed.insert(case) {
            evidence(&format!("COMBAT_CHECK C01 {case} PASS {detail}"));
        }
    }
    fn finish(self, result: TestResult) {
        let error = result.as_ref().err().map(ToString::to_string);
        for case in CASES {
            if !self.passed.contains(case) {
                evidence(&format!("COMBAT_CHECK C01 {case} FAIL {}", error.as_deref().unwrap_or("required-action-not-run").replace([' ', '\n', '\r'], "-")));
            }
        }
        let pass = self.passed.len();
        let fail = CASES.len() - pass;
        evidence(&format!("COMBAT_SUMMARY row=C01 pass={pass} fail={fail} skipped=0"));
        let status = if error.is_none() && pass == CASES.len() { "PASS" } else { "FAIL" };
        if let Some(error) = &error { evidence(&format!("C01_GAP {error}")); }
        evidence(&format!("e2e_all scenario=C01.full status={status} seconds={:.3}", self.started.elapsed().as_secs_f64()));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = match verdict::indexed_timeout(Duration::from_secs(900), verdict::observe(journey(&mut row, clients))).await {
        Ok(result) => result,
        Err(_) => Err("step=journey observed=timeout need=responsive-combat-lifecycle".into()),
    };
    row.finish(result);
}

fn target(client: &DevClientSession) -> TestResult<oteryn_dev_client::EntityRef> {
    let here = client.world_spatial().actor_position;
    client.world_entities().and_then(|entities| entities.iter().find(|entry| {
        entry.kind == EntityKind::Creature && entry.position.floor == here.floor
            && entry.position.x.abs_diff(here.x).max(entry.position.y.abs_diff(here.y)) <= 1
    })).map(|entry| entry.entity).ok_or("step=target observed=no-adjacent-creature".into())
}

fn own(client: &DevClientSession) -> TestResult<oteryn_dev_client::EntityRef> {
    let entities = client.world_entities().ok_or("step=own observed=no-entity-view")?;
    entities.iter().find(|entry| entry.entity.identity == *entities.own_identity())
        .map(|entry| entry.entity).ok_or("step=own observed=own-entity-absent".into())
}

async fn progress(client: &mut DevClientSession) -> TestResult<(u64, u32)> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let reply = client.session_mut().gated_command(wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
        wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, &[1, 0, 0, 1]).await?;
    if reply.status != CommandStatus::Accepted || general_stats::validate(&reply.payload, general_stats::selected(client)).is_err() || reply.payload[0] != 1 {
        return Err(format!("step=xp-query observed={reply:?} need=GeneralStats-frame").into());
    }
    Ok((u64::from_le_bytes(reply.payload[1..9].try_into()?), u32::from_le_bytes(reply.payload[9..13].try_into()?)))
}

async fn mode_attack(client: &mut DevClientSession, world: &BundleWorld, modes: FightModes) -> TestResult<(u32, u32)> {
    let creature = target(client)?;
    let EntityDetail::Actor { health_percent: before, .. } = client.world_entities()
        .and_then(|entities| entities.get(&creature)).ok_or("step=mode observed=target-vanished")?.detail
    else { return Err("step=mode observed=target-without-health".into()); };
    let reply = client.session_mut().set_fight_modes(&modes).await?;
    if reply.disposition != AttackIntentDisposition::Ok || client.session_mut().combat_state().is_none_or(|state| state.modes != modes) {
        return Err(format!("step=mode observed={reply:?}/state={:?} need=accepted-exact-delta", client.session_mut().combat_state()).into());
    }
    let selected = client.session_mut().attack_target(Some(&creature)).await?;
    if selected.disposition != AttackIntentDisposition::Ok || client.session_mut().combat_state().is_none_or(|state| state.target != Some(creature)) {
        return Err(format!("step=attack observed={selected:?} need=accepted-target-delta").into());
    }
    let own = own(client)?;
    let starting_hp = client.actor_vitals().ok_or("step=armor observed=no-vitals")?.health;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(35);
    let mut target_damage = false;
    let mut attributed = None;
    loop {
        client.service_liveness(Duration::from_millis(50)).await?;
        for event in client.take_events() {
            if let SessionEvent::WorldSpatialEntities(delta) = &event {
                target_damage |= delta.value.update.iter().any(|entry| entry.entity == creature
                    && matches!(entry.detail, EntityDetail::Actor { health_percent, .. } if health_percent < before));
            }
            if let SessionEvent::Presentation(delta) = event {
                use oteryn_protocol_oteryn::presentation::PresentationEventKind;
                for shown in delta.value {
                    if let PresentationEventKind::ValueText { target, attacker: Some(attacker), primary, .. } = shown.kind
                        && target == own && attacker == creature && primary.value > 0 { attributed = Some(primary.value); }
                }
            }
        }
        if target_damage && attributed.is_some() { break; }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=damage observed=target:{target_damage}/incoming:{attributed:?} need=both-damage-deltas").into());
        }
    }
    let ending_hp = client.actor_vitals().ok_or("step=armor observed=post-vitals-absent")?.health;
    let shown = attributed.ok_or("step=armor observed=no-attributed-damage")?;
    let shown = u32::try_from(shown)?;
    if starting_hp.checked_sub(shown) != Some(ending_hp) {
        return Err(format!("step=armor observed=hp-{starting_hp}->{ending_hp}/shown-{shown} need=matching-observed-delta").into());
    }
    chase_or_stand(client, world, creature, modes.chase).await?;
    let cleared = client.session_mut().attack_target(None).await?;
    if cleared.disposition != AttackIntentDisposition::Ok { return Err(format!("step=disengage observed={cleared:?}").into()); }
    Ok((shown, starting_hp - ending_hp))
}

/// Step two tiles away from the held target, then watch the player's own position with no
/// further client step: CHASE must walk the player back next to the target, STAND must not move.
async fn chase_or_stand(
    client: &mut DevClientSession,
    world: &BundleWorld,
    creature: oteryn_dev_client::EntityRef,
    chase: ChaseMode,
) -> TestResult {
    let at = |client: &DevClientSession| -> TestResult<TilePos> {
        let entry = client.world_entities().and_then(|entities| entities.get(&creature)).ok_or("step=chase observed=target-vanished")?;
        tile(entry.position)
    };
    let gap = |a: TilePos, b: TilePos| if a.floor == b.floor { a.x.abs_diff(b.x).max(a.y.abs_diff(b.y)) } else { u16::MAX };
    let here = tile(client.world_spatial().actor_position)?;
    let target = at(client)?;
    let away = route(world, here, |tile| gap(tile, target) >= 3).ok_or("step=chase observed=no-route-away-from-target")?;
    walk(client, world, &away).await.map_err(|error| { verdict::record_error(verdict::IndexedError::as_error(&error)); verdict::context(format!("step=chase observed=step-away:{error}"), error) })?;
    let left = tile(client.world_spatial().actor_position)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    let mut moved = false;
    while tokio::time::Instant::now() < deadline {
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
        let now = tile(client.world_spatial().actor_position)?;
        moved |= now != left;
        if chase == ChaseMode::Chase && moved && gap(now, at(client)?) <= 1 {
            return Ok(());
        }
    }
    match chase {
        ChaseMode::Stand if !moved => Ok(()),
        ChaseMode::Stand => Err(format!("step=stand observed=player-moved-from-{left:?} need=no-server-step").into()),
        _ => Err(format!("step=chase observed=moved-{moved}-from-{left:?} need=server-steps-back-next-to-target").into()),
    }
}

fn inventory_signature(client: &DevClientSession) -> TestResult<Vec<(u32, u32)>> {
    let mut values: Vec<_> = client.inventory().ok_or("step=inventory observed=missing")?.entries.iter()
        .map(|entry| (entry.item_definition_ref.get(), entry.count.get())).collect();
    values.sort_unstable();
    Ok(values)
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(bytes, &pin, WorldId::decode(&clients.world)?, ChannelId::decode(&clients.channel)?)?;
    let mut client = connect_world(clients, 0xc1).await?;
    use oteryn_protocol_oteryn::item_view::EquipmentSlot;
    let equipment = &client
        .inventory()
        .ok_or("step=equipment observed=inventory-absent")?
        .equipment;
    let equipped = equipment.len();
    let hand = equipment.iter().any(|entry| {
        matches!(
            entry.slot,
            EquipmentSlot::RightHand | EquipmentSlot::LeftHand
        )
    });
    let armor = equipment.iter().any(|entry| {
        matches!(
            entry.slot,
            EquipmentSlot::Head
                | EquipmentSlot::Armor
                | EquipmentSlot::Legs
                | EquipmentSlot::Feet
        )
    });
    if !hand || !armor {
        return Err(format!("step=equipment observed=hand-{hand}/armor-{armor} need=legitimate-equipped-weapon-and-defence").into());
    }
    let mut defenses = Vec::new();
    for (name, fight) in [("offensive", FightMode::Offensive), ("balanced", FightMode::Balanced), ("defensive", FightMode::Defensive)] {
        for (suffix, chase) in [("stand", ChaseMode::Stand), ("chase", ChaseMode::Chase)] {
            travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await
                .map_err(|error| { verdict::record_error(verdict::IndexedError::as_error(&error)); verdict::context(format!("step={name}-{suffix} observed={error} need=walk-to-live-rat"), error) })?;
            let observed = mode_attack(&mut client, world, FightModes { fight_mode: fight, chase, secure: true }).await?;
            defenses.push(observed);
            let case = CASES.iter().copied().find(|case| *case == format!("{name}-{suffix}")).ok_or("mode case registry")?;
            row.pass(case, format!("accepted-state-and-target-hp-delta incoming={observed:?}"));
        }
    }
    // Equal shown and HP deltas say nothing about mitigation: only a server mark on a blocked or
    // armor-reduced hit proves the defence, and the attributed frames carry none.
    evidence(&format!("C01_GAP armor-defence equipment={equipped} attributed-hp-deltas={defenses:?} needed=server-mark-on-blocked-or-armor-reduced-hit"));
    let me = own(&client)?;
    let before_vitals = *client.actor_vitals().ok_or("step=refusal observed=no-vitals")?;
    let refused = client.session_mut().attack_target(Some(&me)).await?;
    if refused.disposition != AttackIntentDisposition::TargetNotACreature
        || client.actor_vitals() != Some(&before_vitals)
        || client.session_mut().combat_state().is_some_and(|state| state.target.is_some()) {
        return Err(format!("step=refusal observed={refused:?}/state={:?} need=TargetNotACreature-inert", client.session_mut().combat_state()).into());
    }
    let before_xp = progress(&mut client).await?;
    let before_loot = inventory_signature(&client)?;
    let mut after_loot = before_loot.clone();
    for _ in 0..4 {
        travel_to_spawn(&mut client, world, &metadata.spawns, "oteryn:creature.rat").await?;
        client_hunt::hunt(&mut client).await?;
        after_loot = inventory_signature(&client)?;
        if after_loot != before_loot { break; }
    }
    let after_xp = progress(&mut client).await?;
    if after_xp.0 <= before_xp.0 { return Err(format!("step=kill-xp observed={before_xp:?}->{after_xp:?} need=increased-xp").into()); }
    row.pass("kill-xp", format!("xp={}->{},level={}", before_xp.0, after_xp.0, after_xp.1));
    if after_loot == before_loot { return Err("step=corpse-loot observed=four-empty-rolls need=rolled-loot-inventory-delta".into()); }
    row.pass("corpse-loot", "corpse-use-and-item-move-inventory-delta");
    let clear = client.session_mut().attack_target(None).await?;
    if clear.disposition != AttackIntentDisposition::Ok || client.session_mut().combat_state().is_none_or(|state| state.target.is_some()) {
        return Err(format!("step=disengage observed={clear:?} need=cleared-combat-state").into());
    }
    row.pass("monster-target-follow-disengage", "target-selected-chase-exercised-and-cleared");
    let old_session = client.join_snapshot().game_session_id;
    if client.session_mut().logout().await?.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("step=relog observed=logout-refused".into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0xc2).await?;
    let durable_xp = progress(&mut fresh).await?;
    let durable_loot = inventory_signature(&fresh)?;
    if fresh.join_snapshot().game_session_id == old_session || durable_xp != after_xp || durable_loot != after_loot {
        return Err(format!("step=relog observed=session/xp/loot-mismatch xp={after_xp:?}->{durable_xp:?} need=durable-state").into());
    }
    row.pass("relog-xp-loot", "fresh-session-identical-xp-and-loot");
    let _ = fresh.session_mut().logout().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_required_checks_are_distinct() {
        assert_eq!(CASES.iter().copied().collect::<BTreeSet<_>>().len(), CASES.len());
    }
}
