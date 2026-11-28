//! E2E-C17b — Invisibility and targeting, derived from the C17b matrix/audit row
//! and the ordinary-combat/monster-AI owners named there.
//!
//! Stage a live Rat, prove its attributed hit, then check `utana vid`, its cooldown,
//! and the Rat's loss of damage while the condition is active. Global Invisible keeps
//! the caster visible to players: https://tibia.fandom.com/wiki/Invisible.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::actor_spell::{SpellCastDisposition, SpellTarget};
use std::collections::BTreeSet;

const ACTIONS: &[&str] = &[
    "monster-targets-visible",
    "player-sees-visible",
    "cast-and-cost",
    "monster-drops-invisible",
    "player-sees-invisible",
    "cooldown-no-cost",
    "caster-damages-rat",
    "fresh-admission-condition",
];

struct Row {
    passed: std::collections::BTreeSet<&'static str>,
    started: std::time::Instant,
}
impl Row {
    fn new() -> Self {
        Self {
            passed: std::collections::BTreeSet::new(),
            started: std::time::Instant::now(),
        }
    }
    fn pass(&mut self, action: &'static str, sequence: u64) -> TestResult {
        if sequence == 0 || !ACTIONS.contains(&action) || !self.passed.insert(action) {
            return Err(format!(
                "C17b invalid/duplicate receipt for step {action}: sequence={sequence}"
            )
            .into());
        }
        evidence(&format!(
            "C17B_CHECK invisible-targeting PASS step={action} server_sequence={sequence}"
        ));
        Ok(())
    }
    fn finish(self, result: &TestResult) {
        if let Err(error) = result {
            evidence(&format!("C17B_GAP step-failure observed={error}"));
        }
        for action in ACTIONS {
            if !self.passed.contains(action) {
                evidence(&format!("C17B_GAP step={action} observed=not-reached"));
            }
        }
        let pass = result.is_ok() && ACTIONS.iter().all(|a| self.passed.contains(a));
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C17b.full status={} seconds={:.3}{}",
            if pass { "PASS" } else { "FAIL" },
            self.started.elapsed().as_secs_f64(),
            if pass {
                ""
            } else {
                " evidence=runtime-step-failure-see-C17B_GAP"
            }
        ));
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let mut row = Row::new();
    let result = match verdict::indexed_timeout(
        Duration::from_secs(1200),
        verdict::observe(journey(&mut row, clients)),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(
            "step=journey observed=timeout-after-1200s need=responsive-spell/AI/visibility-path"
                .into(),
        ),
    };
    row.finish(&result);
}

pub(super) fn spell_book() -> TestResult<crate::spell::SpellBook> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(
        &root.join("content/spells.manifest.json"),
    )?;
    let catalog =
        crate::spell::executable_catalog::compile(&input.catalog.bytes, &input.catalog.sha256)?;
    let selections = crate::spell::executable_catalog::compile_source_selection(
        &input.source_selection.bytes,
        &input.source_selection.sha256,
        &catalog,
    )?;
    Ok(catalog.into_spell_book_with_selections(&selections)?)
}
pub(super) fn spell_index(book: &crate::spell::SpellBook) -> TestResult<std::num::NonZeroU32> {
    (1..=book.source_len())
        .filter_map(|i| u32::try_from(i).ok().and_then(std::num::NonZeroU32::new))
        .find(|i| {
            book.indexed(*i)
                .is_some_and(|s| s.name.eq_ignore_ascii_case("Invisibility"))
        })
        .ok_or_else(|| "step=cast observed=served-Invisibility-absent need=pinned-spell".into())
}
fn own(client: &DevClientSession) -> TestResult<oteryn_dev_client::EntityRef> {
    let entities = client
        .world_entities()
        .ok_or("step=visibility observed=entity-snapshot-absent")?;
    entities
        .iter()
        .find(|e| &e.entity.identity == entities.own_identity())
        .map(|e| e.entity)
        .ok_or_else(|| "step=visibility observed=own-entity-absent".into())
}
fn visible(client: &DevClientSession, target: oteryn_dev_client::EntityRef) -> bool {
    client
        .world_entities()
        .is_some_and(|entities| entities.iter().any(|e| e.entity == target))
}
fn rat_damage(client: &mut DevClientSession, rat: oteryn_dev_client::EntityRef) -> Option<(u64, u32)> {
    use oteryn_protocol_oteryn::analyser::{AnalyserDamageSource, AnalyserFact};
    client.take_events().into_iter().find_map(|event| {
        let SessionEvent::AnalyserAttributedFacts(delta) = event else { return None };
        delta.value.iter().find_map(|event| match event.fact {
            AnalyserFact::DamageInput { value, source: AnalyserDamageSource::Creature(_), .. }
                if event.attacker == Some(rat) => Some((delta.server_sequence, value.get())),
            _ => None,
        })
    })
}
pub(super) async fn wait_visible(
    client: &mut DevClientSession,
    target: oteryn_dev_client::EntityRef,
    expected: bool,
) -> TestResult<u64> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut sequence = 0;
    loop {
        for event in client.take_events() {
            if let SessionEvent::WorldSpatialEntities(delta) = event {
                let matches = if expected {
                    delta.value.enter.iter().any(|entry| entry.entity == target)
                } else {
                    delta.value.leave.contains(&target)
                };
                if matches {
                    sequence = delta.server_sequence;
                }
            }
        }
        if sequence != 0 && visible(client, target) == expected {
            return Ok(sequence);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("C17b missing visibility transition target={target:?} expected={expected} last_sequence={}", client.last_server_sequence()).into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let (result, detail, _, _) = logout_once(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={detail} need=accepted-logout").into());
    }
    Ok(())
}

pub(super) async fn invisibility_remaining(
    client: &mut DevClientSession,
) -> TestResult<Option<i64>> {
    Ok(invisibility_sample(client).await?.0)
}

pub(super) async fn invisibility_sample(client: &mut DevClientSession) -> TestResult<(Option<i64>, u64)> {
    use oteryn_protocol_oteryn::condition_status as wire;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CONDITION_STATUS_V1,
            wire::COMMAND_TYPE_CONDITION_STATUS_QUERY,
            &[],
        )
        .await?;
    require_mechanic_acceptance(reply.status, "C17b condition status")?;
    let state =
        wire::ConditionStatus::decode(&reply.payload).ok_or("C17b malformed condition status")?;
    Ok((state
        .conditions
        .iter()
        .find(|condition| condition.icon == 9)
        .map(|condition| condition.remaining_ms), reply.server_sequence))
}

