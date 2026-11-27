//! P05 full-row journey, derived from the matrix, the re-audit row, and the
//! Task Board/talisman durable owner.
//!
//! Player scenario:
//! 1. Send the full Task Board query and require an accepted, decodable snapshot.
//! 2. Send Buy Talisman; require success, a 5,000-gold payment, and item 51978
//!    appearing in the ordinary inventory delta.
//! 3. Send Buy Talisman again; require the durable refusal and no second item.
//! 4. Open the bounty board, select an offered creature, kill it through native
//!    combat until complete, and claim; require kill progress and bounty points.
//! 5. Send Upgrade Talisman for the damage path; require the exact point debit,
//!    level/bonus increase, then send an unaffordable repeat and require refusal
//!    without a level or balance change.
//! 6. Equip item 51978 in its player extra slot and fight the selected bounty
//!    race; require an observed talisman combat proc and increased loot result.
//! 7. Log out normally, obtain a fresh admission, query again, and require the
//!    purchased item and upgraded level/balance to remain durable.
//!
//! The journey does not manufacture prerequisites. If an ordinary provisioned
//! character cannot reach a step, its single FAIL names that step and includes
//! the accepted/rejected status, refusal code, or missing delta actually seen.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::task_board::{self as wire, full};
use std::collections::BTreeSet;

pub(super) const P04_BOUNTY_KILL_BUDGET: Duration = Duration::from_secs(60);

const TALISMAN_ITEM: u32 = 51_978;

#[derive(Debug)]
struct StepFailure {
    step: u8,
    observed: String,
    source: Option<Box<dyn std::error::Error>>,
}

impl std::fmt::Display for StepFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "step-{}:{}", self.step, sanitize(&self.observed))
    }
}

impl std::error::Error for StepFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_deref()
    }
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.:,=".contains(character) {
                character
            } else {
                '-'
            }
        })
        .collect()
}

async fn command(
    client: &mut DevClientSession,
    action: u8,
    selection: u16,
) -> Result<(CommandStatus, Option<full::Reply>), StepFailure> {
    let request = full::Request {
        action,
        selection,
        race: String::new(),
    };
    let payload = full::encode_request(&request).map_err(|error| StepFailure {
        step: 1,
        observed: format!("action-{action}-request-encode-{error:?}"),
        source: None,
    })?;
    let response = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_TASK_BOARD_V1,
            wire::COMMAND_TYPE_TASK_BOARD,
            &payload,
        )
        .await
        .map_err(|error| {
            verdict::record_error(verdict::IndexedError::as_error(&error));
            StepFailure {
                step: 1,
                observed: format!("action-{action}-transport-{error}"),
                source: Some(error.into()),
            }
        })?;
    let reply = if response.payload.is_empty() {
        None
    } else {
        Some(
            full::decode_reply(&response.payload).map_err(|error| StepFailure {
                step: 1,
                observed: format!(
                    "action-{action}-status-{:?}-decode-{error:?}",
                    response.status
                ),
                source: None,
            })?,
        )
    };
    Ok((response.status, reply))
}

fn accepted(
    step: u8,
    result: (CommandStatus, Option<full::Reply>),
) -> Result<full::Reply, StepFailure> {
    let (status, reply) = result;
    let reply = reply.ok_or_else(|| StepFailure {
        step,
        observed: format!("status-{status:?}-missing-reply"),
        source: None,
    })?;
    if status != CommandStatus::Accepted || reply.refusal != 0 {
        return Err(StepFailure {
            step,
            observed: format!("status-{status:?}-refusal-{}", reply.refusal),
            source: None,
        });
    }
    Ok(reply)
}

fn item_count(client: &DevClientSession, reference: std::num::NonZeroU32) -> u32 {
    client
        .inventory()
        .map(|inventory| {
            inventory
                .entries
                .iter()
                .filter(|entry| entry.item_definition_ref == reference)
                .map(|entry| entry.count.get())
                .sum()
        })
        .unwrap_or(0)
}

