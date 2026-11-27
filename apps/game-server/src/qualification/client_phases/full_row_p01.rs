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

async fn command(
    client: &mut DevClientSession,
    action: CharmProgressionAction,
) -> TestResult<CharmProgressionReply> {
    let payload = wire::encode_charm_progression_request(&action)
        .map_err(|error| format!("encode {action:?}: {error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1,
            wire::COMMAND_TYPE_CHARM_PROGRESSION,
            &payload,
        )
        .await?;
    require_mechanic_acceptance(response.status, &format!("CHARM_PROGRESSION {action:?}"))?;
    wire::decode_charm_progression_reply(&response.payload)
        .map_err(|error| format!("decode {action:?}: {error:?}").into())
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

async fn query(client: &mut DevClientSession) -> TestResult<CharmProgressionReply> {
    let reply = command(client, CharmProgressionAction::Query).await?;
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

pub(super) async fn prepare_rat_ground(
    client: &mut DevClientSession,
    world: &BundleWorld,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let occupied = client_row_p01::occupied_rat_route_tiles(client);
    let open = |at: TilePos| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
            && !occupied.contains(&at)
    };
    let path = route_avoiding(
        world,
        from,
        |at| {
            open(at)
                && [
                    StepDirection::North,
                    StepDirection::East,
                    StepDirection::South,
                    StepDirection::West,
                ]
                .into_iter()
                .filter(|&direction| destination(world, at, direction).is_some_and(open))
                .count()
                    >= 2
        },
        |at| !occupied.contains(&at),
    )
    .ok_or("no reachable free ground for the Rat fixture")?;
    client_row_p01::walk_rat_route(client, world, &path).await
}

pub(super) async fn place_rat(
    client: &mut DevClientSession,
) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
    place_creature(client, "oteryn:creature.rat").await
}

async fn place_creature(
    client: &mut DevClientSession,
    key: &str,
) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let answer = request_spawn(session, key.to_owned()).await?;
    let target = parse_spawned_entity(&answer)?;
    evidence(&format!("P01_FIXTURE placed key={key} target={target:?}"));
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    verdict::Harness::global().bounded_wait(deadline, "fixture-rat-visibility");
    loop {
        if let Some(entity) = client
            .world_entities()
            .and_then(|view| view.get(&target))
            .copied()
        {
            let actor = client.world_spatial().actor_position;
            if entity.kind != oteryn_dev_client::EntityKind::Creature
                || entity.position.floor != actor.floor
                || entity
                    .position
                    .x
                    .abs_diff(actor.x)
                    .max(entity.position.y.abs_diff(actor.y))
                    != 1
            {
                return Err(format!("incorrect {key} fixture placement: {entity:?}").into());
            }
            return Ok(entity);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("{key} fixture entity {target:?} absent from wire view").into());
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

async fn hunt_exact_creature(
    client: &mut DevClientSession,
    key: &str,
    race: NonZeroU32,
    corpse_ref: u32,
    stage: Option<client_hunt::CorpseStage>,
) -> TestResult<full_row_p05::KillSample> {
    use oteryn_dev_client::{EntityKind, SessionEvent};
    use oteryn_protocol_oteryn::analyser::AnalyserFact;
    let target = place_creature(client, key).await?;
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
    let mut experience = Vec::new();
    let (corpse, raw_xp, gained_xp) = loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        for event in client.take_events() {
            let facts = match event {
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
                SessionEvent::AnalyserFacts(delta) => delta.value,
                SessionEvent::AnalyserAttributedFacts(delta) => {
                    delta.value.into_iter().map(|fact| fact.fact).collect()
                }
                _ => continue,
            };
            for fact in facts {
                match fact {
                    AnalyserFact::Kill { race } => kills.push(race),
                    AnalyserFact::Impact(
                        oteryn_protocol_oteryn::analyser::AnalyserImpact::Damage {
                            element: oteryn_protocol_oteryn::damage_element::DamageElement::Fire,
                            value,
                        },
                    ) => {
                        evidence(&format!(
                            "P01_CHECK fire-impact key={key} value={value} target={:?}",
                            target.entity
                        ));
                        if key == "oteryn:creature.troll" && value.get() != 1 {
                            return Err(
                                format!("Enflame Troll impact expected 1, got {value}").into()
                            );
                        }
                        fire_damage = true;
                    }
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
        if let [Some(_)] = kills.as_slice()
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
    client.session_mut().attack_target(None).await?;
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
    let mut client = character_prep::Seeded {
        profile: "p01-sorcerer",
        reason,
    }
    .connect(clients, tag, 1)
    .await?;
    if scenario == "P01.full" {
        logout_when_ready(&mut client).await?;
        drop(client);
        charm_fixture::seed(clients).await?;
        client = connect_world(clients, 0xa4).await?;
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
    let initial = query(&mut client).await?;
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
    prepare_rat_ground(&mut client, world).await?;
    let first_kill = hunt_exact_creature(
        &mut client,
        rat.key(),
        race,
        rat_stage.fresh_ref,
        Some(rat_stage),
    )
    .await?;
    let current = query(&mut client).await?;
    if first_kill.race != race
        || first_kill.raw_xp != 5
        || first_kill.fire_damage
        || count(&current, race) != 250
        || current.charms.charm_points_available != 405
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
    let refused = command(&mut client, CharmProgressionAction::Assign { charm, race }).await?;
    if refused.outcome != CharmProgressionOutcome::Assign(CharmAssignDisposition::CharmLocked)
        || refused.charms != current.charms
        || refused.bestiary != current.bestiary
    {
        return Err(format!("locked ASSIGN reply/delta mismatch: {refused:?}").into());
    }
    evidence("P01_CHECK locked-assignment-refusal PASS disposition=CharmLocked");

    row.at("unlock-stage");
    let unlocked = command(
        &mut client,
        CharmProgressionAction::Unlock {
            charm,
            expected_stage: stage,
        },
    )
    .await?;
    let unlocked_state = unlocked
        .charms
        .charms
        .get(usize::try_from(charm.get() - 1)?)
        .ok_or("unlocked charm vanished")?;
    if unlocked.outcome != CharmProgressionOutcome::Unlock(CharmUnlockDisposition::Unlocked)
        || cost != 400
        || unlocked_state.unlocked_stage != stage + 1
        || unlocked.charms.charm_points_available + cost != current.charms.charm_points_available
    {
        return Err(format!("UNLOCK reply/delta mismatch: {unlocked:?}").into());
    }
    evidence("P01_CHECK unlock-stage PASS stage-and-balance-delta");

    row.at("assign-unlocked");
    if unlocked_state.assigned_race.is_some() {
        return Err(format!("UNLOCK unexpectedly assigned the charm: {unlocked_state:?}").into());
    }
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
    let assigned = command(&mut client, CharmProgressionAction::Assign { charm, race }).await?;
    let assigned_state = &assigned.charms.charms[usize::try_from(charm.get() - 1)?];
    if assigned.outcome != CharmProgressionOutcome::Assign(CharmAssignDisposition::Assigned)
        || assigned_state.assigned_race != Some(race)
        || !assigned_state.effect_active
    {
        return Err(format!("ASSIGN reply/delta mismatch: {assigned:?}").into());
    }
    evidence("P01_CHECK assign-unlocked PASS assignment-and-active-hook");

    row.at("assigned-combat-hook");
    let before_hook = count(&assigned, race);
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
    let mut observed_proc = false;
    for _ in 0..12 {
        let kill = hunt_exact_creature(&mut client, troll_key, race, troll_corpse, None).await?;
        if kill.race != race || kill.raw_xp != 20 {
            return Err(format!(
                "step=assigned-combat expected-race={race} credited-race={}",
                kill.race
            )
            .into());
        }
        if kill.fire_damage {
            observed_proc = true;
            break;
        }
    }
    let after_hook = query(&mut client).await?;
    let after_state = &after_hook.charms.charms[usize::try_from(charm.get() - 1)?];
    if count(&after_hook, race) != before_hook
        || after_state.assigned_race != Some(race)
        || !after_state.effect_active
    {
        return Err(format!("assigned combat hook lost durable state: {after_hook:?}").into());
    }
    evidence("P01_CHECK assigned-race-kill PASS analyser-credit-and-active-assignment");

    row.at("logout-fresh-admission");
    let old_session = client.join_snapshot().game_session_id;
    logout_when_ready(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xa3).await?;
    if fresh.join_snapshot().game_session_id == old_session {
        return Err("fresh admission reused the old game session".into());
    }
    let durable = query(&mut fresh).await?;
    if durable.bestiary != after_hook.bestiary || durable.charms != after_hook.charms {
        return Err(format!("fresh QUERY lost durable state: {durable:?}").into());
    }
    evidence("P01_CHECK logout-fresh-admission PASS exact-durable-views");
    if !observed_proc {
        return Err(
            "step=charm-proc no Enflame fire damage observed during bounded unarmed hunts".into(),
        );
    }
    logout_when_ready(&mut fresh).await
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