pub(super) async fn wait_rat_combat_end(caster: &mut DevClientSession) -> TestResult<(u32, u32)> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(75);
    loop {
        let state = caster
            .session_mut()
            .combat_state()
            .ok_or("C17b combat state absent before logout")?;
        if !state.in_fight {
            let vitals = caster
                .actor_vitals()
                .ok_or("C17b vitals absent before logout")?;
            return Ok((vitals.mana, vitals.soul));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("C17b combat protection did not clear before first logout".into());
        }
        caster
            .session_mut()
            .service_liveness_discarding_events(Duration::from_millis(100))
            .await?;
    }
}

pub(super) async fn seeded_caster(
    clients: &SeamClients<'_>,
    row: &str,
    tag: u8,
    spell: &crate::spell::SpellDefinition,
) -> TestResult<DevClientSession> {
    prepare_caster(clients, row, tag, spell, None).await
}

const SPEAR: u32 = 3277;
const SPEAR_COUNT: u32 = 5;

async fn prepare_caster(
    clients: &SeamClients<'_>,
    row: &str,
    tag: u8,
    spell: &crate::spell::SpellDefinition,
    weapon: Option<(u32, u32)>,
) -> TestResult<DevClientSession> {
    character_prep::declare_seeded(row, "sorcerer-level-35", "Invisibility cast prerequisites");
    let mut initial = connect_character(clients, tag, 0).await?;
    let previous = initial.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(previous.as_bytes()),
        "--profile",
        "c17b-sorcerer".into(),
    )
    .await?;
    if let Some((id, count)) = weapon {
        character_prep::grant_item(&initial, id, count).await?;
    }
    logout(&mut initial).await?;
    drop(initial);
    let mut caster = connect_character(clients, tag, 0).await?;
    if caster.join_snapshot().game_session_id == previous {
        return Err("C17b caster preparation reused the original session".into());
    }
    let level = full_e2ef5::prepare_spell_access(&mut caster, spell).await?;
    let vocation = full_e2ef5::current_vocation(&mut caster).await?;
    if vocation != crate::spell::Vocation::Sorcerer {
        return Err(format!("C17b regeneration fixture requires base Sorcerer, received {vocation:?}").into());
    }
    let vitals = caster.actor_vitals().ok_or("C17b caster vitals absent")?;
    if level != 35 || vitals.mana != vitals.max_mana || vitals.max_mana < 440 {
        return Err(format!("C17b seeded caster has level={level} vitals={vitals:?}").into());
    }
    use oteryn_protocol_oteryn::item_view::{ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome};
    let equipment = caster
        .inventory()
        .ok_or("C17b caster inventory absent")?
        .equipment
        .clone();
    for equipped in equipment {
        let result = caster
            .move_item(&ItemMoveIntent {
                source: equipped.item.handle,
                destination: ItemMoveDestination::MainBackpack,
            })
            .await?;
        if result.outcome != ItemMoveOutcome::Moved {
            return Err(format!("FIXTURE C17b unequip {:?}: {result:?}", equipped.slot).into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            if caster.inventory().is_some_and(|inventory| {
                inventory.equipment.iter().all(|entry| entry.slot != equipped.slot)
            }) {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("FIXTURE C17b slot {:?} remained equipped", equipped.slot).into());
            }
            caster.service_liveness(Duration::from_millis(50)).await?;
        }
    }
    Ok(caster)
}

// Plan with occupancy before sending STEP. An unsuccessful STEP ends the
// journey. It cannot select another tile or another spawn.
pub(super) struct Staging<'a> {
    world: &'a BundleWorld,
    spawn_cells: BTreeSet<TilePos>,
}

impl<'a> Staging<'a> {
    pub(super) fn new(world: &'a BundleWorld, spawns: &oteryn_world_bundle::spawn::Table) -> Self {
        let spawn_cells = spawns
            .sources
            .iter()
            .flat_map(|source| {
                source.points.iter().map(|point| TilePos {
                    x: point.x,
                    y: point.y,
                    floor: source.floor,
                })
            })
            .collect();
        Self { world, spawn_cells }
    }

    fn path(
        &self,
        client: &DevClientSession,
        peer: Option<&DevClientSession>,
        goal: impl Fn(TilePos) -> bool,
    ) -> TestResult<Vec<StepDirection>> {
        let from = tile(client.world_spatial().actor_position)?;
        let occupied = Self::occupied(client, peer)?;
        route_avoiding(self.world, from, goal, |at| {
            !self.spawn_cells.contains(&at) && !occupied.contains(&at)
        })
        .ok_or_else(|| format!("C17b no eligible route from {from:?}").into())
    }