pub(super) async fn wait_for_inventory(
    client: &mut DevClientSession,
    step: &str,
    committed: impl Fn(&DevClientSession) -> bool,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    while !committed(client) {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step={step} accepted-command-inventory-delta-missing").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    Ok(())
}

pub(super) async fn complete_bounty(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    opened: &full::Reply,
) -> TestResult<full::Reply> {
    complete_bounty_with_placement(client, clients, opened, None).await
}

pub(super) async fn complete_p04_bounty(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    opened: &full::Reply,
    publish_count: tokio::sync::oneshot::Sender<u32>,
) -> TestResult<full::Reply> {
    complete_bounty_with_placement(client, clients, opened, Some(publish_count)).await
}

async fn complete_bounty_with_placement(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    opened: &full::Reply,
    publish_count: Option<tokio::sync::oneshot::Sender<u32>>,
) -> TestResult<full::Reply> {
    let fixture = publish_count.is_some();
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let from = tile(client.world_spatial().actor_position)?;
    let fixture_species = if fixture {
        let input = native_input()?;
        let profiles: oteryn_gameplay_content::content::native_gameplay::CreatureProfilesDocument =
            serde_json::from_slice(&input.creature_profiles.bytes)?;
        profiles.records.into_iter().map(|record| record.profile.target.key).collect()
    } else {
        std::collections::BTreeSet::new()
    };
    let can_place = fixture && p04_fixture_route(world, from, &p04_visible_blockers(client)).is_some();
    let (index, offer) = bounty_offer(
        &opened.bounty.offers,
        fixture,
        |race| has_reachable_spawn(world, from, &metadata.spawns, race),
        |race| can_place && fixture_species.contains(race),
    )
    .ok_or_else(|| {
        let eligibility = if fixture { "fixture-eligible offer" } else { "reachable authored spawn" };
        format!("step=bounty-route no {eligibility} for {:?}", opened.bounty.offers)
    })?;

    if let Some(publish_count) = publish_count {
        publish_count.send(offer.required).map_err(|_| "P04 journey deadline closed")?;
    }
    let mut state = accepted(4, command(client, 8, u16::try_from(index)?).await?)?;
    for previous in 0..offer.required {
        if state.bounty.active.as_ref().is_none_or(|active| {
            active.race != offer.race
                || active.required != offer.required
                || active.points != offer.points
        }) || state.bounty.current != previous
        {
            return Err(format!(
                "step=bounty-selection-or-credit expected={} kills={previous}; observed={:?}",
                offer.race, state.bounty
            )
            .into());
        }
        if fixture {
            let kill = previous + 1;
            let required = offer.required;
            let deadline = tokio::time::Instant::now() + P04_BOUNTY_KILL_BUDGET;
            verdict::Harness::global().bounded_wait(deadline, "p04-bounty-kill");
            evidence(&format!("P04_BOUNTY kill={kill}/{required} state=start budget_seconds={} race={}", P04_BOUNTY_KILL_BUDGET.as_secs(), offer.race));
            state = tokio::time::timeout_at(deadline, async {
                hunt_p04_fixture(client, world, &offer.race).await?;
                TestResult::Ok(accepted(4, command(client, 4, 0).await?)?)
            })
            .await
            .map_err(|elapsed| {
                verdict::record_error(&elapsed);
                format!("P04 bounty kill {kill}/{required} hit its progress deadline")
            })??;
            if state.bounty.current != kill {
                return Err(format!("P04 bounty kill {kill}/{required} credit observed={}", state.bounty.current).into());
            }
            verdict::Harness::global().progress("p04-bounty-credit");
            evidence(&format!("P04_BOUNTY kill={kill}/{required} state=credited race={}", offer.race));
        } else {
            hunt_for_race(client, world, &metadata.spawns, &offer.race).await?;
            state = accepted(4, command(client, 4, 0).await?)?;
        }
    }
    if state.bounty.current != offer.required || state.bounty.state != 3 {
        return Err(format!(
            "step=bounty-completion expected={} kills={}; observed={:?}",
            offer.race, offer.required, state.bounty
        )
        .into());
    }
    let claimed = accepted(4, command(client, 9, 0).await?)?;
    if state.bounty_points.checked_add(u64::from(offer.points)) != Some(claimed.bounty_points) {
        return Err(format!(
            "step=bounty-claim balance={}->{} expected-credit={}",
            state.bounty_points, claimed.bounty_points, offer.points
        )
        .into());
    }
    Ok(claimed)
}

fn spawn_points(spawns: &oteryn_world_bundle::spawn::Table, race: &str) -> Vec<(TilePos, u32)> {
    spawns
        .sources
        .iter()
        .flat_map(|source| {
            source.points.iter().filter_map(move |point| {
                spawns
                    .creatures
                    .get(point.creature as usize)
                    .is_some_and(|entry| entry.key == race)
                    .then_some((
                        TilePos {
                            x: point.x,
                            y: point.y,
                            floor: source.floor,
                        },
                        point.respawn_ms,
                    ))
            })
        })
        .collect()
}

fn next_to(at: TilePos, point: TilePos) -> bool {
    at.floor == point.floor && at.x.abs_diff(point.x).max(at.y.abs_diff(point.y)) <= 1
}

fn outside_spawn_view(at: TilePos, point: TilePos) -> bool {
    at.floor == point.floor
        && (!(-8..=9).contains(&(i32::from(point.x) - i32::from(at.x)))
            || !(-6..=7).contains(&(i32::from(point.y) - i32::from(at.y))))
}

pub(super) async fn wait_for_spawn(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    race: &str,
) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
    use oteryn_dev_client::EntityKind;
    let points = spawn_points(spawns, race);
    let from = tile(client.world_spatial().actor_position)?;
    let approach = |at: TilePos| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
            && points
                .iter()
                .any(|(point, _)| at != *point && next_to(at, *point))
    };
    let path = route(world, from, approach)
        .ok_or_else(|| format!("step=spawn-route race={race} no reachable authored spawn"))?;
    walk(client, world, &path).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(95);
    verdict::Harness::global().bounded_wait(deadline, "authored-spawn-respawn");
    let mut searched = Vec::new();
    let mut retreat_at = Some(tokio::time::Instant::now() + Duration::from_secs(1));
    loop {
        let at = tile(client.world_spatial().actor_position)?;
        if let Some(target) = client.world_entities().and_then(|entities| {
            entities
                .iter()
                .find(|entity| {
                    entity.kind == EntityKind::Creature
                        && tile(entity.position).is_ok_and(|position| {
                            next_to(at, position)
                                && points.iter().any(|(point, _)| *point == position)
                        })
                })
                .copied()
        }) {
            return Ok(target);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=spawn-respawn race={race} no creature on an adjacent authored spawn before deadline").into());
        }
        for (point, _) in &points {
            if next_to(at, *point) && !searched.contains(point) {
                searched.push(*point);
            }
        }
        let visible = client
            .world_entities()
            .into_iter()
            .flat_map(|entities| entities.iter())
            .filter(|entity| entity.kind == EntityKind::Creature)
            .filter_map(|entity| tile(entity.position).ok())
            .filter(|at| points.iter().any(|(point, _)| point == at))
            .find_map(|point| route(world, at, |tile| approach(tile) && next_to(tile, point)));
        let alternate = visible.or_else(|| {
            route(world, at, |tile| {
                approach(tile)
                    && points
                        .iter()
                        .any(|(point, _)| next_to(tile, *point) && !searched.contains(point))
            })
        });
        if let Some(alternate) = alternate.filter(|path| !path.is_empty()) {
            walk(client, world, &alternate).await?;
            continue;
        }
        if retreat_at.is_some_and(|at| tokio::time::Instant::now() >= at) {
            retreat_at = None;
            if let Some(&(point, delay)) = points
                .iter()
                .find(|(point, delay)| *delay > 0 && next_to(at, *point))
                && let Some(retreat) = route(world, at, |tile| outside_spawn_view(tile, point))
            {
                // Ordinary blockable points defer while a player has the cell in view.
                walk(client, world, &retreat).await?;
                let until = (tokio::time::Instant::now() + Duration::from_millis(u64::from(delay)))
                    .min(deadline);
                while tokio::time::Instant::now() < until {
                    client.service_liveness(Duration::from_millis(100)).await?;
                    client.take_events();
                }
                let from = tile(client.world_spatial().actor_position)?;
                let path = route(world, from, |at| approach(at) && next_to(at, point)).ok_or_else(
                    || format!("step=spawn-respawn race={race} return route missing"),
                )?;
                walk(client, world, &path).await?;
            }
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
}

pub(super) fn canonical_races()
-> TestResult<&'static [(crate::domain::bestiary::BestiaryRace, u32)]> {
    static RACES: std::sync::LazyLock<Option<Vec<(crate::domain::bestiary::BestiaryRace, u32)>>> =
        std::sync::LazyLock::new(|| {
            let mut races: Vec<_> =
                crate::content::canonical_bestiary_rows(crate::content::ProjectEvidenceLimits {
                    max_documents: 5,
                    max_document_bytes: 2_500_000,
                    max_total_bytes: 8_000_000,
                    max_json_depth: 10,
                    max_decoded_fields: 70_000,
                    max_string_bytes: 10_000_000,
                    max_locator_bytes: 128,
                    max_locator_segments: 5,
                    max_reference_records: 4096,
                    max_import_records: 1,
                    max_reimport_states: 1,
                })
                .ok()?;
            races.sort_unstable_by(|a, b| a.0.key().cmp(b.0.key()));
            Some(races)
        });
    RACES
        .as_deref()
        .ok_or_else(|| "canonical bestiary unavailable".into())
}

