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
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::charm::{CharmAssignDisposition, CharmUnlockDisposition};
use oteryn_protocol_oteryn::charm_progression::{
    self as wire, CharmProgressionAction, CharmProgressionOutcome, CharmProgressionReply,
};
use oteryn_protocol_oteryn::presentation::{
    PresentationEventKind, PresentationValue, PresentationValueKind,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::collections::BTreeSet;
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

#[derive(Debug, Clone, Copy)]
struct IncomingDamage {
    sequence: u64,
    attacker: Option<EntityRef>,
    target: EntityRef,
    value: PresentationValue,
}

fn incoming_damage(event: &SessionEvent) -> impl Iterator<Item = IncomingDamage> + '_ {
    let frame = match event {
        SessionEvent::Presentation(frame) => Some(frame),
        _ => None,
    };
    frame.into_iter().flat_map(|frame| {
        frame.value.iter().filter_map(|event| {
            let PresentationEventKind::ValueText {
                kind: PresentationValueKind::DamageReceived,
                attacker,
                target,
                primary,
                ..
            } = event.kind else {
                return None;
            };
            Some(IncomingDamage { sequence: frame.server_sequence, attacker, target, value: primary })
        })
    })
}

impl IncomingDamage {
    fn log(self) {
        evidence(&format!(
            "P01_LOGOUT incoming_damage sequence={} attacker={:?} target={:?} value={:?}",
            self.sequence, self.attacker, self.target, self.value
        ));
    }
}

#[derive(Default)]
pub(super) struct CombatContacts(BTreeSet<EntityRef>);

impl CombatContacts {
    fn observe(&mut self, event: &SessionEvent) {
        for damage in incoming_damage(event) {
            damage.log();
            self.0.extend(damage.attacker);
        }
    }

    fn nearby(&self, client: &DevClientSession, floor: i8) -> BTreeMap<EntityRef, TilePos> {
        client_row_p01::route_occupants(client)
            .filter(|(at, entity)| at.floor == floor && self.0.contains(&entity.entity))
            .map(|(at, entity)| (entity.entity, at))
            .collect()
    }
}

