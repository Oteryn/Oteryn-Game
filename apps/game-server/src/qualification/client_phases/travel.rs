//! FLOOR-E2E: multi-floor route planning over the production bundle, travelled by real play.
//!
//! Every planned edge is a command a player sends:
//! - a STEP, resolved by the production floor resolver, so stairs, ramps, holes and teleports
//!   change floor by stepping;
//! - a STEP onto a closed door, after USE on that door (level and quest doors answer `Locked`
//!   below their predicate, which ends the journey with that exact refusal);
//! - USE on an immovable ladder, and USE of a carried rope on a rope spot. Both land one floor up
//!   on the first free tile in the server's order (`crates/gameplay/world/src/map/floor/rope.rs`,
//!   `OFFSETS`).
//!
//! - an NPC ship or carpet passage, as a player buys it: walk next to the captain, say "hi", the
//!   destination keyword and "yes" through the NPC dialogue protocol (`npc_shop::talk`), pay the
//!   fare and arrive at the destination. Captains, keywords, fares and destinations come from the
//!   bundle's NPC placements joined with `travel_routes.json`, the content the server answers from.
//!   A passage is one planner edge priced `TRAVEL_COST` plus its fare. A refusal (no money, no
//!   premium, no such route) ends the journey with the NPC's own line.
//!
//! A refused door USE (a level door below its level, a quest or key door) bans that door and
//! replans; only when no other route remains does the journey fail, naming every refusal.
//!
//! Levers: no bundle fact maps a lever to the passage its quest callback opens, and the row's
//! world copy never sees that callback, so a lever cannot be discovered by the search. A scenario
//! that knows its lever from the quest source calls `pull_lever` with the tiles it opens.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, EntityRef, SessionEvent};
use oteryn_protocol_oteryn::npc_service as service;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

const DIRECTIONS: [StepDirection; 4] = [
    StepDirection::North,
    StepDirection::East,
    StepDirection::South,
    StepDirection::West,
];
/// `rope.rs` `OFFSETS`: south, north, east, west, then the diagonals.
const OFFSETS: [(i16, i16); 8] = [
    (0, 1),
    (0, -1),
    (1, 0),
    (-1, 0),
    (-1, 1),
    (1, 1),
    (-1, -1),
    (1, -1),
];
/// `rope.rs` `ROPE_GROUND` (canary data/global.lua:83-84) and its top-item rope spot 12935.
const ROPE_GROUND: &[u16] = &[
    386, 421, 7762, 12202, 12936, 14238, 17238, 21965, 21966, 21967, 21968, 23363,
];
const ROPE_TOP: u16 = 12935;
const ROPE_ITEM: u32 = 3003;
/// Tiles around the start-to-target box that a detour may use.
const MARGIN: u16 = 160;
/// The search gives up after this many settled tiles instead of exhausting memory.
const SEARCH_LIMIT: usize = 2_000_000;
const MAX_REPLANS: usize = 12;
const STEP_COST: u32 = 10;
/// A closed door costs a USE and may be refused, so an open way round is preferred.
const DOOR_COST: u32 = 40;
const CLIMB_COST: u32 = 20;
/// A passage costs the conversation and the wait, plus its fare in gold.
const TRAVEL_COST: u32 = 300;
/// Talk range of the planned conversation, in tiles (Chebyshev) from the captain's placement.
const TALK_RANGE: i16 = 2;
/// Offers further than this from the target are not allowed to widen the search box.
const OFFER_REACH: u16 = 1500;

/// One NPC passage: the captain's placement, what to say, what it costs and where it lands.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Offer {
    npc: String,
    at: TilePos,
    keyword: String,
    price: u64,
    premium: bool,
    dest: TilePos,
}

/// Every unheld route of `travel_routes.json` whose captain stands in the bundle. Leaked: the
/// planner lives for the whole run.
fn offers(npcs: &oteryn_world_bundle::npc::Table) -> TestResult<Vec<&'static Offer>> {
    let routes: Vec<serde_json::Value> =
        serde_json::from_str(crate::content::npc_catalogue::travel::revision_material())?;
    let mut found = Vec::new();
    for npc in &npcs.npcs {
        let prefix = format!(
            "oteryn:service.travel.{}/",
            npc.key.trim_start_matches("oteryn:npc.")
        );
        for route in routes
            .iter()
            .filter(|route| {
                route["key"]
                    .as_str()
                    .is_some_and(|key| key.starts_with(&prefix))
            })
            .filter(|route| route["held"].as_bool() == Some(false))
        {
            let coordinate = |name: &str| route[name].as_u64().and_then(|v| u16::try_from(v).ok());
            let (Some(keyword), Some(price), Some(x), Some(y), Some(z)) = (
                route["keyword"].as_str(),
                route["price"].as_u64(),
                coordinate("x"),
                coordinate("y"),
                route["floor"].as_i64().and_then(|v| i8::try_from(v).ok()),
            ) else {
                continue;
            };
            for place in &npc.placements {
                found.push(&*Box::leak(Box::new(Offer {
                    npc: npc.key.clone(),
                    at: TilePos {
                        x: place.x,
                        y: place.y,
                        floor: place.floor,
                    },
                    keyword: keyword.to_ascii_lowercase(),
                    price,
                    premium: route["premium"].as_bool().unwrap_or(false),
                    dest: TilePos { x, y, floor: -z },
                })));
            }
        }
    }
    Ok(found)
}