fn race_reference(race: &str) -> TestResult<std::num::NonZeroU32> {
    let index = canonical_races()?
        .binary_search_by(|(entry, _)| entry.key().cmp(race))
        .map_err(|_| {
            format!("step=hunt-credit race={race} has no canonical analyser race reference")
        })?;
    std::num::NonZeroU32::new(
        u32::try_from(index)?
            .checked_add(1)
            .ok_or("race reference overflow")?,
    )
    .ok_or_else(|| "zero race reference".into())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct KillSample {
    pub race: std::num::NonZeroU32,
    pub raw_xp: u64,
    pub gained_xp: u64,
    pub fire_damage: bool,
}

fn corpse_handle(detail: oteryn_dev_client::EntityDetail) -> Option<oteryn_dev_client::ItemHandle> {
    use oteryn_dev_client::EntityDetail;
    match detail {
        EntityDetail::Object { item_handle, .. }
        | EntityDetail::ObjectWithAppearance { item_handle, .. } => item_handle,
        _ => None,
    }
}

pub(super) async fn hunt_for_race(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    key: &str,
) -> TestResult<KillSample> {
    let target = wait_for_spawn(client, world, spawns, key).await?;
    hunt_target(client, key, target).await
}

fn p04_visible_blockers(client: &DevClientSession) -> BTreeSet<TilePos> {
    use oteryn_dev_client::EntityKind;
    client.world_entities().into_iter().flat_map(|entities| {
        entities.iter().filter(move |entity| {
            &entity.entity.identity != entities.own_identity()
                && matches!(entity.kind, EntityKind::Creature | EntityKind::Npc | EntityKind::Player)
        }).filter_map(|entity| tile(entity.position).ok())
    }).collect()
}

fn p04_fixture_route(
    world: &BundleWorld,
    from: TilePos,
    blocked: &BTreeSet<TilePos>,
) -> Option<Vec<StepDirection>> {
    let outside_protection = |at: TilePos| {
        world.base().tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone() && !blocked.contains(&at))
    };
    route_avoiding(world, from, |at| {
        outside_protection(at)
            && [StepDirection::North, StepDirection::East, StepDirection::South, StepDirection::West]
                .into_iter()
                .any(|direction| destination(world, at, direction).is_some_and(outside_protection))
    }, |at| !blocked.contains(&at))
}

