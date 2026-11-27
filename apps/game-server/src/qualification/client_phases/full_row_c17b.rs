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
fn damage_sequence(client: &mut DevClientSession, rat: oteryn_dev_client::EntityRef) -> u64 {
    use oteryn_protocol_oteryn::analyser::{AnalyserDamageSource, AnalyserFact};
    client
        .take_events()
        .into_iter()
        .filter_map(|event| match event {
            SessionEvent::AnalyserAttributedFacts(delta)
                if delta.value.iter().any(|event| {
                    event.attacker == Some(rat)
                        && matches!(
                            event.fact,
                            AnalyserFact::DamageInput {
                                source: AnalyserDamageSource::Creature(_),
                                ..
                            }
                        )
                }) =>
            {
                Some(delta.server_sequence)
            }
            _ => None,
        })
        .max()
        .unwrap_or(0)
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
    Ok(state
        .conditions
        .iter()
        .find(|condition| condition.icon == 9)
        .map(|condition| condition.remaining_ms))
}

pub(super) async fn logout_after_rat_combat(
    caster: &mut DevClientSession,
) -> TestResult<(u32, u32)> {
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
            let paid = (vitals.mana, vitals.soul);
            logout(caster).await?;
            return Ok(paid);
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
    character_prep::declare_seeded(row, "sorcerer-level-35", "Invisibility cast prerequisites");
    let mut initial = connect_character(clients, tag, 0).await?;
    let previous = initial.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(previous.as_bytes()),
        "--profile",
        "c17b-sorcerer".into(),
    )
    .await?;
    logout(&mut initial).await?;
    drop(initial);
    let mut caster = connect_character(clients, tag, 0).await?;
    if caster.join_snapshot().game_session_id == previous {
        return Err("C17b caster preparation reused the original session".into());
    }
    let level = full_e2ef5::prepare_spell_access(&mut caster, spell).await?;
    let vitals = caster.actor_vitals().ok_or("C17b caster vitals absent")?;
    if level != 35 || vitals.mana != vitals.max_mana || vitals.max_mana < 440 {
        return Err(format!("C17b seeded caster has level={level} vitals={vitals:?}").into());
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
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let path = loop {
            let from = tile(client.world_spatial().actor_position)?;
            let occupied = Self::occupied(client, None)?;
            // Cleanup may start outside the outbound map bounds. Keep a
            // bounded corridor between the live position and town.
            if let Ok(path) = find_route(
                from,
                |at| at == TOWN,
                |at, direction| {
                    destination(self.world, at, direction)
                        .filter(|next| in_return_search(*next, from) && !occupied.contains(next))
                },
            ) {
                break path;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(
                    format!("C17b no eligible return route from {from:?} to {TOWN:?}").into(),
                );
            }
            client.service_liveness(Duration::from_millis(100)).await?;
            client.take_events();
        };
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
                return Err(format!("C17b.client staging tile ineligible: {site:?}").into());
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
            return Err("C17b.client staging did not land at the fixed adjacent tiles".into());
        }
        evidence(&format!(
            "FIXTURE_PLACEMENT C17b.client caster={caster_site:?} observer={observer_site:?}"
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
    caster: &DevClientSession,
) -> TestResult<oteryn_dev_client::EntityRef> {
    let caster_site = tile(caster.world_spatial().actor_position)?;
    let placement = request_spawn(
        uuid_text(caster.join_snapshot().game_session_id.as_bytes()),
        "oteryn:creature.rat".into(),
    )
    .await
    .map_err(|error| format!("FIXTURE_PLACEMENT C17b Rat failed: {error}"))?;
    let rat = parse_spawned_entity(&placement)?;
    evidence(&format!(
        "FIXTURE_PLACEMENT C17b caster={caster_site:?} rat={rat:?}"
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
    let mut damage = 0;
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
        damage = damage.max(damage_sequence(caster, rat));
        let health = caster
            .actor_vitals()
            .ok_or("C17b visible caster vitals absent")?
            .health;
        if saw_adjacent && damage != 0 && health < visible_health {
            evidence(&format!(
                "FIXTURE C17b attributed_rat_damage sequence={damage} health_before={visible_health} health_after={health}"
            ));
            return Ok(damage);
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
            return Err(format!("FIXTURE_PLACEMENT C17b Rat timeout exact_rat={rat:?} last_tile={rat_tile:?} saw_adjacent={saw_adjacent} attributed_hit={damage} health_before={visible_health} health_after={health}").into());
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
        if damage_sequence(caster, rat) != 0
            || caster
                .actor_vitals()
                .is_none_or(|vitals| vitals.health < hidden_health)
        {
            return Err("step=monster-drops-invisible observed=attributed-rat-damage-after-cast need=AI-invisibility-filter".into());
        }
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
    let mut caster = seeded_caster(&caster_clients, "C17b.full", 0xb1, spell)
        .await
        .map_err(|error| format!("step=invisibility-access spell=utana vid: {error}"))?;
    let mut observer = connect_character(clients, 0xb2, 0).await?;
    let staging = Staging::new(world, &metadata.spawns);
    let caster_site = TilePos {
        x: 32490,
        y: 32169,
        floor: -7,
    };
    let observer_site = TilePos {
        x: 32488,
        ..caster_site
    };
    let path = staging.path(&caster, None, |tile| tile == caster_site)?;
    provisioning::with_peer_liveness(&mut observer, staging.walk(&mut caster, &path)).await?;
    let path = staging.path(&observer, Some(&caster), |tile| tile == observer_site)?;
    provisioning::with_peer_liveness(&mut caster, staging.walk(&mut observer, &path)).await?;
    if tile(caster.world_spatial().actor_position)? != caster_site
        || tile(observer.world_spatial().actor_position)? != observer_site
    {
        return Err("C17b fixture did not reach the fixed encounter tiles".into());
    }
    let rat = place_rat(&caster).await?;
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
            d.server_sequence == 0 || before.mana.checked_sub(cost) != Some(d.value.mana)
        })
    {
        return Err(format!("step=cast-and-cost observed=status:{:?}/disposition:{:?}/vitals:{:?} need=Cast-and-exact-mana-delta", cast.status, cast.disposition, cast.actor_vitals_delta).into());
    }
    row.pass("cast-and-cost", cast.result_server_sequence)?;
    let paid = caster
        .actor_vitals()
        .copied()
        .ok_or("step=cooldown observed=vitals-absent")?;
    let cooldown = caster
        .cast_spell(spell_index(&book)?, SpellTarget::None, false)
        .await?;
    if cooldown.disposition != SpellCastDisposition::CoolingDown
        || cooldown.actor_vitals_delta.is_some()
        || caster
            .actor_vitals()
            .is_none_or(|v| v.mana != paid.mana || v.soul != paid.soul)
    {
        return Err(format!(
            "step=cooldown-no-cost observed={cooldown:?} need=Cooldown-without-vitals-delta"
        )
        .into());
    }
    row.pass("cooldown-no-cost", cooldown.result_server_sequence)?;
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
    if !visible(&observer, caster_ref) {
        return Err("Global parity: other players must still see the invisible caster".into());
    }
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
    full_e2ef5::with_observer_liveness(
        &mut observer,
        full_e2ef5::approach_melee(&mut caster, world, rat_position),
    )
    .await?;
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
    observer.take_events();

    let session = caster.join_snapshot().game_session_id;
    let position = caster.world_spatial().actor_position;
    let durable_vitals =
        full_e2ef5::with_observer_liveness(&mut observer, logout_after_rat_combat(&mut caster))
            .await?;
    if durable_vitals.0 >= before.mana {
        return Err("C17b spent mana did not persist until logout".into());
    }
    wait_visible(&mut observer, caster_ref, false).await?;
    caster = full_e2ef5::with_observer_liveness(
        &mut observer,
        connect_character(&caster_clients, 0xb3, 0),
    )
    .await?;
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
    let enter_sequence = wait_visible(&mut observer, fresh_ref, true).await?;
    if invisibility_remaining(&mut caster)
        .await?
        .is_none_or(|remaining| remaining <= 0 || remaining >= active)
    {
        return Err("C17b invisibility was absent or reset after relog".into());
    }
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