/// One player action of a route.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Leg {
    Step(StepDirection),
    Ladder { at: TilePos, ordinal: u8 },
    Rope { at: TilePos, ordinal: u8 },
    Travel(&'static Offer),
}

/// Passage state learned during play: tiles a pulled lever opened, doors whose USE was refused.
#[derive(Default)]
pub(super) struct Passages {
    pub(super) open: HashSet<TilePos>,
    pub(super) refused: HashSet<TilePos>,
}

/// What one route action did.
enum Acted {
    At(TilePos),
    DoorRefused(TilePos, String),
}

pub(super) struct Planner<'a> {
    world: &'a BundleWorld,
    /// Door tile -> (placement key, authored action id).
    doors: HashMap<TilePos, (u64, u16)>,
    ladders: HashMap<TilePos, u8>,
    ropes: HashMap<TilePos, u8>,
    offers: Vec<&'static Offer>,
    /// Captain placement -> the passages he sells.
    offer_at: HashMap<TilePos, Vec<&'static Offer>>,
    /// Authored `unique` attribute -> its tile and top-level ordinal.
    pub(super) uniques: HashMap<u16, (TilePos, u8)>,
}

impl<'a> Planner<'a> {
    /// Door facts only; `scan` adds the bundle's ladders, rope spots and unique ids.
    pub(super) fn new(world: &'a BundleWorld) -> Self {
        let mut doors = HashMap::new();
        for key in world.facts().doors().keys() {
            if let Some((at, _)) = crate::map::view::placement_of(key) {
                doors.entry(at).or_insert((key, 0));
            }
        }
        Self {
            world,
            doors,
            ladders: HashMap::new(),
            ropes: HashMap::new(),
            offers: Vec::new(),
            offer_at: HashMap::new(),
            uniques: HashMap::new(),
        }
    }