pub(super) async fn hunt_p04_fixture(
    client: &mut DevClientSession,
    world: &BundleWorld,
    key: &str,
) -> TestResult<KillSample> {
    let mut detours = BTreeSet::new();
    let mut placed = false;
    for attempt in 0..32 {
        let before = tile(client.world_spatial().actor_position)?;
        let mut blocked = p04_visible_blockers(client);
        blocked.extend(detours.iter().copied());
        let path = p04_fixture_route(world, before, &blocked)
            .ok_or("P04 hunt fixture needs a nearby open combat tile")?;
        let Some(&direction) = path.first() else {
            placed = true;
            break;
        };
        let expected = destination(world, before, direction).ok_or("P04 hunt fixture route lost its landing")?;
        evidence(&format!("FIXTURE row=P04.full fixture_walk_position={before:?} target={expected:?} race={key} step={attempt} route_steps={} blockers={}", path.len(), blocked.len()));
        let step = client.step(direction).await?;
        let actual = tile(client.world_spatial().actor_position)?;
        if step.disposition == StepDisposition::Blocked && actual == before
            && p04_visible_blockers(client).contains(&expected) && detours.insert(expected)
        {
            evidence(&format!("FIXTURE row=P04.full fixture_detour from={before:?} occupied={expected:?} race={key}"));
            continue;
        }
        if step.disposition != StepDisposition::Moved || step.world_spatial_delta.is_none() || actual != expected {
            return Err(format!("P04 hunt fixture walk refused at {before:?} target={expected:?} actual={actual:?}: {:?}", step.disposition).into());
        }
    }
    if !placed {
        return Err("P04 hunt fixture route exhausted its 32-step budget".into());
    }
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let position = client.world_spatial().actor_position;
    evidence(&format!("FIXTURE row=P04.full fixture_hunt_position={position:?} target_race={key}"));
    let target = parse_spawned_entity(&request_spawn(session, key.to_owned()).await?)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let target = loop {
        if let Some(entity) = client.world_entities().and_then(|view| view.get(&target)).copied() {
            if entity.kind != oteryn_dev_client::EntityKind::Creature {
                return Err("P04 hunt fixture target is not a creature".into());
            }
            break entity;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("P04 hunt fixture target {target:?} not visible from {position:?}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    };
    evidence(&format!("FIXTURE row=P04.full fixture_hunt_position={position:?} target={:?} target_position={:?}", target.entity, target.position));
    hunt_target(client, key, target).await
}

async fn hunt_target(
    client: &mut DevClientSession,
    key: &str,
    target: oteryn_dev_client::WorldSpatialEntity,
) -> TestResult<KillSample> {
    use oteryn_dev_client::{EntityDetail, EntityKind, SessionEvent};
    use oteryn_protocol_oteryn::analyser::AnalyserFact;
    let race = race_reference(key)?;
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
        return Err(format!("step=hunt-attack race={key} reply={attack:?}").into());
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
                            ..
                        },
                    ) => fire_damage = true,
                    AnalyserFact::Experience { raw, gained } => experience.push((raw, gained)),
                    _ => {}
                }
            }
        }
        if !kills.is_empty() && kills != [Some(race)] || experience.len() > 1 {
            client.session_mut().attack_target(None).await?;
            return Err(format!("step=hunt-credit expected={key}/{race} observed={kills:?} experience={experience:?}").into());
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
                    })
                    .copied()
            })
        {
            break (corpse, *raw, *gained);
        }
        if tokio::time::Instant::now() >= deadline {
            client.session_mut().attack_target(None).await?;
            return Err(format!("step=hunt-credit race={key} missing target corpse, kill credit or XP; kills={kills:?} experience={experience:?}").into());
        }
    };
    client.session_mut().attack_target(None).await?;
    let handle =
        corpse_handle(corpse.detail).ok_or("hunt corpse has no server-issued item handle")?;
    let used = client.use_item(handle).await?;
    if used.status != CommandStatus::Accepted
        || used.disposition != oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    {
        let (item, appearance) = match corpse.detail {
            EntityDetail::ObjectWithAppearance { item_definition_ref, appearance_id, .. } => {
                (Some(item_definition_ref), Some(appearance_id.get()))
            }
            EntityDetail::Object { item_definition_ref, .. } => (Some(item_definition_ref), None),
            _ => (None, None),
        };
        return Err(format!(
            "hunt corpse USE refused command_id={} server_sequence={} status={:?} disposition={:?} item_definition_ref={item:?} appearance_id={appearance:?} position={:?}",
            used.command_id, used.result_server_sequence, used.status, used.disposition, corpse.position,
        ).into());
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
    Ok(KillSample {
        race,
        raw_xp,
        gained_xp,
        fire_damage,
    })
}