pub(super) async fn logout_when_ready(client: &mut DevClientSession) -> TestResult {
    // Global monster combat lasts 60 seconds. Allow one domain 10 publication tick.
    let started = tokio::time::Instant::now();
    let deadline = started + Duration::from_secs(60) + client_row_p01::VISIBILITY_TICK;
    let initial = client.session_mut().combat_state().copied();
    evidence(&format!("P01_LOGOUT wait_ms=60250 initial={initial:?}"));
    verdict::Harness::global().bounded_wait(deadline, "logout-combat-lock");
    let mut last_frame = None;
    let mut last_damage = None;
    loop {
        for event in client.take_events() {
            for damage in incoming_damage(&event) {
                damage.log();
                last_damage = Some(damage);
            }
            if let SessionEvent::ActorCombatState(frame) = event {
                evidence(&format!(
                    "P01_LOGOUT combat sequence={} state={:?}",
                    frame.server_sequence, frame.value
                ));
                last_frame = Some((frame.server_sequence, frame.value));
            }
        }
        if client
            .session_mut()
            .combat_state()
            .is_some_and(|state| !state.in_fight && state.target.is_none())
        {
            evidence(&format!(
                "P01_LOGOUT clear elapsed_ms={} last_frame={last_frame:?}",
                started.elapsed().as_millis()
            ));
            break;
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(format!(
                "combat state did not clear before logout: initial={initial:?} last_frame={last_frame:?} last_damage={last_damage:?}"
            ).into());
        }
        client
            .service_liveness(remaining.min(client_row_p01::VISIBILITY_TICK))
            .await?;
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

async fn retreat_from_spawn(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawn: TilePos,
    contacts: &mut CombatContacts,
) -> TestResult {
    let clear_of = |at: TilePos, origin: TilePos| {
        at.floor != origin.floor || at.x.abs_diff(origin.x).max(at.y.abs_diff(origin.y)) > 11
    };
    for _ in 0..3 {
        client.take_events().iter().for_each(|event| contacts.observe(event));
        let from = tile(client.world_spatial().actor_position)?;
        let nearby = contacts.nearby(client, from.floor);
        let occupied = client_row_p01::occupied_rat_route_tiles(client);
        let retreat = route_avoiding(
            world,
            from,
            |at| clear_of(at, spawn) && nearby.values().all(|&point| clear_of(at, point)),
            |at| !occupied.contains(&at),
        )
        .ok_or_else(|| format!("no walkable retreat from hunt at {from:?} aggressors={nearby:?}"))?;
        client_row_p01::walk_rat_route(client, world, &retreat, &BTreeSet::new(), |event| {
            contacts.observe(event);
        }).await?;
        client.service_liveness(client_row_p01::VISIBILITY_TICK).await?;
        client.take_events().iter().for_each(|event| contacts.observe(event));
        let reached = tile(client.world_spatial().actor_position)?;
        let remaining = contacts.nearby(client, reached.floor);
        evidence(&format!(
            "P01_LOGOUT retreat from={from:?} reached={reached:?} aggressors={:?} remaining={remaining:?}",
            contacts.0
        ));
        if remaining.is_empty() {
            return Ok(());
        }
    }
    Err("P01 retreat still has visible aggressors after three routes".into())
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
    let mut reserved = BTreeSet::new();
    let mut points = std::collections::BTreeSet::new();
    for source in &spawns.sources {
        for point in &source.points {
            let at = TilePos {
                x: point.x,
                y: point.y,
                floor: source.floor,
            };
            reserved.insert(at);
            if spawns
                .creatures
                .get(point.creature as usize)
                .is_some_and(|creature| creature.key == key)
            {
                points.insert(at);
            }
        }
    }
    let occupied = client_row_p01::occupied_rat_route_tiles(client);
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
        |at| !occupied.contains(&at) && !reserved.contains(&at),
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
    client_row_p01::walk_rat_route(client, world, &path, &reserved, |_| {}).await?;
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

pub(super) async fn prepare_unarmed(client: &mut DevClientSession) -> TestResult {
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
    if !client.inventory().is_some_and(|inventory| {
        inventory.equipment.iter().all(|entry| !hands.contains(&entry.slot))
    }) {
        return Err("P01 final inventory did not show both hands empty".into());
    }
    evidence("P01_CHECK unarmed-baseline PASS hands=empty");
    Ok(())
}

pub(super) enum HuntEvidence<'a, 'b> {
    Principal(&'a SeamClients<'b>),
    Contacts(&'a SeamClients<'b>, &'a mut CombatContacts),
}

fn reject_late_hunt_evidence(event: &SessionEvent) -> TestResult {
    use oteryn_protocol_oteryn::analyser::{AnalyserFact, AnalyserImpact};
    use oteryn_protocol_oteryn::damage_element::DamageElement;
    let unexpected = |fact: &AnalyserFact| matches!(
        fact,
        AnalyserFact::Kill { .. } | AnalyserFact::Experience { .. }
            | AnalyserFact::Impact(AnalyserImpact::Damage { element: DamageElement::Fire, .. })
    );
    let invalid = match event {
        SessionEvent::AnalyserFacts(frame) => frame.value.iter().any(unexpected),
        SessionEvent::AnalyserAttributedFacts(frame) => frame.value.iter().any(|entry| unexpected(&entry.fact)),
        SessionEvent::ChatLine(frame) => matches!(&frame.value,
            oteryn_protocol_oteryn::chat::ChatLine::System { text } if text.ends_with(" (enflame charm)")),
        _ => false,
    };
    if invalid {
        return Err(format!("unexpected analyser credit or fire after the selected death: {event:?}").into());
    }
    Ok(())
}

pub(super) async fn hunt_exact_creature(
    client: &mut DevClientSession,
    target: oteryn_dev_client::WorldSpatialEntity,
    key: &str,
    race: NonZeroU32,
    corpse_ref: u32,
    stage: Option<client_hunt::CorpseStage>,
    mut observations: HuntEvidence<'_, '_>,
) -> TestResult<full_row_p05::KillSample> {
    use oteryn_dev_client::{EntityKind, SessionEvent};
    use oteryn_protocol_oteryn::analyser::{AnalyserFact, AnalyserImpact};
    use oteryn_protocol_oteryn::damage_element::DamageElement;
    if key == "oteryn:creature.troll" && matches!(observations, HuntEvidence::Principal(_)) {
        let identity = target.entity.identity.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
        request_fixture_control(
            uuid_text(client.join_snapshot().game_session_id.as_bytes()),
            "--force-enflame",
            format!("{identity} {} 0:64", target.entity.generation),
        ).await?;
        evidence(&format!("P01_FIXTURE forced proc rolls for unassigned target={:?}", target.entity));
    }
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
    let mut hunt_lock = false;
    let mut target_health_fell = false;
    let mut enflame_log_count = 0;
    let mut experience = Vec::new();
    let (corpse, raw_xp, gained_xp) = loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        for event in client.take_events() {
            if let HuntEvidence::Contacts(_, contacts) = &mut observations {
                contacts.observe(&event);
            }
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
                        target_health_fell |= match (target.detail, updated.detail) {
                            (
                                oteryn_dev_client::EntityDetail::Actor { health_percent: initial, .. }
                                | oteryn_dev_client::EntityDetail::ActorWithIcons { health_percent: initial, .. },
                                oteryn_dev_client::EntityDetail::Actor { health_percent: current, .. }
                                | oteryn_dev_client::EntityDetail::ActorWithIcons { health_percent: current, .. },
                            ) => current < initial,
                            _ => false,
                        };
                    }
                    continue;
                }
                SessionEvent::ActorCombatState(frame) => {
                    hunt_lock |= frame.value.in_fight;
                    continue;
                }
                SessionEvent::AnalyserFacts(delta) => {
                    if matches!(observations, HuntEvidence::Principal(_)) {
                        return Err("P01.client received an un-attributed analyser batch".into());
                    }
                    delta.value
                }
                SessionEvent::AnalyserAttributedFacts(delta) => {
                    for entry in &delta.value {
                        let Some(source) = &entry.charm else {
                            if matches!(entry.fact, AnalyserFact::Impact(AnalyserImpact::Damage { element: DamageElement::Fire, .. })) {
                                return Err("unattributed fire damage during the P01 hunt".into());
                            }
                            continue;
                        };
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
                        verdict::Harness::global().progress("p01-attributed-enflame");
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
                    }) if !fire_damage => return Err("unexpected fire damage during the P01 hunt".into()),
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
            let visible = client.world_entities();
            let corpses: Vec<_> = visible
                .into_iter()
                .flat_map(|entities| entities.iter())
                .filter(|entry| entry.kind == EntityKind::Corpse)
                .filter(|entry| entry.position.floor == last_position.floor)
                .filter(|entry| !old_corpses.contains(&entry.entity))
                .take(8)
                .collect();
            evidence(&format!(
                "P01_HUNT missing_corpse target={:?} last_position={last_position:?} expected_ref={corpse_ref} target_present={} sequence={} new_corpses={corpses:?}",
                target.entity,
                visible.is_some_and(|entities| entities.get(&target.entity).is_some()),
                client.last_server_sequence()
            ));
            return Err(format!("step=hunt-credit race={race} missing target corpse, kill credit or XP; kills={kills:?} experience={experience:?}").into());
        }
    };
    if !hunt_lock {
        return Err("selected hunt never published a combat-lock frame".into());
    }
    evidence("P01_CHECK hunt-combat-lock PASS observed-in-fight");
    let fresh_seen = tokio::time::Instant::now();
    if key == "oteryn:creature.troll" {
        if (enflame_log_count == 1) != fire_damage {
            return Err("Enflame Server Log did not match an attributed impact".into());
        }
        if fire_damage {
            if !target_health_fell {
                return Err("attributed Enflame target never showed lower wire health".into());
            }
            evidence(
                "P01_CHECK enflame-server-log PASS A troll loses 1 hitpoint due to your attack. (enflame charm)",
            );
        }
    }
    client.session_mut().attack_target(None).await?;
    {
        let clients = match observations {
            HuntEvidence::Principal(clients) | HuntEvidence::Contacts(clients, _) => clients,
        };
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
            "P01_CHECK exact-hunt-death PASS key={key} target={:?} corpse={:?}",
            target.entity, corpse.entity
        ));
    }
    let corpse = if let Some(stage) = stage {
        let matured = client_hunt::wait_for_corpse_stage(client, corpse, stage).await?;
        let elapsed = fresh_seen.elapsed();
        // The authored fresh Rat stage lasts 10 s. Wire observation has bounded scheduling jitter.
        if !(Duration::from_secs(8)..=Duration::from_secs(15)).contains(&elapsed) {
            return Err(format!("Rat corpse stage outside the ten-second observation window: {elapsed:?}").into());
        }
        evidence(&format!("P01_CHECK rat-decay-timing PASS elapsed_ms={}", elapsed.as_millis()));
        matured
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
        for event in client.take_events() { reject_late_hunt_evidence(&event)?; }
        if tokio::time::Instant::now() >= deadline {
            return Err("hunt corpse container delta missing".into());
        }
    }
    let entries = client
        .open_container()
        .ok_or("corpse view")?
        .entries
        .clone();
    if entries.is_empty() {
        return Err("Selected corpse is empty; strict loot transfer cannot be proved".into());
    }
    let items = provisioning::item_index()?;
    let authored_loot: &[(u32, u32)] = if key == "oteryn:creature.rat" {
        &[(3607, 1), (3031, 4)]
    } else {
        &[(3054, 1), (9689, 1), (23986, 1), (3412, 1), (3336, 1), (3003, 1),
          (3552, 1), (3355, 1), (3277, 1), (3577, 1), (3268, 1), (3031, 12)]
    };
    let mut allowed = BTreeMap::new();
    for &(id, count) in authored_loot {
        allowed.insert(provisioning::item_reference(&items, id)?, count);
    }
    for entry in &entries {
        if allowed.remove(&entry.item_definition_ref).is_none_or(|max| entry.count.get() > max) {
            return Err(format!("unexpected corpse loot for {key}: {entry:?}").into());
        }
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
            for event in client.take_events() { reject_late_hunt_evidence(&event)?; }
        }
    }
    client.service_liveness(client_row_p01::VISIBILITY_TICK).await?;
    for event in client.take_events() {
        reject_late_hunt_evidence(&event)?;
    }
    Ok(full_row_p05::KillSample {
        race,
        raw_xp,
        gained_xp,
        fire_damage,
    })
}