    pub(super) fn load(world: &'a BundleWorld, bytes: &[u8]) -> TestResult<Self> {
        let mut planner = Self::new(world);
        let Self {
            doors,
            ladders,
            ropes,
            uniques,
            ..
        } = &mut planner;
        let visited = oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
            for tile in sector.tiles {
                let at = TilePos {
                    x: tile.x,
                    y: tile.y,
                    floor: sector.floor,
                };
                for (ordinal, item) in tile.items.iter().filter(|item| item.depth == 0).enumerate()
                {
                    let Ok(ordinal) = u8::try_from(ordinal) else {
                        continue;
                    };
                    if let Some(unique) = item.attrs.unique {
                        uniques.entry(unique).or_insert((at, ordinal));
                    }
                    if let Some(door) = doors.get_mut(&at)
                        && crate::map::view::placement_key(at, ordinal) == Some(door.0)
                    {
                        door.1 = item.attrs.action.unwrap_or(0);
                    }
                    let entry = &manifest.palette[item.palette as usize];
                    let Some(id) = crate::map::palette_appearance(&entry.key) else {
                        continue;
                    };
                    if crate::map::floor::is_ladder_appearance(id) {
                        ladders.entry(at).or_insert(ordinal);
                    }
                    let ground = entry.terrain.is_some_and(|terrain| {
                        terrain.kind == oteryn_world_bundle::bundle::TerrainKind::Ground
                    });
                    if (ground && ROPE_GROUND.contains(&id)) || (!ground && id == ROPE_TOP) {
                        ropes.entry(at).or_insert(ordinal);
                    }
                }
            }
            Ok(())
        })?;
        if visited.digest != world.base().digest() {
            return Err("travel planner and boot world have different bundle digests".into());
        }
        planner.offers = offers(&visited.npcs)?;
        for &offer in &planner.offers {
            planner.offer_at.entry(offer.at).or_default().push(offer);
        }
        Ok(planner)
    }

    fn walkable(&self, at: TilePos, through_doors: bool, passages: &Passages) -> bool {
        self.world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.walkable())
            && (self.world.enterable(at)
                || passages.open.contains(&at)
                || (through_doors
                    && self.doors.contains_key(&at)
                    && !passages.refused.contains(&at)))
    }

    /// The production floor resolver, with every door tile enterable: the walker opens a closed
    /// door with USE before it steps, as a player does.
    pub(super) fn step(
        &self,
        from: TilePos,
        direction: StepDirection,
        passages: &Passages,
    ) -> Option<TilePos> {
        use crate::map::floor::{Ground, resolve_landing};
        use crate::movement::CardinalStep;
        let direction = match direction {
            StepDirection::North => CardinalStep::North,
            StepDirection::East => CardinalStep::East,
            StepDirection::South => CardinalStep::South,
            StepDirection::West => CardinalStep::West,
            StepDirection::NorthEast => CardinalStep::NorthEast,
            StepDirection::SouthEast => CardinalStep::SouthEast,
            StepDirection::SouthWest => CardinalStep::SouthWest,
            StepDirection::NorthWest => CardinalStep::NorthWest,
        };
        let position = |(x, y, z): (i32, i32, i32)| {
            Some(TilePos {
                x: u16::try_from(x).ok()?,
                y: u16::try_from(y).ok()?,
                floor: -i8::try_from(z).ok().filter(|z| (0..=15).contains(z))?,
            })
        };
        let ground = |cell| match position(cell) {
            Some(pos) if self.walkable(pos, true, passages) => Ground::Walkable,
            Some(pos) if self.world.base().tile(pos.x, pos.y, pos.floor).is_some() => {
                Ground::Blocked
            }
            _ => Ground::Absent,
        };
        let next = resolve_landing(
            self.world.floors(),
            ground,
            |_| true,
            (i32::from(from.x), i32::from(from.y), -i32::from(from.floor)),
            direction,
        )
        .ok()?;
        position(next)
    }

    /// Ladder and rope landing: one floor up, the first walkable tile in `OFFSETS` order.
    fn climb(&self, spot: TilePos, passages: &Passages) -> Option<TilePos> {
        first_landing(spot, |at| self.walkable(at, false, passages))
    }

    fn edges(
        &self,
        at: TilePos,
        rope: bool,
        passages: &Passages,
        out: &mut Vec<(Leg, TilePos, u32)>,
    ) {
        for direction in DIRECTIONS {
            if let Some(next) = self.step(at, direction, passages) {
                let closed = neighbour(at, direction).is_some_and(|entered| {
                    self.doors.contains_key(&entered) && !self.world.enterable(entered)
                });
                let cost = if closed { DOOR_COST } else { STEP_COST };
                out.push((Leg::Step(direction), next, cost));
            }
        }
        if !self.offer_at.is_empty() {
            for dy in -TALK_RANGE..=TALK_RANGE {
                for dx in -TALK_RANGE..=TALK_RANGE {
                    let (Some(x), Some(y)) =
                        (at.x.checked_add_signed(dx), at.y.checked_add_signed(dy))
                    else {
                        continue;
                    };
                    for &offer in self
                        .offer_at
                        .get(&TilePos { x, y, ..at })
                        .into_iter()
                        .flatten()
                    {
                        let fare = u32::try_from(offer.price).unwrap_or(u32::MAX / 2);
                        out.push((
                            Leg::Travel(offer),
                            offer.dest,
                            TRAVEL_COST.saturating_add(fare),
                        ));
                    }
                }
            }
        }
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (Some(x), Some(y)) = (at.x.checked_add_signed(dx), at.y.checked_add_signed(dy))
                else {
                    continue;
                };
                let spot = TilePos { x, y, ..at };
                if let Some(&ordinal) = self.ladders.get(&spot)
                    && let Some(next) = self.climb(spot, passages)
                {
                    out.push((Leg::Ladder { at: spot, ordinal }, next, CLIMB_COST));
                }
                if rope
                    && let Some(&ordinal) = self.ropes.get(&spot)
                    && let Some(next) = self.climb(spot, passages)
                {
                    out.push((Leg::Rope { at: spot, ordinal }, next, CLIMB_COST));
                }
            }
        }
    }

    /// A route of player actions from `from` to a tile satisfying `goal`, searched inside the
    /// start-to-`target` box plus `MARGIN`. Weighted toward `target`, so not always the shortest.
    /// The error names why the search ended and the tile nearest `target` it reached.
    pub(super) fn plan(
        &self,
        from: TilePos,
        target: TilePos,
        goal: &dyn Fn(TilePos) -> bool,
        rope: bool,
        passages: &Passages,
    ) -> Result<Vec<(Leg, TilePos)>, String> {
        // The box also covers every captain (and his landing) near the target, so a passage
        // whose port lies outside the plain start-to-target box can still be walked to.
        let mut points = vec![from, target];
        for offer in &self.offers {
            let gap = |at: TilePos| at.x.abs_diff(target.x).max(at.y.abs_diff(target.y));
            if gap(offer.at) <= OFFER_REACH && gap(offer.dest) < gap(from) {
                points.extend([offer.at, offer.dest]);
            }
        }
        let span = |value: fn(&TilePos) -> u16| {
            let low = points.iter().map(value).min().unwrap_or(0);
            let high = points.iter().map(value).max().unwrap_or(0);
            (low.saturating_sub(MARGIN), high.saturating_add(MARGIN))
        };
        let (low_x, high_x) = span(|at| at.x);
        let (low_y, high_y) = span(|at| at.y);
        search(
            from,
            goal,
            |at| {
                10 * (u32::from(at.x.abs_diff(target.x))
                    + u32::from(at.y.abs_diff(target.y))
                    + u32::from(at.floor.abs_diff(target.floor)))
            },
            |at, out| {
                self.edges(at, rope, passages, out);
                out.retain(|(_, next, _)| {
                    (low_x..=high_x).contains(&next.x) && (low_y..=high_y).contains(&next.y)
                });
            },
            SEARCH_LIMIT,
        )
    }

    fn door(&self, at: TilePos) -> Option<(u64, u16)> {
        self.doors.get(&at).copied()
    }

    /// Doors of one authored action (1000 + level for a level door) on `near`'s floor within `reach` tiles of it.
    pub(super) fn level_doors(&self, near: TilePos, reach: u16, wanted: u16) -> Vec<TilePos> {
        let mut doors: Vec<_> = self
            .doors
            .iter()
            .filter(|(at, (_, action))| {
                *action == wanted
                    && at.floor == near.floor
                    && at.x.abs_diff(near.x).max(at.y.abs_diff(near.y)) <= reach
            })
            .map(|(at, _)| *at)
            .collect();
        doors.sort_by_key(|at| (at.x, at.y));
        doors
    }

    /// The door's placement bytes for a USE, as `use_map_object` takes them.
    pub(super) fn door_placement(&self, at: TilePos) -> TestResult<Vec<u8>> {
        let (key, _) = self
            .door(at)
            .ok_or_else(|| format!("no door at {}", show(at)))?;
        let mut placement = self.world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        Ok(placement)
    }

    /// Evidence: every passage landing within `reach` tiles of `target`, with captain and fare.
    pub(super) fn offers_near(&self, target: TilePos, reach: u16) -> Vec<String> {
        let near = self
            .offers
            .iter()
            .filter(|offer| {
                offer
                    .dest
                    .x
                    .abs_diff(target.x)
                    .max(offer.dest.y.abs_diff(target.y))
                    <= reach
            })
            .map(|offer| describe_offer(offer))
            .collect::<Vec<_>>();
        if near.is_empty() {
            vec![format!(
                "no placed captain lands within {reach} tiles of {} ({} passages of placed captains in all)",
                show(target),
                self.offers.len()
            )]
        } else {
            near
        }
    }

    /// Evidence lines naming every walk run, door, floor change, ladder and rope of `route`.
    pub(super) fn describe(&self, from: TilePos, route: &[(Leg, TilePos)]) -> Vec<String> {
        describe(from, route, |at| self.door(at).map(|(_, action)| action))
    }

    pub(super) fn placement(&self, at: TilePos, ordinal: u8) -> TestResult<Vec<u8>> {
        let key = crate::map::view::placement_key(at, ordinal)
            .ok_or_else(|| format!("no placement key for {} #{ordinal}", show(at)))?;
        let mut placement = self.world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        Ok(placement)
    }
}