pub(super) fn has_reachable_spawn(
    world: &BundleWorld,
    from: TilePos,
    spawns: &oteryn_world_bundle::spawn::Table,
    race: &str,
) -> bool {
    let points = spawn_points(spawns, race);
    !points.is_empty()
        && route(world, from, |at| {
            world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| !tile.protection_zone())
                && points
                    .iter()
                    .any(|(point, _)| at != *point && next_to(at, *point))
        })
        .is_some()
}

fn bounty_offer(
    offers: &[full::BountyOffer],
    fixture: bool,
    mut naturally_reachable: impl FnMut(&str) -> bool,
    mut fixture_eligible: impl FnMut(&str) -> bool,
) -> Option<(usize, &full::BountyOffer)> {
    offers
        .iter()
        .enumerate()
        .filter(|(_, offer)| {
            !offer.elite && offer.required > 0 && if fixture {
                fixture_eligible(&offer.race)
            } else {
                naturally_reachable(&offer.race)
            }
        })
        .min_by_key(|(_, offer)| offer.required)
}

async fn journey(clients: &SeamClients<'_>) -> Result<(), StepFailure> {
    let mut client = connect_character(clients, 0xd5, 0).await.map_err(|error| {
        verdict::record_error(verdict::IndexedError::as_error(&error));
        StepFailure {
            step: 1,
            observed: format!("admission-{error}"),
            source: Some(error),
        }
    })?;

    let before = accepted(1, command(&mut client, 4, 0).await?)?;
    evidence(&format!(
        "P05_CHECK step=1 status=PASS bounty_points={} talisman_level={}",
        before.bounty_points, before.talisman[0].level
    ));

    let reference = provisioning::item_index()
        .and_then(|definitions| provisioning::item_reference(&definitions, TALISMAN_ITEM))
        .map_err(|error| StepFailure {
            step: 2,
            observed: format!("talisman-definition-{error}"),
            source: Some(error),
        })?;
    let items_before = item_count(&client, reference);
    let bought = accepted(2, command(&mut client, 17, 0).await?).map_err(|mut failure| {
        failure.observed.push_str(";need=5000-spendable-gold;source=crates/gameplay/quests/src/quest/task_board_full.rs:604");
        failure
    })?;
    let items_after = items_before.checked_add(1).ok_or_else(|| StepFailure {
        step: 2,
        observed: "talisman-count-overflow".into(),
        source: None,
    })?;
    wait_for_inventory(&mut client, "talisman-purchase", |client| {
        item_count(client, reference) == items_after
    })
    .await
    .map_err(|error| StepFailure {
        step: 2,
        observed: error.to_string(),
        source: Some(error),
    })?;
    evidence("P05_CHECK step=2 status=PASS purchase-and-inventory-delta");

    // A new command ID can legitimately buy a second talisman. Keep the replay
    // prerequisite explicit instead of spending another 5,000 gold.
    evidence("P05_CHECK step=purchase-replay status=UNPROVEN need=exact-original-command-replay");

    let mut claimed = bought.clone();
    for _ in 0..2 {
        if claimed.bounty_points >= u64::from(claimed.talisman[0].next_cost) {
            break;
        }
        let opened = accepted(4, command(&mut client, 7, 0).await?)?;
        claimed = complete_bounty(&mut client, clients, &opened)
            .await
            .map_err(|error| StepFailure {
                step: 4,
                observed: error.to_string(),
                source: Some(error),
            })?;
    }
    if claimed.bounty_points < u64::from(claimed.talisman[0].next_cost) {
        return Err(StepFailure {
            step: 5,
            observed: format!(
                "upgrade-funding bounty-points={} next-cost={};need=more-completed-bounties",
                claimed.bounty_points, claimed.talisman[0].next_cost
            ),
            source: None,
        });
    }
    let prior = claimed.talisman[0];
    let upgraded = accepted(5, command(&mut client, 18, 0).await?)?;
    if prior.level.checked_add(1) != Some(upgraded.talisman[0].level)
        || upgraded.talisman[0].bonus <= prior.bonus
        || claimed
            .bounty_points
            .checked_sub(u64::from(prior.next_cost))
            != Some(upgraded.bounty_points)
    {
        return Err(StepFailure {
            step: 5,
            observed: format!("upgrade-delta {prior:?} -> {:?}", upgraded.talisman[0]),
            source: None,
        });
    }
    let next_cost = u64::from(upgraded.talisman[0].next_cost);
    if next_cost > upgraded.bounty_points {
        let refused = command(&mut client, 18, 0).await?;
        if !matches!(refused, (CommandStatus::Accepted, Some(ref reply)) if reply.refusal == 4
            && reply.talisman == upgraded.talisman && reply.bounty_points == upgraded.bounty_points)
        {
            return Err(StepFailure {
                step: 5,
                observed: format!("unaffordable-upgrade refusal/state mismatch {refused:?}"),
                source: None,
            });
        }
    } else {
        evidence(
            "P05_CHECK step=unaffordable-upgrade status=UNPROVEN need=balance-below-next-cost",
        );
    }
    let saved = accepted(7, command(&mut client, 4, 0).await?)?;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, 0xd6, 0)
        .await
        .map_err(|error| StepFailure {
            step: 7,
            observed: format!("fresh-admission-{error}"),
            source: Some(error),
        })?;
    let persisted = accepted(7, command(&mut fresh, 4, 0).await?)?;
    if persisted != saved || item_count(&fresh, reference) != items_after {
        return Err(StepFailure {
            step: 7,
            observed: "fresh-admission lost board or talisman custody".into(),
            source: None,
        });
    }
    logout(&mut fresh).await?;
    Err(StepFailure {
        step: 6,
        observed: "need=exact-original-purchase-command-replay-and-wire-equipment-slot-10-for-talisman-combat-and-loot;source=crates/protocol-oteryn/src/item_view.rs:209".into(),
        source: None,
    })
}

