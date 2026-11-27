//! P01 full-row player scenario, derived from the matrix/audit and the native
//! `charm_progression` owner:
//!
//! 1. Admit the node-boot Character and send QUERY; expect an accepted reply
//!    containing the complete charm view and its current durable Bestiary rows.
//! 2. Send ASSIGN for a still-locked major charm and a real Bestiary race;
//!    expect the explicit `CharmLocked` refusal and no assignment/balance delta.
//! 3. Hunt a placed Rat and verify the final Bestiary kill and mastery award.
//! 4. Send UNLOCK with the stage observed by QUERY; expect `Unlocked`, a
//!    one-stage increase, and exactly the advertised charm-point debit.
//! 5. Send ASSIGN for that unlocked charm and the completed race; expect
//!    `Assigned`, the race assignment delta, and `effect_active=true`.
//! 6. Walk back to a live creature, send ATTACK, and require a second real
//!    kill while the assignment is active, proving the production combat hook
//!    is traversed without losing the assignment.
//! 7. Send LOGOUT, obtain a fresh admission, then send QUERY; expect the kill
//!    counts, charm-point balance, unlocked stage, and assigned race to equal
//!    the pre-logout durable view.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::charm::{CharmAssignDisposition, CharmUnlockDisposition};
use oteryn_protocol_oteryn::charm_progression::{
    self as wire, CharmProgressionAction, CharmProgressionOutcome, CharmProgressionReply,
};
use std::num::NonZeroU32;

#[path = "full_row_p01_charm_fixture.rs"]
mod charm_fixture;

const DEADLINE: Duration = Duration::from_secs(1_800);

struct Row {
    started: std::time::Instant,
    step: &'static str,
}

impl Row {
    fn new() -> Self {
        Self {
            started: std::time::Instant::now(),
            step: "admission-query",
        }
    }

    fn at(&mut self, step: &'static str) {
        self.step = step;
    }

    fn finish(self, result: TestResult) {
        match &result {
            Ok(()) => evidence("P01_GAP status=PASS"),
            Err(error) => evidence(&format!(
                "P01_GAP status=FAIL step={} observed={}",
                self.step,
                compact(&error.to_string())
            )),
        }
        match result {
            Ok(()) => evidence(&format!(
                "SEAM_EVIDENCE e2e_all scenario=P01.full status=PASS seconds={:.3}",
                self.started.elapsed().as_secs_f64()
            )),
            Err(error) => evidence(&format!(
                "SEAM_EVIDENCE e2e_all scenario=P01.full status=FAIL seconds={:.3} evidence=step:{};observed:{};need:reachable-player-earned-charm-points-and-native-reply",
                self.started.elapsed().as_secs_f64(),
                self.step,
                compact(&error.to_string())
            )),
        }
    }
}

fn compact(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_:.,=".contains(character) {
                character
            } else {
                '_'
            }
        })
        .take(180)
        .collect()
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result =
        verdict::indexed_timeout(DEADLINE, verdict::observe(journey(&mut row, clients))).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err(format!("deadline while executing {}", row.step).into()),
    };
    row.finish(result);
}

pub(super) async fn logout_when_ready(client: &mut DevClientSession) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    verdict::Harness::global().bounded_wait(deadline, "logout-combat-lock");
    while client
        .session_mut()
        .combat_state()
        .is_none_or(|state| state.in_fight)
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("combat state did not clear before logout".into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
    }
    let (result, detail, _, _) = logout_once(client).await?;
    if result != Some(LogoutResult::Accepted) {
        return Err(format!("logout refused: {detail}").into());
    }
    Ok(())
}

pub(super) async fn close_relog_fixture(client: &mut DevClientSession) -> TestResult {
    let (cleanup, detail, _, _) = fixture_logout_retrying_busy(client).await?;
    if cleanup != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("P01 relog fixture cleanup refused: {detail}").into());
    }
    Ok(())
}

async fn query(
    client: &mut DevClientSession,
    frame: &str,
    frames: &mut String,
) -> TestResult<CharmProgressionReply> {
    let reply = client_row_p01::send(client, CharmProgressionAction::Query, frame, frames).await?;
    if reply.outcome != CharmProgressionOutcome::Query || reply.charms.charms.is_empty() {
        return Err(format!("query reply missing native views: {reply:?}").into());
    }
    Ok(reply)
}