    fn occupied(
        client: &DevClientSession,
        peer: Option<&DevClientSession>,
    ) -> TestResult<BTreeSet<TilePos>> {
        let view = client
            .world_entities()
            .ok_or("C17b route needs an entity snapshot")?;
        Ok(view
            .iter()
            .filter(|entry| &entry.entity.identity != view.own_identity())
            .chain(
                peer.and_then(DevClientSession::world_entities)
                    .into_iter()
                    .flat_map(|view| view.iter()),
            )
            .filter_map(|entry| tile(entry.position).ok())
            .collect())
    }

    async fn return_to_town(&self, client: &mut DevClientSession) -> TestResult {
        let from = tile(client.world_spatial().actor_position)?;
        let occupied = Self::occupied(client, None)?;
        let path = find_route(
            from,
            |at| at == TOWN,
            |at, direction| {
                destination(self.world, at, direction)
                    .filter(|next| in_return_search(*next, from) && !occupied.contains(next))
            },
        ).map_err(|error| format!("C17b no eligible return route from {from:?} to {TOWN:?}: {error}"))?;
        self.walk(client, &path).await
    }

    async fn walk(&self, client: &mut DevClientSession, path: &[StepDirection]) -> TestResult {
        for &direction in path {
            let from = tile(client.world_spatial().actor_position)?;
            let expected =
                destination(self.world, from, direction).ok_or("C17b planned landing absent")?;
            Self::wait_for_free_landing(client, expected).await?;
            let (outcome, replies) =
                oteryn_dev_client::native_session::test_support::capture_command_results(
                    client.step(direction),
                )
                .await;
            let outcome = outcome?;
            let actual = tile(client.world_spatial().actor_position)?;
            if outcome.status != oteryn_protocol_oteryn::CommandStatus::Accepted
                || outcome.disposition != StepDisposition::Moved
                || outcome.world_spatial_delta.is_none()
                || outcome.step_wait_ms == 0
                || actual != expected
                || actual == from
            {
                let ground = self
                    .world
                    .base()
                    .tile(expected.x, expected.y, expected.floor);
                let occupants = client
                    .world_entities()
                    .into_iter()
                    .flat_map(|view| view.iter())
                    .filter(|entity| tile(entity.position).ok() == Some(expected))
                    .map(|entity| (entity.entity, entity.kind))
                    .collect::<Vec<_>>();
                evidence(&format!(
                    "C17B_STEP_TILE command={} sequence={} at={expected:?} ground={:?} items={:?} walkable={} enterable={} no_logout={} protection_zone={} view_occupants={occupants:?}",
                    outcome.command_id,
                    outcome.result_server_sequence,
                    ground.as_ref().and_then(|tile| tile.ground()),
                    ground.as_ref().map(|tile| tile.ids()),
                    ground.as_ref().is_some_and(|tile| tile.walkable()),
                    self.world.enterable(expected),
                    ground.as_ref().is_some_and(|tile| tile.no_logout()),
                    ground.as_ref().is_some_and(|tile| tile.protection_zone())
                ));
                for reply in replies {
                    let Some(sequence) = oteryn_protocol_oteryn::decode_wire_envelope(&reply)
                        .ok()
                        .map(|frame| frame.server_sequence())
                    else {
                        continue;
                    };
                    if sequence == outcome.result_server_sequence {
                        let hex = reply
                            .iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect::<String>();
                        evidence(&format!(
                            "C17B_FIRST_STEP_FRAME sequence={sequence} command={} frame={hex}",
                            outcome.command_id
                        ));
                    }
                }
                return Err(format!(
                    "C17b first STEP {direction:?} from={from:?} expected={expected:?} actual={actual:?}: {outcome:?}"
                ).into());
            }
            verdict::Harness::global().progress("walk");
            client.take_events();
            client
                .session_mut()
                .service_liveness_discarding_events(Duration::from_millis(u64::from(
                    outcome.step_wait_ms,
                )))
                .await?;
        }
        Ok(())
    }

    async fn wait_for_free_landing(client: &mut DevClientSession, landing: TilePos) -> TestResult {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
        let mut reported = false;
        loop {
            let view = client
                .world_entities()
                .ok_or("C17b fixture step requires an entity view")?;
            let occupant = view.iter().find(|entity| {
                &entity.entity.identity != view.own_identity()
                    && tile(entity.position).ok() == Some(landing)
            });
            let Some(occupant) = occupant else {
                return Ok(());
            };
            if !reported {
                evidence(&format!(
                    "FIXTURE_PLACEMENT C17b waiting for landing={landing:?} occupant={:?}",
                    occupant.entity
                ));
                reported = true;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!(
                    "FIXTURE_PLACEMENT C17b landing={landing:?} remains occupied by {:?} at sequence={}",
                    occupant.entity,
                    client.last_server_sequence()
                )
                .into());
            }
            client.service_liveness(Duration::from_millis(100)).await?;
        }
    }

    pub(super) async fn encounter(
        &self,
        caster: &mut DevClientSession,
        observer: &mut DevClientSession,
        capture: &oteryn_dev_client::native_session::test_support::WireCapture,
    ) -> TestResult {
        // At the north exit, the adjacent observer can see the caster from
        // the protection zone without becoming a second target for the Rat.
        let caster_site = TilePos {
            x: 32369,
            y: 32230,
            floor: -7,
        };
        let observer_site = TilePos {
            y: 32231,
            ..caster_site
        };
        for (site, protected) in [(caster_site, false), (observer_site, true)] {
            let ground = self.world.base().tile(site.x, site.y, site.floor);
            if !self.world.enterable(site)
                || !ground
                    .is_some_and(|tile| tile.walkable() && tile.protection_zone() == protected)
                || self.spawn_cells.contains(&site)
            {
                return Err(format!("C17b staging tile ineligible: {site:?}").into());
            }
        }
        let path = self.path(caster, None, |tile| tile == caster_site)?;
        super::mp::observations::while_servicing_liveness(self.walk(caster, &path), async || {
            capture
                .observe(observer.service_liveness(Duration::from_millis(25)))
                .await?;
            observer.take_events();
            Ok(())
        })
        .await??;
        let path = self.path(observer, Some(caster), |tile| tile == observer_site)?;
        provisioning::with_peer_liveness(caster, capture.observe(self.walk(observer, &path)))
            .await?;
        if tile(caster.world_spatial().actor_position)? != caster_site
            || tile(observer.world_spatial().actor_position)? != observer_site
        {
            return Err("C17b staging did not land at the fixed adjacent tiles".into());
        }
        evidence(&format!(
            "FIXTURE_PLACEMENT C17b encounter caster={caster_site:?} observer={observer_site:?}"
        ));
        Ok(())
    }
}