/// The first tile of `OFFSETS` around `spot`, one floor up, that `free` accepts.
fn first_landing(spot: TilePos, free: impl Fn(TilePos) -> bool) -> Option<TilePos> {
    let floor = spot.floor.checked_add(1).filter(|floor| *floor <= 0)?;
    OFFSETS.iter().find_map(|&(dx, dy)| {
        let at = TilePos {
            x: spot.x.checked_add_signed(dx)?,
            y: spot.y.checked_add_signed(dy)?,
            floor,
        };
        free(at).then_some(at)
    })
}

/// Weighted A* over player actions, each with its own cost.
fn search<E: Copy>(
    from: TilePos,
    goal: &dyn Fn(TilePos) -> bool,
    estimate: impl Fn(TilePos) -> u32,
    mut edges: impl FnMut(TilePos, &mut Vec<(E, TilePos, u32)>),
    limit: usize,
) -> Result<Vec<(E, TilePos)>, String> {
    let mut open = BinaryHeap::from([Reverse((estimate(from), 0u32, from))]);
    let mut costs = HashMap::from([(from, 0u32)]);
    let mut parents: HashMap<TilePos, (TilePos, E)> = HashMap::new();
    let mut buffer = Vec::new();
    let mut closest = (estimate(from), from);
    let mut settled = 0usize;
    while let Some(Reverse((_, cost, at))) = open.pop() {
        if costs.get(&at) != Some(&cost) {
            continue;
        }
        settled += 1;
        closest = closest.min((estimate(at), at));
        if goal(at) {
            let mut path = Vec::new();
            let mut cursor = at;
            while cursor != from {
                let &(previous, edge) = parents
                    .get(&cursor)
                    .ok_or("route reconstruction lost a parent")?;
                path.push((edge, cursor));
                cursor = previous;
            }
            path.reverse();
            return Ok(path);
        }
        if costs.len() > limit {
            return Err(format!(
                "search limit of {limit} tiles hit after {settled} settled; closest tile reached {}",
                show(closest.1)
            ));
        }
        buffer.clear();
        edges(at, &mut buffer);
        for &(edge, next, step) in &buffer {
            let candidate = cost.saturating_add(step);
            if costs.get(&next).is_none_or(|&old| candidate < old) {
                costs.insert(next, candidate);
                parents.insert(next, (at, edge));
                open.push(Reverse((
                    candidate.saturating_add(estimate(next)),
                    candidate,
                    next,
                )));
            }
        }
    }
    Err(format!(
        "every reachable tile of the search box exhausted ({settled} settled); closest tile reached {}",
        show(closest.1)
    ))
}