async fn connect_hunter_with_system_chat(
    clients: &SeamClients<'_>,
    tag: u8,
) -> TestResult<DevClientSession> {
    const HUNTER: usize = 1;
    wait_character_released(clients, HUNTER).await?;
    let account = &clients.accounts[HUNTER];
    let token = if requires_platform_grant(account) {
        platform_row_grant(clients, HUNTER).await?
    } else {
        let security_generation = platform_generation(clients.descriptor, account).await?;
        let (nonce, attempt) = fresh_grant_material(tag)?;
        sign_grant(
            &Grant {
                signing: clients.signing,
                key_id: clients.key_id,
                account_id: account,
                character_id: clients.characters[HUNTER],
                world_id: clients.world,
                channel_id: clients.channel,
                security_generation,
                scope_generation: clients.scope_generation,
                nonce,
                attempt,
            },
            now_seconds()?,
        )
    };
    let mut capabilities = oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES.to_vec();
    capabilities.push(oteryn_protocol_oteryn::chat::CAPABILITY_CHAT_SYSTEM_MESSAGE_V1);
    capabilities.sort_unstable();
    capabilities.dedup();
    let client = oteryn_dev_client::connect_session_with_capabilities(
        oteryn_dev_client::JoinRequest {
            address: clients.address,
            server_name: "localhost",
            root_certificate: clients.certificate,
            schema_revision: 1,
            character_id: crate::foundation::CharacterId::decode(&clients.characters[HUNTER])?,
            admission_material: token.as_bytes(),
            client_build_id: "oteryn-dev-client/world-qualification",
            deadline: Duration::from_secs(20),
        },
        &capabilities,
    )
    .await?;
    verdict::Harness::global().progress("admission");
    Ok(client)
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
    let mut client = if scenario == "P01.full" {
        connect_hunter_with_system_chat(clients, 0xa4).await?
    } else {
        connect_world(clients, 0xa4).await?
    };
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

async fn refusal_wallet(client: &DevClientSession, clients: &SeamClients<'_>) -> TestResult<([u64; 3], u64)> {
    let carried = client_row_e02::backpack_coins(client, client_row_e02::coin_refs()?)?;
    let bank = full_row_e06::bank_balance(clients, 1).await?;
    if bank == 0 && carried == [0; 3] {
        return Err("P01.full requires positive gold to detect a refusal debit".into());
    }
    Ok((carried, bank))
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
        || !initial.bestiary.iter().any(|entry| {
            entry.race == race && entry.kill_count == 249
                && entry.kill_thresholds == [10, 100, 250] && !entry.is_complete()
        })
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
        authored_target(&mut client, world, &metadata.spawns, rat.key(), "Rat", Some(TilePos { x: 32491, y: 32169, floor: -7 })).await?;
    let first_kill = hunt_exact_creature(
        &mut client,
        target,
        rat.key(),
        race,
        rat_stage.fresh_ref,
        Some(rat_stage),
        HuntEvidence::Principal(clients),
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
        || !current.bestiary.iter().any(|entry| {
            entry.race == race && entry.kill_count == 250
                && entry.kill_thresholds == [10, 100, 250] && entry.is_complete()
        })
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

    let locked_wallet = refusal_wallet(&client, clients).await?;
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
    let readback = query(&mut client, "p01-full-locked-readback.json", &mut frames).await?;
    let mut expected_readback = current.clone();
    expected_readback.outcome = CharmProgressionOutcome::Query;
    if readback != expected_readback || refusal_wallet(&client, clients).await? != locked_wallet {
        return Err("locked refusal changed independent state or wallet".into());
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
    let incomplete_wallet = refusal_wallet(&client, clients).await?;
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
    let readback = query(&mut client, "p01-full-incomplete-readback.json", &mut frames).await?;
    let mut expected_readback = unlocked.clone();
    expected_readback.outcome = CharmProgressionOutcome::Query;
    if readback != expected_readback || refusal_wallet(&client, clients).await? != incomplete_wallet {
        return Err("incomplete refusal changed independent state or wallet".into());
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
    let assigned_wallet = refusal_wallet(&client, clients).await?;
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
    let assigned_readback = query(&mut client, "p01-full-assigned-readback.json", &mut frames).await?;
    let mut assigned_query = expected_assigned.clone();
    assigned_query.outcome = CharmProgressionOutcome::Query;
    if assigned_readback != assigned_query || refusal_wallet(&client, clients).await? != assigned_wallet {
        return Err("successful assignment changed wallet or disagreed with independent readback".into());
    }
    evidence("P01_CHECK assign-unlocked PASS assignment-and-active-hook zero-wallet-debit");

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
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::chat::CAPABILITY_CHAT_SYSTEM_MESSAGE_V1)
    {
        return Err("step=enflame-attribution missing negotiated System Log capability 617".into());
    }
    let (spawn, troll) = authored_target(
        &mut client,
        world,
        &metadata.spawns,
        troll_key,
        "Troll",
        Some(TilePos { x: 32469, y: 32235, floor: -8 }),
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
    let mut contacts = CombatContacts::default();
    let kill = hunt_exact_creature(
        &mut client,
        troll,
        troll_key,
        race,
        troll_corpse,
        None,
        HuntEvidence::Contacts(clients, &mut contacts),
    )
    .await?;
    if kill.race != race || kill.raw_xp != 20 {
        return Err(format!("incorrect Troll kill credit: {kill:?}").into());
    }
    retreat_from_spawn(&mut client, world, spawn, &mut contacts).await?;
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