fn in_return_search(at: TilePos, from: TilePos) -> bool {
    let nearest = TilePos {
        x: at.x.clamp(from.x.min(TOWN.x), from.x.max(TOWN.x)),
        y: at.y.clamp(from.y.min(TOWN.y), from.y.max(TOWN.y)),
        floor: at
            .floor
            .clamp(from.floor.min(TOWN.floor), from.floor.max(TOWN.floor)),
    };
    map_journey::in_town(at, nearest)
}

pub(super) async fn place_rat(
    caster: &mut DevClientSession,
) -> TestResult<oteryn_dev_client::EntityRef> {
    if caster.inventory().is_none_or(|inventory| !inventory.equipment.is_empty()) {
        return Err("FIXTURE C17b requires a caster with empty equipment slots".into());
    }
    let defence = read_defence(caster).await?;
    spawn_rat(caster, defence).await
}

struct Defence {
    armour: i64,
    total: Option<i64>,
    equipment: Option<i64>,
}

async fn read_defence(caster: &mut DevClientSession) -> TestResult<Defence> {
    use oteryn_dev_client::client_ui::cyclopedia_character::{
        CyclopediaCharacterPanel, Intent, request,
    };
    use oteryn_protocol_oteryn::cyclopedia_character::{
        CharacterTab,
        combat::{CombatStat, CombatStats},
    };
    let mut panel = CyclopediaCharacterPanel {
        selected: true,
        tab: CharacterTab::Defence,
        ..Default::default()
    };
    let command = request(&mut panel, Intent::Tab).ok_or("C17b defence tab unavailable")?;
    let reply = caster
        .session_mut()
        .gated_command(command.capability, command.kind, &command.payload)
        .await?;
    let receipt = format!(
        "FIXTURE C17b defence tab status={:?} command={} sequence={} payload_bytes={}",
        reply.status, reply.command_id, reply.server_sequence, reply.payload.len()
    );
    evidence(&receipt);
    require_mechanic_acceptance(reply.status, &receipt)?;
    let stats = CombatStats::decode(&reply.payload)
        .map_err(|error| format!("C17b defence tab decode: {error:?}"))?;
    if stats.tab() != CharacterTab::Defence {
        return Err(format!("C17b expected defence tab, received {:?}", stats.tab()).into());
    }
    let armour = stats.get(CombatStat::Armor).ok_or("C17b armour stat absent")?;
    if armour != 0 {
        return Err(format!("FIXTURE C17b unarmoured caster has armour={armour}").into());
    }
    Ok(Defence {
        armour,
        total: stats.get(CombatStat::DefenceTotal),
        equipment: stats.get(CombatStat::DefenceEquipment),
    })
}

async fn spawn_rat(
    caster: &DevClientSession,
    Defence { armour, total, equipment }: Defence,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let position = caster.world_spatial().actor_position;
    let caster_site = tile(position)?;
    let rat_site = TilePos {
        y: caster_site.y.checked_sub(1).ok_or("C17b Rat tile outside the map")?,
        ..caster_site
    };
    let placement = request_fixture_control(
        uuid_text(caster.join_snapshot().game_session_id.as_bytes()),
        "--spawn-at",
        format!("{},{},{}:oteryn:creature.rat", rat_site.x, rat_site.y, position.floor),
    )
    .await
    .map_err(|error| format!("FIXTURE_PLACEMENT C17b Rat failed at {rat_site:?}: {error}"))?;
    let rat = parse_spawned_entity(&placement)?;
    evidence(&format!(
        "FIXTURE_PLACEMENT C17b caster={caster_site:?} rat_site={rat_site:?} rat={rat:?} armour={armour} defence={total:?} equipment_defence={equipment:?}"
    ));
    Ok(rat)
}