fn describe_offer(offer: &Offer) -> String {
    format!(
        "{}@{} says '{}' for {} gold{} -> {}",
        offer.npc,
        show(offer.at),
        offer.keyword,
        offer.price,
        if offer.premium { " (premium)" } else { "" },
        show(offer.dest)
    )
}

fn show(at: TilePos) -> String {
    format!("{},{},{}", at.x, at.y, -i16::from(at.floor))
}

fn neighbour(at: TilePos, direction: StepDirection) -> Option<TilePos> {
    let (dx, dy) = match direction {
        StepDirection::North => (0, -1),
        StepDirection::East => (1, 0),
        StepDirection::South => (0, 1),
        StepDirection::West => (-1, 0),
        StepDirection::NorthEast => (1, -1),
        StepDirection::SouthEast => (1, 1),
        StepDirection::SouthWest => (-1, 1),
        StepDirection::NorthWest => (-1, -1),
    };
    Some(TilePos {
        x: at.x.checked_add_signed(dx)?,
        y: at.y.checked_add_signed(dy)?,
        floor: at.floor,
    })
}

fn describe(
    from: TilePos,
    route: &[(Leg, TilePos)],
    door: impl Fn(TilePos) -> Option<u16>,
) -> Vec<String> {
    let mut segments = Vec::new();
    let mut run = 0usize;
    let mut at = from;
    for &(leg, landed) in route {
        let segment = match leg {
            Leg::Step(direction) => {
                let entered = neighbour(at, direction);
                match entered.and_then(|entered| door(entered).map(|action| (entered, action))) {
                    Some((entered, action)) => {
                        Some(format!("door@{}(action={action})", show(entered)))
                    }
                    None if landed.floor != at.floor => {
                        Some(format!("floor-step@{}->{}", show(at), show(landed)))
                    }
                    None => None,
                }
            }
            Leg::Ladder { at: spot, .. } => {
                Some(format!("ladder-USE@{}->{}", show(spot), show(landed)))
            }
            Leg::Rope { at: spot, .. } => {
                Some(format!("rope-USE@{}->{}", show(spot), show(landed)))
            }
            Leg::Travel(offer) => Some(format!(
                "passage[cost={}]({})",
                TRAVEL_COST.saturating_add(u32::try_from(offer.price).unwrap_or(0)),
                describe_offer(offer)
            )),
        };
        match segment {
            Some(segment) => {
                if run != 0 {
                    segments.push(format!("walk({run})->{}", show(at)));
                    run = 0;
                }
                segments.push(segment);
            }
            None => run += 1,
        }
        at = landed;
    }
    if run != 0 {
        segments.push(format!("walk({run})->{}", show(at)));
    }
    segments.chunks(24).map(|chunk| chunk.join(" ")).collect()
}