pub(super) async fn logout_after_combat(client: &mut DevClientSession) -> TestResult {
    use oteryn_protocol_oteryn::logout::LogoutResult;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    verdict::Harness::global().bounded_wait(deadline, "logout-combat-lock");
    loop {
        let result = client.session_mut().logout().await?.result;
        let retry = match result {
            Some(LogoutResult::Accepted) => return Ok(()),
            Some(LogoutResult::Busy) => Duration::from_millis(250),
            Some(LogoutResult::InFight { retry_after_ms }) => {
                Duration::from_millis(u64::from(retry_after_ms).max(100))
            }
            _ => return Err(format!("step=logout observed={result:?}").into()),
        };
        let until = (tokio::time::Instant::now() + retry).min(deadline);
        while tokio::time::Instant::now() < until {
            client.service_liveness(Duration::from_millis(100)).await?;
            client.take_events();
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=logout retry deadline observed={result:?}").into());
        }
    }
}

async fn logout(client: &mut DevClientSession) -> Result<(), StepFailure> {
    logout_after_combat(client)
        .await
        .map_err(|error| StepFailure {
            step: 7,
            observed: error.to_string(),
            source: Some(error),
        })
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients)))
            .await;
    match &result {
        Ok(Ok(())) => evidence("P05_GAP status=PASS"),
        Ok(Err(error)) => evidence(&format!("P05_GAP status=FAIL observed={error}")),
        Err(_) => evidence("P05_GAP status=FAIL step=deadline observed=no-reply"),
    }
    match result {
        Ok(Ok(())) => evidence(&format!(
            "e2e_all scenario=P05.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P05.full status=FAIL seconds={:.3} evidence={error}",
            started.elapsed().as_secs_f64()
        )),
        Err(_) => evidence(&format!(
            "e2e_all scenario=P05.full status=FAIL seconds={:.3} evidence=step-timeout-observed-no-reply",
            started.elapsed().as_secs_f64()
        )),
    }
}