pub(super) async fn wait_for_rat_hit(
    caster: &mut DevClientSession,
    rat: oteryn_dev_client::EntityRef,
) -> TestResult<u64> {
    if !caster
        .session_mut()
        .is_selected(oteryn_protocol_oteryn::analyser::CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1)
    {
        return Err("FIXTURE C17b requires analyser hit attribution capability 486".into());
    }
    let caster_tile = tile(caster.world_spatial().actor_position)?;
    caster.take_events();
    let visible_health = caster
        .actor_vitals()
        .ok_or("C17b visible caster vitals absent")?
        .health;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut damage = None;
    let mut saw_adjacent = false;
    let wire = oteryn_dev_client::native_session::test_support::WireCapture::default();
    loop {
        wire.observe(caster.service_liveness(Duration::from_millis(100)))
            .await?;
        let rat_tile = caster
            .world_entities()
            .and_then(|view| view.get(&rat))
            .map(|entity| tile(entity.position))
            .transpose()?;
        saw_adjacent |= rat_tile.is_some_and(|at| {
            at.floor == caster_tile.floor
                && at
                    .x
                    .abs_diff(caster_tile.x)
                    .max(at.y.abs_diff(caster_tile.y))
                    == 1
        });
        if let Some(hit) = rat_damage(caster, rat) {
            damage.get_or_insert(hit);
        }
        let health = caster
            .actor_vitals()
            .ok_or("C17b visible caster vitals absent")?
            .health;
        if let Some((sequence, amount)) = damage
            && saw_adjacent && visible_health.checked_sub(health) == Some(amount) {
            evidence(&format!(
                "FIXTURE C17b attributed_rat_damage sequence={sequence} damage={amount} health_before={visible_health} health_after={health}"
            ));
            return Ok(sequence);
        }
        if tokio::time::Instant::now() >= deadline {
            let (received, _) = wire.take();
            if let Some((sequence, bytes)) = received.iter().rev().find_map(|bytes| {
                let frame = oteryn_protocol_oteryn::decode_wire_envelope(bytes).ok()?;
                if frame.message_type() != oteryn_protocol_oteryn::MessageType::StateDelta {
                    return None;
                }
                let delta = oteryn_protocol_oteryn::decode_state_delta(frame.payload()).ok()?;
                (delta.domain_id == 15).then_some((frame.server_sequence(), bytes))
            }) {
                let hex = bytes
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                evidence(&format!(
                    "C17B_RAT_LAST_ANALYSER_FRAME sequence={sequence} frame={hex}"
                ));
            }
            return Err(format!("FIXTURE_PLACEMENT C17b Rat timeout exact_rat={rat:?} last_tile={rat_tile:?} saw_adjacent={saw_adjacent} attributed_hit={damage:?} health_before={visible_health} health_after={health}").into());
        }
    }
}

pub(super) async fn require_rat_stops(
    caster: &mut DevClientSession,
    rat: oteryn_dev_client::EntityRef,
) -> TestResult {
    let hidden_health = caster
        .actor_vitals()
        .ok_or("C17b hidden caster vitals absent")?
        .health;
    let no_damage_until = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < no_damage_until {
        caster.service_liveness(Duration::from_millis(100)).await?;
        if rat_damage(caster, rat).is_some()
            || caster
                .actor_vitals()
                .is_none_or(|vitals| vitals.health < hidden_health)
        {
            return Err("step=monster-drops-invisible observed=attributed-rat-damage-after-cast need=AI-invisibility-filter".into());
        }
    }
    Ok(())
}

// The full-row caster wears no armour or shield. A plain spear replaces
// fist defence with zero without changing the Rat or its damage rolls.
async fn full_caster_loadout(caster: &mut DevClientSession) -> TestResult<Defence> {
    use oteryn_protocol_oteryn::item_view::{EquipmentSlot, ItemMoveOutcome};
    let spear = provisioning::item_reference(&provisioning::item_index()?, SPEAR)?;
    let result = caster.session_mut().equip_item_definition(spear).await?;
    if result.outcome != ItemMoveOutcome::Moved {
        return Err(format!("FIXTURE C17b spear equip refused: {result:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        if caster.inventory().is_some_and(|inventory| {
            matches!(inventory.equipment.as_slice(), [entry]
                if entry.slot == EquipmentSlot::RightHand && entry.item.item_definition_ref == spear)
        }) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("FIXTURE C17b expected only the spear in the right hand".into());
        }
        caster.service_liveness(Duration::from_millis(50)).await?;
    }
    let defence = read_defence(caster).await?;
    if defence.total != Some(0) || defence.equipment != Some(0) {
        return Err(format!("FIXTURE C17b spear defence total={:?} equipment={:?}", defence.total, defence.equipment).into());
    }
    Ok(defence)
}

// Global base Sorcerer, including a Premium character without promotion:
// https://tibia.fandom.com/wiki/Sorcerer#Regeneration
// Vitals frames contain full pool values but no tick timestamp. Prove at most
// one credit per pool within a sub-period window, even across buffered frames.
pub(super) fn verify_recast(
    outcome: &oteryn_dev_client::native_session::CastOutcome,
    paid: oteryn_protocol_oteryn::actor_spell::ActorVitals,
    sequence: u64,
    after: Option<oteryn_protocol_oteryn::actor_spell::ActorVitals>,
    events: &[SessionEvent],
    elapsed: Duration,
) -> TestResult {
    require_mechanic_acceptance(outcome.status, "C17b cooldown recast")?;
    if outcome.disposition != SpellCastDisposition::CoolingDown
        || outcome.actor_vitals_delta.is_some() || outcome.result_server_sequence <= sequence {
        return Err(format!("C17b invalid cooldown receipt {outcome:?} after sequence={sequence}").into());
    }
    let mut observed = paid;
    let mut last = sequence;
    for event in events {
        let SessionEvent::ActorVitals(delta) = event else { continue };
        if delta.server_sequence <= sequence { continue; }
        if delta.server_sequence <= last || delta.server_sequence >= outcome.result_server_sequence {
            return Err(format!("C17b out-of-order recast vitals sequence={}", delta.server_sequence).into());
        }
        verify_regen_credit(paid, observed, delta.value, elapsed)?;
        observed = delta.value;
        last = delta.server_sequence;
    }
    if after != Some(observed) {
        return Err(format!("C17b unexplained recast vitals paid={paid:?} observed={observed:?} after={after:?}").into());
    }
    evidence(&format!("C17b cooldown vitals before_sequence={sequence} after_sequence={} elapsed_ms={} mana={}->{} health={}->{}", outcome.result_server_sequence, elapsed.as_millis(), paid.mana, observed.mana, paid.health, observed.health));
    Ok(())
}