fn overlay_revision(client: &DevClientSession, placement: &[u8]) -> u64 {
    client
        .world_object_overlay()
        .iter()
        .find(|entry| entry.placement == placement)
        .map_or(0, |entry| entry.revision)
}

/// USE on a map object at the revision this client last saw, as a player's click sends it.
/// An `Exhausted` answer (the stair-hop cooldown) is retried a bounded number of times.
pub(super) async fn use_map_object(
    client: &mut DevClientSession,
    placement: &[u8],
) -> TestResult<UseDisposition> {
    let mut disposition = UseDisposition::Exhausted;
    for _ in 0..10 {
        let revision = overlay_revision(client, placement);
        disposition = client.use_object(placement, revision).await?.disposition;
        if disposition != UseDisposition::Exhausted {
            break;
        }
        client.service_liveness(Duration::from_millis(300)).await?;
        client.take_events();
    }
    Ok(disposition)
}

/// One STEP with the town walker's bounded retry of a stationary `Rejected` (pacing).
async fn paced_step(
    client: &mut DevClientSession,
    direction: StepDirection,
) -> TestResult<(StepDisposition, String)> {
    let before = client.world_spatial().actor_position;
    let mut outcome = client.step_retrying(direction).await?;
    for attempt in 1..=6 {
        let Some(delay) = town_route_retry_delay(
            outcome.disposition,
            client.world_spatial().actor_position == before,
            attempt,
        ) else {
            break;
        };
        tokio::time::sleep(delay).await;
        outcome = client.step_retrying(direction).await?;
    }
    // A `Moved` reply without a published position change is checked by the caller's landing.
    Ok((outcome.disposition, format!("{outcome:?}")))
}

impl Planner<'_> {
    async fn execute(
        &self,
        client: &mut DevClientSession,
        label: &str,
        at: TilePos,
        leg: Leg,
    ) -> TestResult<Acted> {
        match leg {
            Leg::Step(direction) => {
                let (mut disposition, mut detail) = paced_step(client, direction).await?;
                let door = neighbour(at, direction)
                    .and_then(|entered| self.door(entered).map(|door| (entered, door)));
                if disposition == StepDisposition::Blocked
                    && let Some((entered, (key, action))) = door
                {
                    let mut placement = self.world.base().digest().to_vec();
                    placement.extend_from_slice(&key.to_be_bytes());
                    let reply = use_map_object(client, &placement).await?;
                    evidence(&format!(
                        "TRAVEL_DOOR {label} door={} action={action} use={reply:?}",
                        show(entered)
                    ));
                    if reply != UseDisposition::Committed {
                        let level = (action > 1000)
                            .then(|| format!(" (level door, requires level {})", action - 1000))
                            .unwrap_or_default();
                        return Ok(Acted::DoorRefused(
                            entered,
                            format!(
                                "door {} action={action} USE refused: {reply:?}{level}",
                                show(entered)
                            ),
                        ));
                    }
                    (disposition, detail) = paced_step(client, direction).await?;
                }
                if disposition != StepDisposition::Moved {
                    return Err(format!(
                        "{label} STEP {direction:?} from {} refused: {detail}",
                        show(at)
                    )
                    .into());
                }
            }
            Leg::Travel(offer) => travel_by_npc(client, label, offer).await?,
            Leg::Ladder { at: spot, ordinal } | Leg::Rope { at: spot, ordinal } => {
                let kind = if matches!(leg, Leg::Ladder { .. }) {
                    "ladder"
                } else {
                    "rope"
                };
                let reply = if kind == "ladder" {
                    use_map_object(client, &self.placement(spot, ordinal)?).await?
                } else {
                    self.use_rope(client, spot, ordinal).await?
                };
                evidence(&format!(
                    "TRAVEL_CLIMB {label} {kind}={} use={reply:?}",
                    show(spot)
                ));
                if reply != UseDisposition::Committed {
                    return Err(format!(
                        "{label} {kind} USE at {} from {} refused: {reply:?}",
                        show(spot),
                        show(at)
                    )
                    .into());
                }
                let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
                loop {
                    let landed = tile(client.world_spatial().actor_position)?;
                    if landed.floor == spot.floor + 1
                        && landed.x.abs_diff(spot.x).max(landed.y.abs_diff(spot.y)) <= 1
                    {
                        break;
                    }
                    if tokio::time::Instant::now() >= deadline {
                        return Err(format!(
                            "{label} {kind} USE at {} committed but no landing one floor up was published (at {})",
                            show(spot),
                            show(landed)
                        )
                        .into());
                    }
                    client.service_liveness(Duration::from_millis(50)).await?;
                    client.take_events();
                }
            }
        }
        client.take_events();
        verdict::Harness::global().progress("walk");
        Ok(Acted::At(tile(client.world_spatial().actor_position)?))
    }

    async fn use_rope(
        &self,
        client: &mut DevClientSession,
        spot: TilePos,
        ordinal: u8,
    ) -> TestResult<UseDisposition> {
        use oteryn_protocol_oteryn::world_object::*;
        let reference = provisioning::item_reference(&provisioning::item_index()?, ROPE_ITEM)?;
        let source = client
            .inventory()
            .and_then(|inventory| {
                inventory
                    .entries
                    .iter()
                    .find(|item| item.item_definition_ref == reference)
            })
            .ok_or("planned rope is no longer carried")?
            .handle;
        let placement = self.placement(spot, ordinal)?;
        let intent = UseIntent {
            target: UseTarget::Item(source),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                expected_revision: overlay_revision(client, &placement),
                placement,
            })),
            use_with_position: None,
        };
        let payload =
            encode_item_use_intent(&intent).map_err(|error| format!("rope encode: {error:?}"))?;
        let response = client
            .session_mut()
            .gated_command(CAPABILITY_ITEM_USE_V1, COMMAND_TYPE_USE_INTENT, &payload)
            .await?;
        if response.status != CommandStatus::Accepted {
            return Err(format!("rope USE command not accepted: {response:?}").into());
        }
        decode_use_result(&response.payload)
            .map_err(|error| format!("rope USE result: {error:?}").into())
    }
}