impl verdict::IndexedError for StepFailure {
    fn as_error(&self) -> &(dyn std::error::Error + 'static) {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{bounty_offer, full, sanitize};

    #[test]
    fn selects_shortest_eligible_ordinary_offer_at_its_wire_index_in_each_mode() {
        let offer = |race: &str, required, elite| full::BountyOffer {
            elite,
            dust: 0,
            race: race.into(),
            required,
            experience: 1,
            points: 1,
            grade: 0,
        };
        let offers = [
            offer("unreachable", 1, false),
            offer("reachable", 1, true),
            offer("reachable", 6, false),
            offer("reachable", 4, false),
        ];
        assert_eq!(
            bounty_offer(&offers, false, |race| race == "reachable", |_| true),
            Some((3, &offers[3]))
        );
        assert_eq!(bounty_offer(&offers, false, |_| false, |_| true), None);
        assert_eq!(
            bounty_offer(&offers, true, |_| false, |race| race == "unreachable"),
            Some((0, &offers[0]))
        );
        assert_eq!(bounty_offer(&offers, true, |_| true, |_| false), None);
    }

    #[test]
    fn evidence_is_one_token() {
        assert_eq!(
            sanitize("status Accepted / missing delta"),
            "status-Accepted---missing-delta"
        );
    }
}

#[cfg(test)]
#[test]
fn row_05_publishes_a_gap_for_an_unproven_journey() {
    assert!(
        include_str!("full_row_p05.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(production, _)| production.contains("P05_GAP status="))
    );
}

#[cfg(test)]
#[test]
fn review_p05_guards_the_journey() {
    assert!(
        include_str!("full_row_p05.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(body, _)| body.contains("wait_for_spawn"))
    );
}

#[cfg(test)]
#[test]
fn p05_distinguishes_a_purchase_replay_and_funds_the_upgrade_before_submitting() {
    let production = include_str!("full_row_p05.rs").split_once("#[cfg(test)]");
    assert!(production.is_some_and(|(body, _)| {
        body.contains("step=purchase-replay status=UNPROVEN")
            && body.contains("need=more-completed-bounties")
    }));
}

#[cfg(test)]
#[test]
fn review_2_p05() {
    assert!(
        include_str!("full_row_p05.rs")
            .split_once("#[cfg(test)]")
            .is_some_and(|(body, _)| body.contains("corpse_handle"))
    );
}

#[cfg(test)]
#[test]
fn loot_uses_handles_from_both_corpse_projections() -> TestResult {
    use oteryn_dev_client::{EntityDetail, ItemHandle};
    let handle = ItemHandle::new(7).ok_or("test handle")?;
    let appearance_id = std::num::NonZeroU16::new(8).ok_or("test appearance")?;
    assert_eq!(
        corpse_handle(EntityDetail::Object {
            item_definition_ref: 9,
            quantity: 1,
            item_handle: Some(handle),
        }),
        Some(handle)
    );
    assert_eq!(
        corpse_handle(EntityDetail::ObjectWithAppearance {
            item_definition_ref: 9,
            quantity: 1,
            item_handle: Some(handle),
            appearance_id,
        }),
        Some(handle)
    );
    Ok(())
}