fn verify_regen_credit(
    paid: oteryn_protocol_oteryn::actor_spell::ActorVitals,
    previous: oteryn_protocol_oteryn::actor_spell::ActorVitals,
    next: oteryn_protocol_oteryn::actor_spell::ActorVitals,
    elapsed: Duration,
) -> TestResult {
    let credit = |initial: u32, previous: u32, next: u32, cap: u32, amount: u32, seconds| {
        next == previous || (previous == initial && next > previous
            && next == previous.saturating_add(amount).min(cap)
            && elapsed < Duration::from_secs(seconds))
    };
    if !credit(paid.health, previous.health, next.health, paid.max_health, 1, 12)
        || !credit(paid.mana, previous.mana, next.mana, paid.max_mana, 2, 3)
        || (oteryn_protocol_oteryn::actor_spell::ActorVitals {
            health: previous.health,
            mana: previous.mana,
            ..next
        }) != previous || next == previous {
        return Err(format!("C17b unproved recast regeneration before={previous:?} after={next:?} window_ms={}", elapsed.as_millis()).into());
    }
    Ok(())
}

fn require_continuous_visibility(
    observer: &mut DevClientSession,
    caster: oteryn_dev_client::EntityRef,
) -> TestResult {
    if observer.take_events().into_iter().any(|event| matches!(event,
        SessionEvent::WorldSpatialEntities(delta)
            if delta.value.leave.iter().any(|entity| entity.identity == caster.identity)))
        || !visible(observer, caster)
    {
        return Err("C17b invisible caster left the observer view before logout".into());
    }
    Ok(())
}

fn verify_unchanged_recast(
    caster: &mut DevClientSession,
    outcome: &oteryn_dev_client::native_session::CastOutcome,
    paid: oteryn_protocol_oteryn::actor_spell::ActorVitals,
    sequence: u64,
) -> TestResult {
    let events = caster.take_events();
    verify_recast(outcome, paid, sequence, caster.actor_vitals().copied(), &events, Duration::ZERO)?;
    if events.iter().any(|event| matches!(event, SessionEvent::ActorVitals(delta)
        if delta.server_sequence > sequence))
    {
        return Err("C17b cooldown probe changed vitals without an established source".into());
    }
    Ok(())
}