/// One line to the NPC through the production dialogue protocol, and the lines it answers.
async fn converse(
    client: &mut DevClientSession,
    npc: EntityRef,
    text: &str,
) -> TestResult<Vec<String>> {
    client.take_events();
    let payload = service::encode_npc_talk_intent(&service::NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(|error| format!("NPC talk '{text}' encode: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            service::CAPABILITY_NPC_SERVICE_V1,
            service::COMMAND_TYPE_NPC_TALK_INTENT,
            &payload,
        )
        .await?;
    let disposition = service::decode_npc_intent_result(&reply.payload)
        .map_err(|error| format!("NPC talk '{text}' reply: {error:?}"))?;
    if reply.status != CommandStatus::Accepted || disposition != service::NpcIntentDisposition::Ok {
        return Err(format!(
            "NPC talk '{text}' status={:?} disposition={disposition:?}",
            reply.status
        )
        .into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
    loop {
        for event in client.take_events() {
            if let SessionEvent::NpcConversation(delta) = event
                && delta.server_sequence > reply.server_sequence
            {
                return Ok(delta.value.lines);
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("NPC talk '{text}' got no conversation delta").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

/// Buy a passage as a player does: say "hi", the destination and "yes" to the captain next to
/// whom the route stopped, then wait for the arrival at the destination port.
async fn travel_by_npc(client: &mut DevClientSession, label: &str, offer: &Offer) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let npc = loop {
        let found = client.world_entities().and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == EntityKind::Npc
                    && entity.position.x == i32::from(offer.at.x)
                    && entity.position.y == i32::from(offer.at.y)
                    && entity.position.floor == -i16::from(offer.at.floor)
            })
        });
        if let Some(entity) = found {
            break entity.entity;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "{label} captain {} is not visible at {}",
                offer.npc,
                show(offer.at)
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    };
    let need = || {
        format!(
            "{label} passage {} needs {} gold{}; the node-boot fixture mints axe, rope and shovel only (qualification_fixture.rs prepare_items)",
            describe_offer(offer),
            offer.price,
            if offer.premium {
                " and a premium account"
            } else {
                ""
            }
        )
    };
    let mut quote = Vec::new();
    for text in ["hi", offer.keyword.as_str()] {
        quote = converse(client, npc, text).await?;
        evidence(&format!("TRAVEL_NPC {label} say={text:?} reply={quote:?}"));
    }
    if !quote.iter().any(|line| line.contains("passage")) {
        return Err(format!("{} refused: {quote:?}", need()).into());
    }
    let reply = converse(client, npc, "yes").await.unwrap_or_default();
    evidence(&format!("TRAVEL_NPC {label} say=\"yes\" reply={reply:?}"));
    if let Some(line) = reply.iter().find(|line| !line.contains("sails")) {
        return Err(format!("{} refused: {line}", need()).into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        let here = tile(client.world_spatial().actor_position)?;
        if here.floor == offer.dest.floor
            && here
                .x
                .abs_diff(offer.dest.x)
                .max(here.y.abs_diff(offer.dest.y))
                <= 4
        {
            evidence(&format!(
                "TRAVEL_ARRIVED {label} {} landed={}",
                describe_offer(offer),
                show(here)
            ));
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "{label} paid passage {} never landed at the destination (at {})",
                describe_offer(offer),
                show(here)
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
}

fn carries(client: &DevClientSession, item: u32) -> TestResult<bool> {
    let reference = provisioning::item_reference(&provisioning::item_index()?, item)?;
    Ok(client.inventory().is_some_and(|inventory| {
        inventory
            .entries
            .iter()
            .any(|entry| entry.item_definition_ref == reference)
    }))
}

/// Reach a tile satisfying `goal` near `target` by real STEP and USE commands only. The route is
/// printed before it is walked. A divergent landing (an occupied ladder landing) replans from
/// where the server put the character; a refused door is banned and the route replanned.
pub(super) async fn travel_to(
    client: &mut DevClientSession,
    planner: &Planner<'_>,
    label: &str,
    target: TilePos,
    goal: &dyn Fn(TilePos) -> bool,
    passages: &mut Passages,
) -> TestResult {
    let rope = carries(client, ROPE_ITEM)?;
    let mut refusals = Vec::new();
    for attempt in 0..=MAX_REPLANS {
        let from = tile(client.world_spatial().actor_position)?;
        if goal(from) {
            return Ok(());
        }
        verdict::Harness::global().progress("walk");
        let route = planner
            .plan(from, target, goal, rope, passages)
            .map_err(|reason| {
                format!(
                    "{label} no route by step, door, stairs, hole, ladder{} from {} to {}: {reason}; refused on the way: [{}]",
                    if rope { " or rope" } else { "" },
                    show(from),
                    show(target),
                    refusals.join("; ")
                )
            })?;
        verdict::Harness::global().progress("walk");
        evidence(&format!(
            "TRAVEL_ROUTE {label} attempt={attempt} from={} to={} actions={}",
            show(from),
            show(target),
            route.len()
        ));
        for line in planner.describe(from, &route) {
            evidence(&format!("TRAVEL_ROUTE {label} {line}"));
        }
        let mut at = from;
        let mut diverged = false;
        for &(leg, expected) in &route {
            match planner.execute(client, label, at, leg).await? {
                Acted::At(landed) if landed == expected => at = landed,
                Acted::At(landed) => {
                    evidence(&format!(
                        "TRAVEL_REPLAN {label} leg={leg:?} expected={} landed={}",
                        show(expected),
                        show(landed)
                    ));
                    diverged = true;
                    break;
                }
                Acted::DoorRefused(door, reason) => {
                    evidence(&format!("TRAVEL_REPLAN {label} {reason}"));
                    refusals.push(reason);
                    passages.refused.insert(door);
                    diverged = true;
                    break;
                }
            }
        }
        if !diverged {
            return Ok(());
        }
    }
    Err(format!(
        "{label} route still diverged after {MAX_REPLANS} replans; refused on the way: [{}]",
        refusals.join("; ")
    )
    .into())
}

/// Walk next to a lever the quest source names, pull it with USE and treat `opens` as passable
/// from then on, as a player who saw the wall move would.
#[expect(
    dead_code,
    reason = "scenario-facing: no current Q01 chest is behind a lever; quest rows with one call it"
)]
pub(super) async fn pull_lever(
    client: &mut DevClientSession,
    planner: &Planner<'_>,
    label: &str,
    (lever, ordinal): (TilePos, u8),
    opens: &[TilePos],
    passages: &mut Passages,
) -> TestResult {
    travel_to(client, planner, label, lever, &beside(lever), passages).await?;
    let reply = use_map_object(client, &planner.placement(lever, ordinal)?).await?;
    evidence(&format!(
        "TRAVEL_LEVER {label} lever={} use={reply:?}",
        show(lever)
    ));
    if reply != UseDisposition::Committed {
        return Err(format!("{label} lever USE at {} refused: {reply:?}", show(lever)).into());
    }
    passages.open.extend(opens.iter().copied());
    Ok(())
}

/// Next to `at` on its floor (Chebyshev distance at most one), where USE reaches an object.
pub(super) fn beside(at: TilePos) -> impl Fn(TilePos) -> bool {
    move |here| here.floor == at.floor && here.x.abs_diff(at.x).max(here.y.abs_diff(at.y)) <= 1
}

#[cfg(test)]
#[path = "travel_tests.rs"]
mod tests;