fn count(reply: &CharmProgressionReply, race: NonZeroU32) -> u32 {
    reply
        .bestiary
        .iter()
        .find(|entry| entry.race == race)
        .map_or(0, |entry| entry.kill_count)
}

pub(super) fn rat_corpse_stage() -> TestResult<client_hunt::CorpseStage> {
    let items = provisioning::item_index()?;
    Ok(client_hunt::CorpseStage {
        fresh_ref: provisioning::item_reference(&items, 5964)?.get(),
        mature_ref: provisioning::item_reference(&items, 3994)?.get(),
    })
}

pub(super) async fn retreat_from_spawn(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawn: TilePos,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let occupied = client_row_p01::occupied_rat_route_tiles(client);
    // Leave native eleven-tile creature sight before respawn.
    let retreat = route_avoiding(
        world,
        from,
        |at| at.floor != spawn.floor || at.x.abs_diff(spawn.x).max(at.y.abs_diff(spawn.y)) > 11,
        |at| !occupied.contains(&at),
    )
    .ok_or_else(|| format!("no walkable retreat from hunt at {from:?}"))?;
    client_row_p01::walk_rat_route(client, world, &retreat, true).await
}

pub(super) async fn authored_target(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    key: &str,
    name: &str,
    required_spawn: Option<TilePos>,
) -> TestResult<(TilePos, oteryn_dev_client::WorldSpatialEntity)> {
    use oteryn_dev_client::EntityKind;
    let from = tile(client.world_spatial().actor_position)?;
    let mut occupied = client_row_p01::occupied_rat_route_tiles(client);
    // This landing was reported occupied by the previous live approach.
    occupied.insert(TilePos {
        x: 32444,
        y: 32178,
        floor: -7,
    });
    let mut points = std::collections::BTreeSet::new();
    for source in &spawns.sources {
        for point in &source.points {
            let at = TilePos {
                x: point.x,
                y: point.y,
                floor: source.floor,
            };
            occupied.insert(at);
            if spawns
                .creatures
                .get(point.creature as usize)
                .is_some_and(|creature| creature.key == key)
            {
                points.insert(at);
            }
        }
    }
    let adjacent = |a: TilePos, b: TilePos| {
        a.floor == b.floor && a.x.abs_diff(b.x).max(a.y.abs_diff(b.y)) == 1
    };
    let path = route_avoiding(
        world,
        from,
        |at| {
            points.iter().any(|&point| {
                required_spawn.is_none_or(|required| point == required) && adjacent(at, point)
            }) && world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| !tile.protection_zone())
        },
        |at| !occupied.contains(&at),
    )
    .ok_or_else(|| format!("no first-attempt route to an authored {key} spawn"))?;
    let goal = path
        .iter()
        .try_fold(from, |at, &direction| destination(world, at, direction))
        .ok_or("authored spawn route has no landing")?;
    let point = points
        .iter()
        .find(|&&point| {
            required_spawn.is_none_or(|required| point == required) && adjacent(goal, point)
        })
        .copied()
        .ok_or("route does not end adjacent to its authored spawn")?;
    client_row_p01::walk_rat_route(client, world, &path, required_spawn.is_none()).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    verdict::Harness::global().bounded_wait(deadline, "authored-creature-visibility");
    loop {
        let target = {
            let mut targets = client
                .world_entities()
                .into_iter()
                .flat_map(|view| view.iter())
                .filter(|entity| {
                    entity.kind == EntityKind::Creature
                        && entity
                            .display_name
                            .as_ref()
                            .and_then(|text| text.as_str().ok())
                            == Some(name)
                })
                .filter(|entity| tile(entity.position).is_ok_and(|at| at == point));
            let target = targets.next().copied();
            if targets.next().is_some() {
                return Err(format!("ambiguous wire {name} at authored spawn {point:?}").into());
            }
            target
        };
        if let Some(target) = target {
            evidence(&format!(
                "P01_CHECK authored-spawn PASS key={key} point={point:?} target={:?} wire_position={:?}",
                target.entity, target.position
            ));
            return Ok((point, target));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                format!("authored {key} is absent from its selected spawn {point:?}").into(),
            );
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

async fn prepare_unarmed(client: &mut DevClientSession) -> TestResult {
    use oteryn_dev_client::{ItemMoveDestination, ItemMoveIntent};
    use oteryn_protocol_oteryn::item_view::{EquipmentSlot, ItemMoveOutcome};

    let hands = [EquipmentSlot::LeftHand, EquipmentSlot::RightHand];
    let equipment = client
        .inventory()
        .ok_or("unarmed fixture inventory absent")?;
    let held: Vec<_> = equipment
        .equipment
        .iter()
        .filter(|entry| hands.contains(&entry.slot))
        .map(|entry| (entry.slot, entry.item.handle))
        .collect();
    for (slot, handle) in held {
        let result = client
            .move_item(&ItemMoveIntent {
                source: handle,
                destination: ItemMoveDestination::MainBackpack,
            })
            .await?;
        if result.outcome != ItemMoveOutcome::Moved {
            return Err(format!("unarmed fixture refused to stow {slot:?}: {result:?}").into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            if client.inventory().is_some_and(|inventory| {
                inventory.equipment.iter().all(|entry| entry.slot != slot)
                    && inventory.entries.iter().any(|item| item.handle == handle)
            }) {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("unarmed fixture did not stow {slot:?}: {result:?}").into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
        }
        evidence(&format!("P01_FIXTURE stowed hand={slot:?}"));
    }
    evidence("P01_CHECK unarmed-baseline PASS hands=empty");
    Ok(())
}

pub(super) async fn hunt_exact_creature(
    client: &mut DevClientSession,
    target: oteryn_dev_client::WorldSpatialEntity,
    key: &str,
    race: NonZeroU32,
    corpse_ref: u32,
    stage: Option<client_hunt::CorpseStage>,
    verify_principal: Option<&SeamClients<'_>>,
) -> TestResult<full_row_p05::KillSample> {
    use oteryn_dev_client::{EntityKind, SessionEvent};
    use oteryn_protocol_oteryn::analyser::{AnalyserFact, AnalyserImpact};
    use oteryn_protocol_oteryn::damage_element::DamageElement;
    let old_corpses: Vec<_> = client
        .world_entities()
        .into_iter()
        .flat_map(|entities| {
            entities
                .iter()
                .filter(|entity| entity.kind == EntityKind::Corpse)
                .map(|entity| entity.entity)
        })
        .collect();
    client.take_events();
    let attack = client
        .session_mut()
        .attack_target(Some(&target.entity))
        .await?;
    if attack.disposition != oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok {
        return Err(format!("step=hunt-attack race={race} reply={attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    verdict::Harness::global().bounded_wait(deadline, "targeted-combat-and-credit");
    let mut last_position = target.position;
    let mut kills = Vec::new();
    let mut fire_damage = false;
    let mut enflame_log_count = 0;
    let mut experience = Vec::new();
    let (corpse, raw_xp, gained_xp) = loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        for event in client.take_events() {
            let facts = match event {
                SessionEvent::ChatLine(delta) => {
                    if let oteryn_protocol_oteryn::chat::ChatLine::System { text } = delta.value
                        && text.ends_with(" (enflame charm)")
                    {
                        if key != "oteryn:creature.troll"
                            || text
                                != "A troll loses 1 hitpoint due to your attack. (enflame charm)"
                        {
                            return Err(format!("unexpected Enflame Server Log: {text:?}").into());
                        }
                        enflame_log_count += 1;
                    }
                    continue;
                }

                SessionEvent::WorldSpatialEntities(delta) => {
                    if let Some(updated) = delta
                        .value
                        .update
                        .iter()
                        .find(|entry| entry.entity == target.entity)
                    {
                        last_position = updated.position;
                    }
                    continue;
                }
                SessionEvent::AnalyserFacts(delta) => {
                    if verify_principal.is_some() {
                        return Err("P01.client received an un-attributed analyser batch".into());
                    }
                    delta.value
                }
                SessionEvent::AnalyserAttributedFacts(delta) => {
                    for entry in &delta.value {
                        let Some(source) = &entry.charm else { continue };
                        if source.charm_key != "oteryn:charm.enflame"
                            || source.target != target.entity
                        {
                            return Err(format!(
                                "unexpected charm source in unarmed hunt: {source:?}"
                            )
                            .into());
                        }
                        let AnalyserFact::Impact(AnalyserImpact::Damage {
                            element: DamageElement::Fire,
                            value,
                        }) = entry.fact
                        else {
                            return Err(format!("non-fire Enflame impact: {entry:?}").into());
                        };
                        if key == "oteryn:creature.troll" && value.get() != 1 {
                            return Err(
                                format!("Enflame Troll impact expected 1, got {value}").into()
                            );
                        }
                        evidence(&format!(
                            "P01_CHECK enflame-impact PASS charm={} damage={value} target={:?}",
                            source.charm_key, source.target
                        ));
                        if fire_damage {
                            return Err("duplicate attributed Enflame impact".into());
                        }
                        fire_damage = true;
                    }
                    delta.value.into_iter().map(|entry| entry.fact).collect()
                }
                _ => continue,
            };
            for fact in facts {
                match fact {
                    AnalyserFact::Kill { race } => kills.push(race),
                    AnalyserFact::Impact(AnalyserImpact::Damage {
                        element: DamageElement::Fire,
                        ..
                    }) if key == "oteryn:creature.rat" => fire_damage = true,
                    AnalyserFact::Experience { raw, gained } => experience.push((raw, gained)),
                    _ => {}
                }
            }
        }
        if !kills.is_empty() && kills != [Some(race)] || experience.len() > 1 {
            client.session_mut().attack_target(None).await?;
            return Err(format!(
                "step=hunt-credit expected={key}/{race} observed={kills:?} experience={experience:?}"
            )
            .into());
        }
        if key == "oteryn:creature.troll" && enflame_log_count > 1 {
            return Err("duplicate Enflame Server Log".into());
        }
        if (key != "oteryn:creature.troll" || !fire_damage || enflame_log_count == 1)
            && let [Some(_)] = kills.as_slice()
            && let [(raw, gained)] = experience.as_slice()
            && let Some(corpse) = client.world_entities().and_then(|entities| {
                if entities.get(&target.entity).is_some() {
                    return None;
                }
                entities
                    .iter()
                    .find(|entry| {
                        entry.kind == EntityKind::Corpse
                            && entry.position == last_position
                            && !old_corpses.contains(&entry.entity)
                            && matches!(
                                entry.detail,
                                oteryn_dev_client::EntityDetail::Object { item_definition_ref, .. }
                                | oteryn_dev_client::EntityDetail::ObjectWithAppearance { item_definition_ref, .. }
                                if item_definition_ref == corpse_ref
                            )
                    })
                    .copied()
            })
        {
            break (corpse, *raw, *gained);
        }
        if tokio::time::Instant::now() >= deadline {
            client.session_mut().attack_target(None).await?;
            return Err(format!("step=hunt-credit race={race} missing target corpse, kill credit or XP; kills={kills:?} experience={experience:?}").into());
        }
    };
    if key == "oteryn:creature.troll" {
        if (enflame_log_count == 1) != fire_damage {
            return Err("Enflame Server Log did not match an attributed impact".into());
        }
        if fire_damage {
            evidence(
                "P01_CHECK enflame-server-log PASS A troll loses 1 hitpoint due to your attack. (enflame charm)",
            );
        }
    }
    client.session_mut().attack_target(None).await?;
    if let Some(clients) = verify_principal {
        let principal = clients.characters[1];
        let death = committed_hunt_death(
            client,
            clients,
            Some(principal),
            key,
            target.entity,
            deadline,
        )
        .await?;
        if death.shared
            || death.awards.len() != 1
            || death.awards.get(&principal) != Some(&gained_xp)
            || corpse.entity.identity != death.actor.corpse_identity()
        {
            return Err("wire corpse or XP did not match the hunter's exact committed kill".into());
        }
        evidence(&format!(
            "P01_CHECK exact-rat-death PASS target={:?} corpse={:?}",
            target.entity, corpse.entity
        ));
    }
    let corpse = if let Some(stage) = stage {
        client_hunt::wait_for_corpse_stage(client, corpse, stage).await?
    } else {
        corpse
    };
    let handle = match corpse.detail {
        oteryn_dev_client::EntityDetail::Object { item_handle, .. }
        | oteryn_dev_client::EntityDetail::ObjectWithAppearance { item_handle, .. } => item_handle,
        _ => None,
    }
    .ok_or("Rat corpse has no server-issued item handle")?;
    let used = if stage.is_some() {
        client_hunt::use_rat_corpse(client, handle).await?
    } else {
        client.use_item(handle).await?
    };
    if used.status != CommandStatus::Accepted
        || used.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        return Err("hunt corpse USE refused".into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while client
        .open_container()
        .is_none_or(|view| view.container_handle != Some(handle))
    {
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
        if tokio::time::Instant::now() >= deadline {
            return Err("hunt corpse container delta missing".into());
        }
    }
    let entries = client
        .open_container()
        .ok_or("corpse view")?
        .entries
        .clone();
    if verify_principal.is_some() && key == "oteryn:creature.rat" && entries.is_empty() {
        return Err("Rat corpse is empty; strict loot transfer cannot be proved".into());
    }
    if let Some(item) = entries.first() {
        let before = client
            .inventory()
            .ok_or("hunt backpack")?
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref == item.item_definition_ref)
            .map(|entry| u64::from(entry.count.get()))
            .sum::<u64>();
        let corpse_before = entries
            .iter()
            .filter(|entry| entry.item_definition_ref == item.item_definition_ref)
            .map(|entry| u64::from(entry.count.get()))
            .sum::<u64>();
        let moved = client
            .move_item(&oteryn_dev_client::ItemMoveIntent {
                source: item.handle,
                destination: oteryn_dev_client::ItemMoveDestination::MainBackpack,
            })
            .await?;
        if moved.outcome != oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved {
            return Err(format!("hunt loot move: {moved:?}").into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let after = client
                .inventory()
                .ok_or("hunt inventory delta")?
                .entries
                .iter()
                .filter(|entry| entry.item_definition_ref == item.item_definition_ref)
                .map(|entry| u64::from(entry.count.get()))
                .sum::<u64>();
            let corpse_after = client
                .open_container()
                .ok_or("loot corpse delta")?
                .entries
                .iter()
                .filter(|entry| entry.item_definition_ref == item.item_definition_ref)
                .map(|entry| u64::from(entry.count.get()))
                .sum::<u64>();
            if after == before + u64::from(item.count.get())
                && corpse_after + u64::from(item.count.get()) == corpse_before
            {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err("hunt loot inventory conservation failed".into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
        }
    }
    Ok(full_row_p05::KillSample {
        race,
        raw_xp,
        gained_xp,
        fire_damage,
    })
}

pub(super) async fn prepare_hunter(
    clients: &SeamClients<'_>,
    scenario: &str,
    tag: u8,
) -> TestResult<DevClientSession> {
    use oteryn_protocol_oteryn::attack::{
        AttackIntentDisposition, ChaseMode, FightMode, FightModes,
    };
    use oteryn_protocol_oteryn::cyclopedia_character as stats;

    let reason = "ordinary leveling does not fit the timed Bestiary hunts";
    character_prep::declare_seeded(scenario, "level-80-sorcerer", reason);
    let mut setup = connect_world(clients, tag).await?;
    let before = setup.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(before.as_bytes()),
        "--profile",
        "p01-sorcerer".to_owned(),
    )
    .await?;
    logout_when_ready(&mut setup).await?;
    drop(setup);
    charm_fixture::seed(clients, scenario == "P01.full").await?;
    let mut client = connect_world(clients, 0xa4).await?;
    if client.join_snapshot().game_session_id == before {
        return Err(format!("{scenario} fixture re-admission reused the setup session").into());
    }
    let reply = client
        .session_mut()
        .gated_command(
            stats::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            stats::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[stats::CharacterTab::GeneralStats as u8, 0, 0, 1],
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || general_stats::validate(&reply.payload, general_stats::selected(&client))?
            != stats::vocation_id("sorcerer")
        || reply.payload.first() != Some(&(stats::CharacterTab::GeneralStats as u8))
    {
        return Err("P01 requires the admitted Sorcerer GeneralStats view".into());
    }
    let xp = u64::from_le_bytes(reply.payload[1..9].try_into()?);
    let level = u32::from_le_bytes(reply.payload[9..13].try_into()?);
    if (xp, level) != (7_915_800, 80) {
        return Err(format!("P01 seeded Sorcerer stats xp={xp} level={level}").into());
    }
    let vitals = client
        .actor_vitals()
        .ok_or("P01 seeded Sorcerer vitals absent")?;
    if (vitals.max_health, vitals.max_mana) != (545, 2250) {
        return Err(format!("P01 seeded Sorcerer max vitals {vitals:?}").into());
    }
    evidence(&format!(
        "P01_CHECK hunter-profile PASS vocation=sorcerer level={level} xp={xp} max_health=545 max_mana=2250"
    ));
    let modes = FightModes {
        fight_mode: FightMode::Offensive,
        chase: ChaseMode::Chase,
        secure: true,
    };
    if client
        .session_mut()
        .combat_state()
        .is_none_or(|state| state.modes == modes)
    {
        return Err("P01 requires a distinct initial fight-mode state".into());
    }
    let reply = client.session_mut().set_fight_modes(&modes).await?;
    require_mechanic_acceptance(reply.status, "P01 offensive chase")?;
    if reply.disposition != AttackIntentDisposition::Ok {
        return Err(format!("P01 offensive chase refused: {reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while client
        .session_mut()
        .combat_state()
        .is_none_or(|state| state.modes != modes)
    {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "P01 accepted offensive chase but observed combat state {:?}",
                client.session_mut().combat_state()
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    evidence("P01_CHECK hunt-modes PASS offensive-chase");
    Ok(client)
}

async fn journey(row: &mut Row, clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = prepare_hunter(clients, "P01.full", 0xa2).await?;
    if !client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1)
    {
        return Err("admission omitted capability 67".into());
    }
    let mut frames = String::new();
    let initial = query(&mut client, "p01-full-initial.json", &mut frames).await?;
    evidence("P01_CHECK admission-query PASS accepted-native-views");

    row.at("player-earned-kill-count");
    let rat_stage = rat_corpse_stage()?;
    let (index, (rat, rat_points)) = full_row_p05::canonical_races()?
        .iter()
        .enumerate()
        .find(|(_, (definition, _))| definition.key() == "oteryn:creature.rat")
        .ok_or("Rat absent from Bestiary")?;
    let race = NonZeroU32::new(u32::try_from(index)? + 1).ok_or("Rat race index")?;
    if rat.final_kill_threshold() != 250
        || *rat_points != 5
        || count(&initial, race) != 249
        || initial.charms.charm_points_available != 400
    {
        return Err(format!(
            "incorrect fixture baseline: Rat kills={} points={}",
            count(&initial, race),
            initial.charms.charm_points_available
        )
        .into());
    }
    prepare_unarmed(&mut client).await?;
    let (_, target) =
        authored_target(&mut client, world, &metadata.spawns, rat.key(), "Rat", None).await?;
    let first_kill = hunt_exact_creature(
        &mut client,
        target,
        rat.key(),
        race,
        rat_stage.fresh_ref,
        Some(rat_stage),
        None,
    )
    .await?;
    let current = query(&mut client, "p01-full-mastery.json", &mut frames).await?;
    let mut expected_mastery = initial.clone();
    expected_mastery
        .bestiary
        .iter_mut()
        .find(|entry| entry.race == race)
        .ok_or("missing Rat fixture entry")?
        .kill_count = 250;
    expected_mastery.charms.charm_points_available = 405;
    if first_kill.race != race
        || first_kill.raw_xp != 5
        || first_kill.fire_damage
        || current != expected_mastery
    {
        return Err(format!(
            "incorrect Rat kill or mastery award: kill={first_kill:?} kills={} points={}",
            count(&current, race),
            current.charms.charm_points_available
        )
        .into());
    }
    evidence("P01_CHECK kill-count-and-mastery PASS kills=249->250 charm_points=400->405");

    let catalogue = crate::content::charm_source::CanonicalCharmCatalogue::embedded()?;
    let fire_charm = catalogue
        .catalogue()
        .definitions()
        .position(|entry| entry.key.as_str() == "oteryn:charm.enflame")
        .ok_or("step=charm-proc canonical Enflame missing")?;
    let (charm, stage, cost) = current
        .charms
        .charms
        .iter()
        .enumerate()
        .filter(|(index, charm)| {
            *index == fire_charm && charm.unlocked_stage == 0 && charm.next_stage_cost > 0
        })
        .min_by_key(|(_, charm)| charm.next_stage_cost)
        .and_then(|(index, charm)| {
            Some((
                NonZeroU32::new(u32::try_from(index).ok()?.checked_add(1)?)?,
                charm.unlocked_stage,
                charm.next_stage_cost,
            ))
        })
        .ok_or("query has no locked charm with a stage cost")?;

    row.at("locked-assignment-refusal");
    let refused = client_row_p01::send(
        &mut client,
        CharmProgressionAction::Assign { charm, race },
        "p01-full-locked.json",
        &mut frames,
    )
    .await?;
    if refused.outcome != CharmProgressionOutcome::Assign(CharmAssignDisposition::CharmLocked)
        || refused.fee_gold != 0
        || refused.charms != current.charms
        || refused.bestiary != current.bestiary
    {
        return Err(format!("locked ASSIGN reply/delta mismatch: {refused:?}").into());
    }
    evidence("P01_CHECK locked-assignment-refusal PASS disposition=CharmLocked");

    row.at("unlock-stage");
    let unlocked = client_row_p01::send(
        &mut client,
        CharmProgressionAction::Unlock {
            charm,
            expected_stage: stage,
        },
        "p01-full-unlocked.json",
        &mut frames,
    )
    .await?;
    let mut expected_unlocked = expected_mastery.clone();
    let charm_index = usize::try_from(charm.get() - 1)?;
    expected_unlocked.outcome = CharmProgressionOutcome::Unlock(CharmUnlockDisposition::Unlocked);
    expected_unlocked.charms.charm_points_available = 5;
    expected_unlocked.charms.minor_charm_echoes_available = expected_mastery
        .charms
        .minor_charm_echoes_available
        .checked_add(50)
        .ok_or("minor charm echo balance overflow")?;
    expected_unlocked.charms.charms[charm_index].unlocked_stage = 1;
    expected_unlocked.charms.charms[charm_index].next_stage_cost = 600;
    if cost != 400 || unlocked != expected_unlocked {
        return Err(format!("UNLOCK reply/delta mismatch: {unlocked:?}").into());
    }
    evidence("P01_CHECK unlock-stage PASS stage=0->1 points=405->5 minor_echoes_delta=50");

    row.at("assign-incomplete-refusal");
    let index = full_row_p05::canonical_races()?
        .iter()
        .position(|(definition, _)| definition.key() == charm_fixture::INCOMPLETE)
        .ok_or("fixture refusal race missing from catalogue")?;
    let incomplete = NonZeroU32::new(u32::try_from(index)? + 1).ok_or("incomplete race index")?;
    if !unlocked
        .bestiary
        .iter()
        .any(|entry| entry.race == incomplete && entry.kill_count == 1 && !entry.is_complete())
    {
        return Err("P01 expected a known, incomplete Bestiary entry".into());
    }
    let mut expected_incomplete = expected_unlocked.clone();
    expected_incomplete.outcome =
        CharmProgressionOutcome::Assign(CharmAssignDisposition::RaceStageTooLow);
    let incomplete_reply = client_row_p01::send(
        &mut client,
        CharmProgressionAction::Assign {
            charm,
            race: incomplete,
        },
        "p01-full-incomplete.json",
        &mut frames,
    )
    .await?;
    if incomplete_reply != expected_incomplete {
        return Err(format!("incomplete ASSIGN reply/delta mismatch: {incomplete_reply:?}").into());
    }
    evidence(
        "P01_CHECK assign-incomplete-refusal PASS disposition=RaceStageTooLow state=unchanged",
    );

    row.at("assign-unlocked");
    // Rat's one-point Enflame base damage truncates to zero under mitigation.
    // Troll is the mastered fixture race for the positive fire-damage assertion.
    let troll_key = "oteryn:creature.troll";
    let (troll_index, (troll, _)) = full_row_p05::canonical_races()?
        .iter()
        .enumerate()
        .find(|(_, (definition, _))| definition.key() == troll_key)
        .ok_or("Troll absent from Bestiary")?;
    let race = NonZeroU32::new(u32::try_from(troll_index)? + 1).ok_or("Troll race index")?;
    if count(&unlocked, race) != troll.final_kill_threshold() {
        return Err("Troll fixture must be mastered before the Enflame assignment".into());
    }
    let troll_corpse = provisioning::item_reference(&provisioning::item_index()?, 5960)?.get();
    let assigned = client_row_p01::send(
        &mut client,
        CharmProgressionAction::Assign { charm, race },
        "p01-full-assigned.json",
        &mut frames,
    )
    .await?;
    let mut expected_assigned = expected_unlocked.clone();
    expected_assigned.outcome = CharmProgressionOutcome::Assign(CharmAssignDisposition::Assigned);
    expected_assigned.charms.charms[charm_index].assigned_race = Some(race);
    expected_assigned.charms.charms[charm_index].effect_active = true;
    if assigned != expected_assigned {
        return Err(format!("ASSIGN reply/delta mismatch: {assigned:?}").into());
    }
    evidence("P01_CHECK assign-unlocked PASS assignment-and-active-hook");

    row.at("assigned-combat-hook");
    if client.inventory().is_none_or(|inventory| {
        inventory.equipment.iter().any(|item| {
            matches!(
                item.slot,
                oteryn_protocol_oteryn::item_view::EquipmentSlot::LeftHand
                    | oteryn_protocol_oteryn::item_view::EquipmentSlot::RightHand
            )
        })
    }) {
        return Err("step=charm-proc need=unarmed-physical-baseline-to-isolate-Enflame".into());
    }
    row.at("enflame-attribution");
    let capability = oteryn_protocol_oteryn::analyser::CAPABILITY_ANALYSER_CHARM_SOURCE_V1;
    if !client.selected_capabilities().contains(&capability) {
        return Err("step=enflame-attribution missing negotiated capability 625".into());
    }
    let (spawn, troll) = authored_target(
        &mut client,
        world,
        &metadata.spawns,
        troll_key,
        "Troll",
        None,
    )
    .await?;
    let target_identity = troll
        .entity
        .identity
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    // Force one proc, then misses for the rest of this exact Troll's hunt so
    // the row can require one impact and one Server Log line.
    let rolls = format!("{target_identity} {} 0:1,9999:63", troll.entity.generation);
    request_fixture_control(
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        "--force-enflame",
        rolls,
    )
    .await?;
    evidence(&format!(
        "P01_FIXTURE enflame-rolls target={:?}",
        troll.entity
    ));
    let kill = hunt_exact_creature(
        &mut client,
        troll,
        troll_key,
        race,
        troll_corpse,
        None,
        None,
    )
    .await?;
    if kill.race != race || kill.raw_xp != 20 {
        return Err(format!("incorrect Troll kill credit: {kill:?}").into());
    }
    retreat_from_spawn(&mut client, world, spawn).await?;
    let after_hook = query(&mut client, "p01-full-after-hook.json", &mut frames).await?;
    let mut expected_durable = expected_assigned;
    expected_durable.outcome = CharmProgressionOutcome::Query;
    if after_hook != expected_durable {
        return Err(
            format!("assigned combat hook changed unexpected state: {after_hook:?}").into(),
        );
    }
    evidence("P01_CHECK assigned-race-kill PASS analyser-credit-and-active-assignment");

    row.at("wait-for-logout-block-and-relog");
    let old_session = client.join_snapshot().game_session_id;
    logout_when_ready(&mut client).await?;
    evidence("P01_CHECK logout-block-cleared PASS first-attempt-logout-accepted");
    drop(client);
    let mut fresh = connect_world(clients, 0xa3).await?;
    if fresh.join_snapshot().game_session_id == old_session {
        return Err("fresh admission reused the old game session".into());
    }
    let durable = query(&mut fresh, "p01-full-durable.json", &mut frames).await?;
    if durable != expected_durable {
        return Err(format!("fresh QUERY lost durable state: {durable:?}").into());
    }
    evidence("P01_CHECK logout-fresh-admission PASS exact-durable-views");
    use std::fmt::Write as _;
    writeln!(frames, "session={}", uuid_text(old_session.as_bytes()))?;
    writeln!(
        frames,
        "durable_session={}",
        uuid_text(fresh.join_snapshot().game_session_id.as_bytes())
    )?;
    writeln!(frames, "rat={}", first_kill.race)?;
    writeln!(frames, "troll={race}")?;
    writeln!(frames, "incomplete={incomplete}")?;
    writeln!(frames, "charm={charm}")?;
    client_row_p01::capture("p01-full-expected.txt", frames.as_bytes())?;
    close_relog_fixture(&mut fresh).await?;
    row.at("enflame-attribution");
    if !kill.fire_damage {
        return Err("no attributed Enflame proc on the selected Troll".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_token_is_single_line_and_bounded() {
        let token = compact("a reply with spaces\nand punctuation!?/x");
        assert!(!token.contains([' ', '\n']));
        assert!(token.len() <= 180);
    }
}

#[cfg(test)]
#[test]
fn row_01_publishes_a_gap_for_an_unproven_journey() {
    assert!(
        include_str!("full_row_p01.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(production, _)| production.contains("P01_GAP status="))
    );
}

#[cfg(test)]
#[test]
fn review_p01_guards_the_journey() {
    assert!(
        include_str!("full_row_p01.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(body, _)| body.contains("hunt_exact_creature"))
    );
}

#[cfg(test)]
#[test]
fn review_2_p01() {
    assert!(
        include_str!("full_row_p01.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(body, _)| body.contains("kill-count-and-mastery"))
    );
}