fn verify_relog_timer(
    before: i64,
    after: i64,
    online_min: Duration,
    online_max: Duration,
    offline_min: Duration,
) -> TestResult {
    let min = i64::try_from(online_min.as_millis())?.saturating_sub(1).max(0);
    let max = i64::try_from(online_max.as_micros().div_ceil(1_000))? + 1;
    if i64::try_from(offline_min.as_millis())? <= max - min {
        return Err("C17b offline interval cannot distinguish a paused timer".into());
    }
    let lost = before.checked_sub(after).ok_or("C17b condition loss overflow")?;
    if after <= 0 || !(min..=max).contains(&lost) {
        return Err(format!("C17b condition lost {lost}ms with only {min}..={max}ms online").into());
    }
    Ok(())
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
    let fixture = character_prep::dedicated_character(clients, "NODE_BOOT_C17B_ACCOUNT").await?;
    let caster_clients = fixture.clients(clients);
    let book = spell_book()?;
    let spell = book
        .spoken("utana vid")
        .ok_or("step=cast observed=words-unresolved need=Invisibility-catalog")?
        .spell;
    let crate::spell::ManaCost::Fixed(cost) = spell.mana else {
        return Err("step=cast observed=non-fixed-mana need=assertable-cost".into());
    };
    if cost != 440 {
        return Err(format!("C17b served mana cost={cost} expected=440 per Global Tibia").into());
    }
    let mut caster = prepare_caster(&caster_clients, "C17b.full", 0xb1, spell, Some((SPEAR, SPEAR_COUNT)))
        .await
        .map_err(|error| format!("step=invisibility-access spell=utana vid: {error}"))?;
    let defence = full_caster_loadout(&mut caster).await?;
    let mut observer = connect_character(clients, 0xb2, 0).await?;
    let staging = Staging::new(world, &metadata.spawns);
    let capture = oteryn_dev_client::native_session::test_support::WireCapture::default();
    staging.encounter(&mut caster, &mut observer, &capture).await?;
    drop(capture);
    let rat = spawn_rat(&caster, defence).await?;
    let caster_ref = own(&caster)?;
    let pre_damage =
        provisioning::with_peer_liveness(&mut observer, wait_for_rat_hit(&mut caster, rat)).await?;
    row.pass("monster-targets-visible", pre_damage)?;
    observer
        .service_liveness(Duration::from_millis(100))
        .await?;
    if !visible(&observer, caster_ref) {
        return Err("C17b observer did not see the caster before casting".into());
    }
    row.pass("player-sees-visible", observer.last_server_sequence())?;
    if invisibility_remaining(&mut caster).await?.is_some() {
        return Err("C17b caster already invisible before the cast".into());
    }
    let before = caster
        .actor_vitals()
        .copied()
        .ok_or("step=cast observed=vitals-absent")?;
    caster.take_events();
    observer.take_events();
    let cast = caster
        .cast_spell(spell_index(&book)?, SpellTarget::None, false)
        .await?;
    require_mechanic_acceptance(cast.status, "C17b.full Invisibility")?;
    if cast.disposition != SpellCastDisposition::Cast
        || cast.actor_vitals_delta.as_ref().is_none_or(|d| {
            d.server_sequence == 0 || before.mana.checked_sub(cost) != Some(d.value.mana) || before.soul != d.value.soul
        })
    {
        return Err(format!("step=cast-and-cost observed=status:{:?}/disposition:{:?}/vitals:{:?} need=Cast-and-exact-mana-and-soul", cast.status, cast.disposition, cast.actor_vitals_delta).into());
    }
    row.pass("cast-and-cost", cast.result_server_sequence)?;
    let paid = cast.actor_vitals_delta.as_ref().ok_or("C17b cast vitals absent")?;
    let cooldown = caster.cast_spell(spell_index(&book)?, SpellTarget::None, false).await?;
    verify_unchanged_recast(&mut caster, &cooldown, paid.value, paid.server_sequence)?;
    let light = super::client_spell_journey::spell_index(&book, "utevo lux")?;
    if paid.value.mana < 20 {
        return Err("C17b lacks mana for the 20-mana Support cooldown probe".into());
    }
    let support = caster.cast_spell(light, SpellTarget::None, false).await?;
    verify_unchanged_recast(&mut caster, &support, paid.value, cooldown.result_server_sequence)?;
    row.pass("cooldown-no-cost", support.result_server_sequence)?;
    let active = invisibility_remaining(&mut caster)
        .await?
        .ok_or("C17b active condition absent after cast")?;
    if !(195_000..=200_000).contains(&active) {
        return Err(
            format!("C17b initial invisibility duration {active}ms outside Global range").into(),
        );
    }
    require_rat_stops(&mut caster, rat).await?;
    row.pass("monster-drops-invisible", cast.result_server_sequence)?;
    observer
        .service_liveness(Duration::from_millis(100))
        .await?;
    require_continuous_visibility(&mut observer, caster_ref)?;
    row.pass("player-sees-invisible", observer.last_server_sequence())?;
    use oteryn_protocol_oteryn::{
        attack::AttackIntentDisposition,
        presentation::{PresentationEventKind, PresentationValueKind},
    };
    let rat_position = caster
        .world_entities()
        .and_then(|view| view.get(&rat))
        .ok_or("C17b exact Rat absent before caster attack")?
        .position;
    let rat_tile = tile(rat_position)?;
    let approach = staging.path(&caster, Some(&observer), |at| {
        at.floor == rat_tile.floor && at.x.abs_diff(rat_tile.x).max(at.y.abs_diff(rat_tile.y)) == 1
    })?;
    full_e2ef5::with_observer_liveness(&mut observer, staging.walk(&mut caster, &approach)).await?;
    if invisibility_remaining(&mut caster).await?.is_none_or(|time| time <= 0) {
        return Err("C17b caster lost Invisible before attacking the Rat".into());
    }
    caster.take_events();
    let selected = caster.session_mut().attack_target(Some(&rat)).await?;
    if selected.disposition != AttackIntentDisposition::Ok {
        return Err(format!("C17b Rat target refused: {selected:?}").into());
    }
    let damage = full_e2ef5::with_observer_liveness(&mut observer, async {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            for event in caster.take_events() {
                if let SessionEvent::Presentation(delta) = event
                    && delta.server_sequence > selected.result_server_sequence
                    && delta.value.iter().any(|event| matches!(event.kind,
                        PresentationEventKind::ValueText {kind: PresentationValueKind::DamageDealt, attacker: Some(attacker), target, primary, ..}
                        if attacker == caster_ref && target == rat && primary.value > 0))
                { return Ok(delta.server_sequence); }
            }
            if tokio::time::Instant::now() >= deadline { return Err("C17b invisible caster did not damage the exact Rat".into()); }
            caster.service_liveness(Duration::from_millis(100)).await?;
        }
    }).await?;
    if invisibility_remaining(&mut caster).await?.is_none_or(|time| time <= 0) {
        return Err("C17b caster lost Invisible by the attributed hit".into());
    }
    row.pass("caster-damages-rat", damage)?;
    if caster
        .session_mut()
        .combat_state()
        .is_none_or(|state| state.target != Some(rat))
    {
        return Err("C17b exact Rat target absent before clear".into());
    }
    full_e2ef5::with_observer_liveness(&mut observer, async {
        let clear = caster.session_mut().attack_target(None).await?;
        require_mechanic_acceptance(clear.status, "C17b Rat target clear")?;
        if clear.disposition != AttackIntentDisposition::Ok {
            return Err(format!("C17b Rat target clear refused: {clear:?}").into());
        }
        // Domain 10 follows the command result on the same connection.
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            if caster.take_events().into_iter().any(|event| {
                matches!(event, SessionEvent::ActorCombatState(delta)
                    if delta.server_sequence > clear.result_server_sequence && delta.value.target.is_none())
            }) && caster
                .session_mut()
                .combat_state()
                .is_some_and(|state| state.target.is_none())
            {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("C17b Rat target did not clear after {clear:?}").into());
            }
            caster.service_liveness(Duration::from_millis(50)).await?;
        }
    })
    .await?;
    let session = caster.join_snapshot().game_session_id;
    let position = caster.world_spatial().actor_position;
    let durable_vitals = full_e2ef5::with_observer_liveness(
        &mut observer, wait_rat_combat_end(&mut caster)
    ).await?;
    if durable_vitals.0 >= before.mana || durable_vitals.1 != before.soul {
        return Err("C17b persisted resources disagree with the paid cast".into());
    }
    use tokio::time::Instant;
    let pre_sent = Instant::now();
    let (pre_logout, pre_sequence) = full_e2ef5::with_observer_liveness(
        &mut observer, invisibility_sample(&mut caster)
    ).await?;
    let pre_received = Instant::now();
    let pre_logout = pre_logout.ok_or("C17b missing pre-logout Invisible timer")?;
    require_continuous_visibility(&mut observer, caster_ref)?;
    let logout_sent = Instant::now();
    full_e2ef5::with_observer_liveness(&mut observer, logout(&mut caster)).await?;
    let logout_received = Instant::now();
    wait_visible(&mut observer, caster_ref, false).await?;
    full_e2ef5::with_observer_liveness(&mut observer, async {
        tokio::time::sleep(Duration::from_secs(20)).await;
        Ok(())
    }).await?;
    let login_sent = Instant::now();
    caster = full_e2ef5::with_observer_liveness(
        &mut observer,
        connect_character(&caster_clients, 0xb3, 0),
    )
    .await?;
    let login_received = Instant::now();
    if caster.join_snapshot().game_session_id == session
        || caster.world_spatial().actor_position != position
        || caster
            .actor_vitals()
            .is_none_or(|v| (v.mana, v.soul) != durable_vitals)
    {
        return Err("step=fresh-admission-condition observed=session/position/vitals-mismatch need=durable-condition-state".into());
    }
    let fresh_ref = own(&caster)?;
    if fresh_ref == caster_ref {
        return Err("C17b fresh admission reused entity incarnation".into());
    }
    let post_sent = Instant::now();
    let (post_login, post_sequence) = full_e2ef5::with_observer_liveness(
        &mut observer, invisibility_sample(&mut caster)
    ).await?;
    let post_received = Instant::now();
    let post_login = post_login.ok_or("C17b missing post-login Invisible timer")?;
    let online_min = (logout_sent - pre_received) + (post_sent - login_received);
    let online_max = (logout_received - pre_sent) + (post_received - login_sent);
    let offline_min = login_sent - logout_received;
    evidence(&format!("C17B_FULL_TIMER pre_sequence={pre_sequence} post_sequence={post_sequence} before_ms={pre_logout} after_ms={post_login} online_min_us={} online_max_us={} offline_min_us={}", online_min.as_micros(), online_max.as_micros(), offline_min.as_micros()));
    verify_relog_timer(pre_logout, post_login, online_min, online_max, offline_min)?;
    if pre_logout >= active {
        return Err("C17b Invisible timer did not decrease during the online encounter".into());
    }
    let enter_sequence = wait_visible(&mut observer, fresh_ref, true).await?;
    row.pass("fresh-admission-condition", enter_sequence)?;
    provisioning::with_peer_liveness(&mut observer, staging.return_to_town(&mut caster)).await?;
    logout(&mut caster).await?;
    staging.return_to_town(&mut observer).await?;
    logout(&mut observer).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relog_timer_rejects_offline_decay_and_reset() -> TestResult {
        let min = Duration::from_secs(2);
        let max = Duration::from_secs(3);
        let offline = Duration::from_secs(20);
        verify_relog_timer(140_000, 137_500, min, max, offline)?;
        assert!(verify_relog_timer(140_000, 117_500, min, max, offline).is_err());
        assert!(verify_relog_timer(140_000, 199_000, min, max, offline).is_err());
        Ok(())
    }
    #[test]
    fn recast_regeneration_rejects_a_charge_and_unproved_credits() -> TestResult {
        use oteryn_protocol_oteryn::actor_spell::ActorVitals;
        let paid = ActorVitals { mana: 460, max_mana: 900, health: 314, max_health: 320, ..Default::default() };
        let next = ActorVitals { mana: 462, health: 315, ..paid };
        let elapsed = Duration::from_secs(1);
        verify_regen_credit(paid, paid, next, elapsed)?;
        assert!(verify_regen_credit(paid, next, ActorVitals { mana: 22, ..next }, elapsed).is_err());
        assert!(verify_regen_credit(paid, next, ActorVitals { mana: 464, ..next }, elapsed).is_err());
        assert!(verify_regen_credit(paid, paid, next, Duration::from_secs(3)).is_err());
        Ok(())
    }

    #[test]
    fn return_search_includes_landings_outside_the_town_window() {
        let from = TilePos {
            x: 32536,
            y: 32514,
            floor: -7,
        };
        let next = TilePos {
            y: from.y - 1,
            ..from
        };
        assert!(!map_journey::in_town(next, TOWN));
        assert!(in_return_search(next, from));
        assert!(in_return_search(TOWN, from));
        assert!(!in_return_search(
            TilePos {
                y: u16::MAX,
                ..from
            },
            from
        ));
    }

    #[test]
    fn row_requires_every_lifecycle_receipt() -> TestResult {
        let mut row = Row::new();
        for (index, action) in ACTIONS.iter().enumerate() {
            assert!(row.pass(action, u64::try_from(index + 1)?).is_ok());
        }
        assert!(ACTIONS.iter().all(|action| row.passed.contains(action)));
        Ok(())
    }
}
