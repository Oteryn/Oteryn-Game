//! Play view after admission (N2N3-1): the map around the own actor, one marker per overlay
//! object, and the walk over `Session::step`.
//!
//! The view holds no authority. It is built from the join snapshot, and only a step outcome's
//! domain-1 and domain-2 deltas move it. The map is the start area anchored at the join
//! position (CLIENT-VIS-1 milestone 1) until positions are map positions.

use crate::combat_input::{CombatCommand, CombatReport, PlayCommand, run_combat};
use crate::input::{ClickWalk, StepDeadline, StepDir, StepResult, TargetKind, Targetable};
use crate::map_view::{MapTarget, MapView};
use crate::scene::Scene;
use crate::spell::{SPELL_SLOTS, SpellFeedback};
use crate::world::{START, World};
use oteryn_platform_client::native_login::PublicClass;
use oteryn_renderer::{BatchError, TileCoord};
use oteryn_session::{
    ActorPosition, EntityKind, EntityRef, JoinSnapshot, Session, SessionError, SessionEvent,
    SessionStream, StepDirection, StepDisposition, StepOutcome, StepReceiptOutcome,
    WorldObjectOverlayEntry, WorldSpatialEntity,
};
use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// How long the session task serves liveness probes while no step is pending; well inside the
/// server's probe cadence.
const IDLE_SLICE: Duration = Duration::from_millis(20);

/// Byte length of a bundle placement key (`OTERYN_WORLD_BUNDLE_FORMAT_V1.md` §7), and of the key
/// prefixed by the 32-byte bundle digest.
const KEY_BYTES: usize = 8;
const DIGEST_AND_KEY_BYTES: usize = 32 + KEY_BYTES;

/// Semantic placements the server emits today, with the cell each names. The native entry
/// room's door cell (`native_entry.rs` `DOOR_CELL`); a placeholder until map loading carries
/// cell coordinates to the client.
const SEMANTIC_PLACEMENTS: [(&[u8], i32, i32, i16); 1] = [(b"oteryn:cell/entry-door", 1, -1, 0)];

/// The tile and floor a placement names: a known semantic cell key, or the last eight bytes as
/// the big-endian key `x << 32 | y << 16 | (-floor) << 8 | ordinal`. Other shapes name no tile
/// and draw no marker.
fn placement_tile(placement: &[u8]) -> Option<(TileCoord, i16)> {
    if let Some(&(_, x, y, floor)) = SEMANTIC_PLACEMENTS
        .iter()
        .find(|(name, ..)| *name == placement)
    {
        return Some((TileCoord::new(x, y), floor));
    }
    if placement.len() != KEY_BYTES && placement.len() != DIGEST_AND_KEY_BYTES {
        return None;
    }
    let key = u64::from_be_bytes(placement[placement.len() - KEY_BYTES..].try_into().ok()?);
    let x = i32::try_from(key >> 32).ok()?;
    let y = i32::from(u16::try_from((key >> 16) & 0xffff).ok()?);
    let floor = -i16::from(u8::try_from((key >> 8) & 0xff).ok()?);
    Some((TileCoord::new(x, y), floor))
}

const fn step_direction(direction: StepDir) -> StepDirection {
    match direction {
        StepDir::North => StepDirection::North,
        StepDir::East => StepDirection::East,
        StepDir::South => StepDirection::South,
        StepDir::West => StepDirection::West,
    }
}

/// Any session error ends play and returns to login with this public class.
#[must_use]
pub const fn public_class(_error: &SessionError) -> PublicClass {
    PublicClass::SessionUnavailable
}

/// One input waiting for its turn on the session task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Input {
    Arrow(StepDir),
    Cast(NonZeroU32),
    Attack(EntityRef),
    Stop,
    UseMap(MapTarget),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleTargetSelection {
    First,
    Next,
    Previous,
}

/// Most inputs held while the session task is busy; a later one is dropped.
const MAX_QUEUED_INPUTS: usize = 16;

#[derive(Debug, Clone, Copy)]
struct StepAnimation {
    previous: ActorPosition,
    destination: ActorPosition,
    received_at: Instant,
    duration: Duration,
}

impl StepAnimation {
    fn finished(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.received_at) >= self.duration
    }
    fn offset(&self, now: Instant) -> [i32; 2] {
        let remaining = self
            .duration
            .saturating_sub(now.saturating_duration_since(self.received_at));
        let pixels =
            i32::try_from(32 * remaining.as_nanos() / self.duration.as_nanos()).unwrap_or(0);
        [
            (self.previous.x - self.destination.x) * pixels,
            (self.previous.y - self.destination.y) * pixels,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct PlayView {
    own: TileCoord,
    /// The direction of the latest step sent; the player is drawn facing it.
    facing: StepDir,
    floor: i16,
    /// Every decodable overlay entry with its floor; the current floor selects what is drawn.
    markers: BTreeMap<Vec<u8>, (TileCoord, i16)>,
    world: Arc<World>,
    /// Added to a session position to reach the map position drawn there.
    anchor: TileCoord,
    /// The session floor the map is anchored to; on any other floor no map is drawn.
    map_floor: Option<i16>,
    scene: Scene,
    walk: ClickWalk,
    /// A step sent to the session task whose outcome has not arrived.
    in_flight: bool,
    retry_cancelled: bool,
    paced: bool,
    walking: Option<StepAnimation>,
    /// Arrow, hotkey and click input not yet sent, in the order it was given.
    inputs: VecDeque<Input>,
    /// The other entities the server reports as visible.
    entities: Vec<WorldSpatialEntity>,
    /// Latest complete server-composed map; never reconstructed from local placements.
    map: Option<MapView>,
    map_draw: Arc<Mutex<crate::map_draw::MapDraw>>,
    /// The last combat line the server's answer produced, shown until the next one.
    combat_line: Option<&'static str>,
    /// Preserved when pushed session state rebuilds the scene.
    target_frame: bool,
}

impl PlayView {
    pub fn from_join(snapshot: &JoinSnapshot) -> Result<Self, BatchError> {
        let position = snapshot.world_spatial.actor_position;
        let mut view = Self::new(
            TileCoord::new(position.x, position.y),
            position.floor,
            &snapshot.world_object_overlay,
        )?;
        if let Some(snapshot) = &snapshot.world_map {
            let payload = oteryn_session::map_view::encode_world_map_snapshot(snapshot)
                .map_err(|_| BatchError::InvalidView)?;
            let mut map = MapView::new(snapshot.header.content_generation)
                .map_err(|_| BatchError::InvalidView)?;
            map.replace_snapshot(&payload)
                .map_err(|_| BatchError::InvalidView)?;
            view.map = Some(map);
            view.rebuild()?;
        }
        Ok(view)
    }

    pub fn new(
        own: TileCoord,
        floor: i16,
        overlay: &[WorldObjectOverlayEntry],
    ) -> Result<Self, BatchError> {
        let world = Arc::new(World::builtin()?);
        let anchor = TileCoord::new(0, 0);
        let mut view = Self {
            own,
            facing: StepDir::South,
            floor,
            markers: BTreeMap::new(),
            scene: Scene::centered_on(Arc::clone(&world), None, own, StepDir::South, &[])?,
            world,
            anchor,
            map_floor: None,
            walk: ClickWalk::new(),
            in_flight: false,
            retry_cancelled: false,
            paced: false,
            walking: None,
            inputs: VecDeque::new(),
            entities: Vec::new(),
            map: None,
            map_draw: Arc::new(Mutex::new(crate::map_draw::MapDraw::placeholders())),
            combat_line: None,
            target_frame: true,
        };
        for entry in overlay {
            view.set_marker(entry);
        }
        view.rebuild()?;
        Ok(view)
    }

    /// Draws the other visible entities the join snapshot carried.
    pub fn with_entities(mut self, entities: Vec<WorldSpatialEntity>) -> Result<Self, BatchError> {
        self.entities = entities;
        self.rebuild()?;
        Ok(self)
    }

    #[must_use]
    pub const fn own(&self) -> TileCoord {
        self.own
    }

    #[must_use]
    pub const fn facing(&self) -> StepDir {
        self.facing
    }

    #[must_use]
    pub const fn scene(&self) -> &Scene {
        &self.scene
    }

    pub fn take_page_uploads(&mut self) -> Vec<oteryn_renderer::SpritePageUpload> {
        self.scene.take_page_uploads()
    }

    pub fn reset_page(&mut self) -> Result<(), BatchError> {
        self.scene.reset_page()
    }

    #[must_use]
    pub fn world_map(&self) -> Option<&MapView> {
        self.map.as_ref()
    }

    #[must_use]
    pub const fn floor(&self) -> i16 {
        self.floor
    }

    pub fn set_target_frame(&mut self, enabled: bool) -> Result<(), BatchError> {
        self.target_frame = enabled;
        self.scene.set_target_frame(enabled)
    }

    #[must_use]
    pub fn minimap_image(&self) -> Option<crate::minimap::LoadedMinimap> {
        if self.map.is_some() {
            return None;
        }
        crate::minimap::LoadedMinimap::from_world(&self.world)
    }

    #[must_use]
    pub fn minimap_location(&self) -> Option<crate::minimap::MapLocation> {
        crate::minimap::MapLocation::from_session(
            ActorPosition {
                x: self.own.x,
                y: self.own.y,
                floor: self.floor,
            },
            self.anchor,
            self.map_floor,
        )
    }

    /// Draws `world` with the current own position on the start tile and the current floor on
    /// the start floor.
    pub fn set_world(&mut self, world: Arc<World>) -> Result<(), BatchError> {
        self.world = world;
        if self.map.is_some() {
            self.anchor = TileCoord::new(0, 0);
            self.map_floor = None;
            return self.rebuild();
        }
        self.map_floor = Some(self.floor);
        // Wrapping keeps the offset exact for any join position: `own + anchor` wraps back to START.
        self.anchor = TileCoord::new(
            START.0.wrapping_sub(self.own.x),
            START.1.wrapping_sub(self.own.y),
        );
        self.rebuild()
    }

    /// Changing the provider invalidates atlas residency and all pending old pixels.
    pub fn set_map_draw(
        &mut self,
        draw: Arc<Mutex<crate::map_draw::MapDraw>>,
    ) -> Result<(), BatchError> {
        if Arc::ptr_eq(&draw, &self.map_draw) {
            return Ok(());
        }
        self.map_draw = draw;
        self.scene.streamed = None;
        self.rebuild()
    }

    #[must_use]
    pub fn marker_count(&self) -> usize {
        self.drawn_markers().count()
    }

    fn drawn_markers(&self) -> impl Iterator<Item = TileCoord> + '_ {
        self.markers
            .values()
            .filter(|(_, floor)| *floor == self.floor)
            .map(|(tile, _)| *tile)
    }

    /// A click on `tile`: a drawn object or actor is selected, any other tile becomes the walk
    /// goal.
    pub fn click(&mut self, tile: TileCoord) -> Result<(), BatchError> {
        if let Some(map) = &self.map {
            if let Some(target) = crate::input::map_actor_target(map, &self.entities, tile) {
                self.scene.select_map_actor(target)?;
                if let Some(entity) = target.entity {
                    self.queue(Input::Attack(entity));
                }
                return Ok(());
            }
            if let Some(picked) = crate::input::map_pick(map, tile) {
                self.scene.select_map_tile(tile)?;
                if picked.position.floor == self.floor
                    && map
                        .tile(picked.position)
                        .and_then(|tile| tile.items.last())
                        .is_some_and(|item| {
                            matches!(
                                item.definition,
                                oteryn_session::map_view::MapDefinition::Terrain(_)
                            ) || self
                                .map_draw
                                .lock()
                                .unwrap_or_else(PoisonError::into_inner)
                                .is_ground(item.appearance_id)
                        })
                {
                    self.walk
                        .set_goal(TileCoord::new(picked.position.x, picked.position.y));
                    self.retry_cancelled = false;
                } else if picked.target.is_none() {
                    self.combat_line = Some("That item is out of reach");
                }
                return Ok(());
            }
        }
        match self.scene.select_tile(tile)? {
            None => {
                self.walk.set_goal(tile);
                self.retry_cancelled = false;
            }
            Some(Targetable {
                entity: Some(target),
                ..
            }) => self.queue(Input::Attack(target)),
            Some(_) => {}
        }
        Ok(())
    }

    /// A native USE click names only the highest visible received entry.
    pub fn use_map(&mut self, tile: TileCoord) -> Result<(), BatchError> {
        let Some(picked) = self
            .map
            .as_ref()
            .and_then(|map| crate::input::map_pick(map, tile))
        else {
            return Ok(());
        };
        self.scene.select_map_tile(tile)?;
        if let Some(target) = picked.target {
            self.queue(Input::UseMap(target));
        } else {
            self.combat_line = Some("That item is out of reach");
        }
        Ok(())
    }

    /// Select and attack an admitted, currently visible battle target. Cycling
    /// follows the server projection order and the same attackability boundary
    /// as pointer selection; players, NPCs and object-only entities are not
    /// guessed to be attackable by this client.
    pub fn select_battle_target(
        &mut self,
        selection: BattleTargetSelection,
    ) -> Result<bool, BatchError> {
        let candidates = self
            .entities
            .iter()
            .filter(|entity| {
                entity.position.floor == self.floor
                    && entity.kind == EntityKind::Creature
                    && self.map.as_ref().is_none_or(|map| {
                        crate::input::map_actor_target(
                            map,
                            &self.entities,
                            TileCoord::new(entity.position.x, entity.position.y),
                        )
                        .is_some_and(|target| target.entity == Some(entity.entity))
                    })
                    && self
                        .scene
                        .view()
                        .contains(TileCoord::new(entity.position.x, entity.position.y))
            })
            .map(|entity| {
                (
                    entity.entity,
                    TileCoord::new(entity.position.x, entity.position.y),
                )
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Ok(false);
        }
        let current = self
            .scene
            .target()
            .and_then(|target| target.entity)
            .and_then(|target| {
                candidates
                    .iter()
                    .position(|(candidate, _)| *candidate == target)
            });
        let index = match selection {
            BattleTargetSelection::First => 0,
            BattleTargetSelection::Next => {
                current.map_or(0, |index| (index + 1) % candidates.len())
            }
            BattleTargetSelection::Previous => current.map_or(candidates.len() - 1, |index| {
                index.checked_sub(1).unwrap_or(candidates.len() - 1)
            }),
        };
        let (entity, tile) = candidates[index];
        self.scene.select_tile(tile)?;
        self.queue(Input::Attack(entity));
        Ok(true)
    }

    /// Drop queued local actions and enqueue the protocol's explicit stop-attack
    /// intent. An already-sent step can only be resolved by the server.
    pub fn stop_all(&mut self) {
        self.retry_cancelled = true;
        self.inputs.clear();
        self.walk.cancel();
        self.queue(Input::Stop);
    }

    /// A spell hotkey: casts the 1-based spell-book entry. Entries past the bar are ignored and
    /// at most one press per bar slot waits for the next tick.
    pub fn cast(&mut self, spell: NonZeroU32) {
        if spell.get() <= u32::from(SPELL_SLOTS) {
            self.queue(Input::Cast(spell));
        }
    }

    /// Appends `input` unless the same one already waits (a repeated press of a slot or click of
    /// a target is one command) or the queue is full.
    fn queue(&mut self, input: Input) {
        if self.inputs.len() < MAX_QUEUED_INPUTS && !self.inputs.contains(&input) {
            self.inputs.push_back(input);
        }
    }

    /// The minimal spell bar: its hotkeys and the last combat or cast line.
    #[must_use]
    pub fn combat_status(&self) -> String {
        let line = self.combat_line.unwrap_or("");
        format!("spells 1-{SPELL_SLOTS} {line}")
            .trim_end()
            .to_owned()
    }

    /// Shows the server's answer to a combat command.
    pub fn apply_combat(&mut self, report: CombatReport) {
        self.combat_line = report.text();
    }

    /// An arrow key: the latest press wins over an arrow still waiting at the end of the queue.
    pub fn arrow(&mut self, direction: StepDir) {
        self.retry_cancelled = false;
        if self.paced {
            self.walk.cancel();
            self.inputs
                .retain(|input| !matches!(input, Input::Arrow(_)));
        } else if matches!(self.inputs.back(), Some(Input::Arrow(_))) {
            self.inputs.pop_back();
        }
        self.queue(Input::Arrow(direction));
    }

    /// The step to send now: an arrow key first, else the next step toward the click goal.
    pub fn next_step(&mut self) -> Option<StepDir> {
        if self.in_flight {
            return None;
        }
        let direction = self.pending_step();
        self.in_flight = direction.is_some();
        if let Some(facing) = direction {
            self.facing = facing;
        }
        direction
    }

    fn pending_step(&mut self) -> Option<StepDir> {
        match self.inputs.front() {
            Some(&Input::Arrow(direction)) => {
                self.inputs.pop_front();
                return Some(direction);
            }
            // A hotkey or click goes out first, in `tick`.
            Some(_) => return None,
            None => {}
        }
        self.walk.next_step(self.own)
    }

    /// Applies one step outcome: the domain-1 delta moves the own actor, the domain-2 delta
    /// moves a marker. A refused step ends the click walk.
    pub fn apply(&mut self, outcome: &StepOutcome) -> Result<(), BatchError> {
        self.retry_cancelled = false;
        self.in_flight = false;
        self.walk
            .on_result(if outcome.disposition == StepDisposition::Moved {
                StepResult::Moved
            } else {
                StepResult::Refused
            });
        if let Some(delta) = &outcome.world_spatial_delta {
            self.place(delta.value.actor_position);
        }
        if let Some(delta) = &outcome.world_object_overlay_delta {
            self.set_marker(&delta.value);
        }
        self.rebuild()
    }

    fn apply_receipt(
        &mut self,
        result: &StepReceiptOutcome,
        now: Instant,
    ) -> Result<(), BatchError> {
        let cancelled = std::mem::take(&mut self.retry_cancelled);
        self.paced |= result.receipt.is_some();
        if self.paced {
            let last = self
                .inputs
                .iter()
                .rposition(|input| matches!(input, Input::Arrow(_)));
            let mut index = 0;
            self.inputs.retain(|input| {
                let keep = !matches!(input, Input::Arrow(_)) || Some(index) == last;
                index += 1;
                keep
            });
        }
        if result.receipt.is_some() && result.disposition == StepDisposition::TooEarly {
            self.in_flight = false;
            self.walk.on_result(StepResult::Retry);
            if !cancelled
                && self.walk.goal().is_none()
                && !self
                    .inputs
                    .iter()
                    .any(|input| matches!(input, Input::Arrow(_)))
            {
                self.arrow(self.facing);
            }
            return self.rebuild_at(now);
        }
        let previous = ActorPosition {
            x: self.own.x,
            y: self.own.y,
            floor: self.floor,
        };
        self.walking = None;
        self.apply(&result.outcome)?;
        if let Some(receipt) = result.receipt
            && result.disposition == StepDisposition::Moved
            && let Some(delta) = &result.world_spatial_delta
        {
            let destination = delta.value.actor_position;
            if previous.floor == destination.floor
                && u64::from(previous.x.abs_diff(destination.x))
                    + u64::from(previous.y.abs_diff(destination.y))
                    == 1
            {
                self.walking = Some(StepAnimation {
                    previous,
                    destination,
                    received_at: receipt.received_at,
                    duration: Duration::from_millis(u64::from(receipt.wait_ms)),
                });
            }
        }
        self.rebuild_at(now)
    }

    /// Applies what the server pushed outside any step, coalesced: the latest own position and
    /// the latest entry per placement. Other domains are not drawn by this view.
    pub fn apply_pushed(&mut self, pushed: &Pushed) -> Result<(), BatchError> {
        if let Some(position) = pushed.own {
            self.place(position);
        }
        if let Some(entities) = &pushed.entities {
            self.entities.clone_from(entities);
        }
        if let Some(map) = &pushed.map {
            self.map = Some((**map).clone());
        }
        for entry in &pushed.markers {
            self.set_marker(entry);
        }
        self.rebuild()
    }

    fn place(&mut self, position: ActorPosition) {
        if self.own != TileCoord::new(position.x, position.y) || self.floor != position.floor {
            self.walking = None;
        }
        self.own = TileCoord::new(position.x, position.y);
        self.floor = position.floor;
    }

    fn set_marker(&mut self, entry: &WorldObjectOverlayEntry) {
        match placement_tile(&entry.placement) {
            Some(placed) => {
                self.markers.insert(entry.placement.clone(), placed);
            }
            None => {
                self.markers.remove(&entry.placement);
            }
        }
    }

    fn rebuild(&mut self) -> Result<(), BatchError> {
        self.rebuild_at(Instant::now())
    }

    fn rebuild_at(&mut self, now: Instant) -> Result<(), BatchError> {
        if let Some(map) = &self.map {
            let header = map.header().ok_or(BatchError::InvalidView)?;
            let drawable = |position: ActorPosition| {
                position.floor <= header.first_visible_floor
                    && oteryn_session::map_view::in_window(header.origin, position)
            };
            let entities = self
                .entities
                .iter()
                .filter(|entity| {
                    drawable(entity.position)
                        && matches!(
                            entity.kind,
                            EntityKind::Player | EntityKind::Creature | EntityKind::Npc
                        )
                })
                .collect::<Vec<_>>();
            let visible = entities
                .iter()
                .filter_map(|entity| {
                    Some(Targetable {
                        tile: crate::map_draw::projected(entity.position, header.origin)?,
                        kind: TargetKind::Entity,
                        glyph: true,
                        entity: (entity.kind == EntityKind::Creature).then_some(entity.entity),
                    })
                })
                .collect::<Vec<_>>();
            let own = ActorPosition {
                x: self.own.x,
                y: self.own.y,
                floor: self.floor,
            };
            let actors = entities
                .into_iter()
                .map(|entity| (entity.position, false))
                .chain([(own, true)])
                .map(|(position, own_actor)| crate::map_draw::MapActor {
                    position,
                    previous: self
                        .walking
                        .filter(|animation| own_actor && !animation.finished(now))
                        .map(|animation| animation.previous),
                    offset: self
                        .walking
                        .filter(|animation| own_actor && !animation.finished(now))
                        .map_or([0, 0], |animation| animation.offset(now)),
                    cells: Vec::new(),
                })
                .collect();
            let mut scene = Scene::centered_with(
                Arc::clone(&self.world),
                None,
                self.own,
                self.facing,
                &[],
                &visible,
            )?;
            scene.set_target_frame(self.target_frame)?;
            scene.streamed = Some(crate::streamed_scene::StreamedScene::compose(
                map,
                actors,
                Arc::clone(&self.map_draw),
                self.scene.streamed.take(),
                scene.view(),
                None,
            )?);
            self.scene = scene;
            return Ok(());
        }
        let markers = self.drawn_markers().collect::<Vec<_>>();
        let entities = self
            .entities
            .iter()
            .filter(|entity| entity.position.floor == self.floor)
            .map(|entity| Targetable {
                tile: TileCoord::new(entity.position.x, entity.position.y),
                // Only a creature may be attacked; NPCs and the rest are drawn and inspected.
                kind: match entity.kind {
                    EntityKind::Corpse | EntityKind::GroundItem => TargetKind::Object,
                    _ => TargetKind::Entity,
                },
                glyph: true,
                entity: matches!(entity.kind, EntityKind::Creature).then_some(entity.entity),
            })
            .collect::<Vec<_>>();
        let mut scene = Scene::centered_with(
            Arc::clone(&self.world),
            (self.map_floor == Some(self.floor)).then_some(self.anchor),
            self.own,
            self.facing,
            &markers,
            &entities,
        )?;
        scene.set_target_frame(self.target_frame)?;
        self.scene = scene;
        Ok(())
    }

    /// One shell tick, never waiting on the network: applies every outcome the session task has
    /// delivered, then hands it the next step if one is due. A session end returns to login with
    /// its public class; a view error is a render failure.
    pub fn tick(&mut self, link: &PlayLink) -> Result<(), PublicClass> {
        self.tick_at(link, Instant::now())
    }

    fn tick_at(&mut self, link: &PlayLink, now: Instant) -> Result<(), PublicClass> {
        while let Some(event) = link.poll() {
            match event {
                PlayEvent::Stepped(outcome) => self
                    .apply_receipt(&outcome, now)
                    .map_err(|_error| PublicClass::SessionUnavailable)?,
                PlayEvent::Pushed(pushed) => self
                    .apply_pushed(&pushed)
                    .map_err(|_error| PublicClass::SessionUnavailable)?,
                PlayEvent::Combat(report) => self.apply_combat(report),
                PlayEvent::Used(outcome) => {
                    self.combat_line = Some(
                        match outcome.as_deref().map(|outcome| outcome.disposition) {
                            Some(oteryn_session::UseDisposition::Committed) => "Item used",
                            Some(oteryn_session::UseDisposition::StaleState) | None => {
                                "Item is no longer available"
                            }
                            Some(oteryn_session::UseDisposition::TooFar) => {
                                "That item is out of reach"
                            }
                            _ => "Item use refused",
                        },
                    )
                }
                PlayEvent::Ended(class) => {
                    self.walk.cancel();
                    self.inputs.clear();
                    self.in_flight = false;
                    self.paced = false;
                    self.walking = None;
                    return Err(class);
                }
            }
        }
        if let Some(animation) = self.walking {
            if animation.finished(now) {
                self.walking = None;
            }
            self.rebuild_at(now)
                .map_err(|_| PublicClass::SessionUnavailable)?;
        }
        // One chronological queue: each input goes out in the order it was given. A step still
        // in flight holds back the arrow and what was given after it.
        while let Some(&input) = self.inputs.front() {
            match input {
                Input::Cast(spell) => link.request_combat(CombatCommand::Cast(spell)),
                Input::Attack(target) => link.request_combat(CombatCommand::Attack(target)),
                Input::Stop => link.request_combat(CombatCommand::Stop),
                Input::UseMap(target) => {
                    if link.commands.send(PlayCommand::UseMap(target)).is_ok() {
                        lock(&link.mailbox).wake.notify_one();
                    }
                }
                Input::Arrow(_) => {
                    if !link
                        .request_step_at(now, || self.send_step())
                        .map_err(|_error| PublicClass::SessionUnavailable)?
                    {
                        break;
                    }
                }
            }
            if !matches!(input, Input::Arrow(_)) {
                self.inputs.pop_front();
            }
        }
        if self.inputs.is_empty() {
            link.request_step_at(now, || self.send_step())
                .map_err(|_error| PublicClass::SessionUnavailable)?;
        }
        Ok(())
    }

    /// [`Self::next_step`], redrawing at once when the player turns, so the new facing shows
    /// before the outcome arrives.
    fn send_step(&mut self) -> Result<Option<StepDir>, BatchError> {
        let before = self.facing;
        let direction = self.next_step();
        if self.facing != before {
            self.scene.set_facing(self.facing)?;
        }
        Ok(direction)
    }
}

/// What the session task reports to the shell.
#[derive(Debug)]
pub enum PlayEvent {
    Stepped(Box<StepReceiptOutcome>),
    /// Deltas pushed outside any step, coalesced since the last poll.
    Pushed(Pushed),
    /// The server's answer to a combat command.
    Combat(CombatReport),
    Used(Option<Box<oteryn_session::UseOutcome>>),
    /// The session ended; return to login with this public class.
    Ended(PublicClass),
}

/// Pushed deltas coalesced to latest state, so an undrained shell holds a bounded amount.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Pushed {
    /// The latest own-actor position pushed by domain 1.
    pub own: Option<ActorPosition>,
    /// The latest full list of the other visible entities, when it changed.
    pub entities: Option<Vec<WorldSpatialEntity>>,
    /// Complete latest-state map, bounded by the Session store, not a growing delta queue.
    pub map: Option<Box<MapView>>,
    /// The latest domain-2 entry per placement, in first-pushed order of placement.
    pub markers: Vec<WorldObjectOverlayEntry>,
}

impl Pushed {
    fn set_marker(&mut self, entry: WorldObjectOverlayEntry) {
        if let Some(held) = self
            .markers
            .iter_mut()
            .find(|held| held.placement == entry.placement)
        {
            *held = entry;
        } else if self.markers.len() < MAX_PUSHED_MARKERS {
            self.markers.push(entry);
        }
    }

    /// Folds `newer` over `self`: its position and entries win.
    fn absorb(&mut self, newer: Self) {
        self.own = newer.own.or(self.own);
        self.entities = newer.entities.or(self.entities.take());
        self.map = newer.map.or(self.map.take());
        for entry in newer.markers {
            self.set_marker(entry);
        }
    }
}

/// Most distinct placements held while the shell is not draining; a later new placement is
/// dropped until the next poll, an update of a held one is still kept.
const MAX_PUSHED_MARKERS: usize = 1024;

#[derive(Debug, Default)]
struct Mailbox {
    /// Pushes that arrived before `stepped` was stored: older than its outcome.
    before: Pushed,
    stepped: Option<Box<StepReceiptOutcome>>,
    /// Held from enqueue until the shell consumes the outcome, not just until task completion.
    step_pending: bool,
    step_deadline: StepDeadline,
    connection_generation: Option<u64>,
    wake: Arc<tokio::sync::Notify>,
    pushed: Pushed,
    last_map_revision: Option<u64>,
    /// Combat answers not yet polled; the oldest is dropped past [`MAX_COMBAT_REPORTS`].
    combat: VecDeque<CombatReport>,
    used: Option<Option<Box<oteryn_session::UseOutcome>>>,
    ended: Option<PublicClass>,
    end_delivered: bool,
    game: GameState,
    chat_pending: Option<oteryn_session::ChatIntent>,
    chat_result: Option<oteryn_session::ChatOutcome>,
    chat_receipts: VecDeque<ChatReceipt>,
    action_pending: Option<crate::action_bar::ActionBarCommand>,
    action_result: Option<crate::action_bar::ActionBarOutcome>,
}

const MAX_COMBAT_REPORTS: usize = 8;

type SharedMailbox = Arc<Mutex<Mailbox>>;

fn lock(mailbox: &SharedMailbox) -> MutexGuard<'_, Mailbox> {
    mailbox.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The task's end of the mailbox.
#[derive(Debug, Clone)]
pub struct Outbox(SharedMailbox);

impl Outbox {
    #[cfg(test)]
    fn stepped(&self, outcome: StepOutcome) {
        self.stepped_with_receipt(outcome.into());
    }

    fn stepped_with_receipt(&self, result: StepReceiptOutcome) {
        let mut mailbox = lock(&self.0);
        mailbox.step_deadline.reset();
        if let Some(receipt) = result.receipt
            && (receipt.command_id != result.command_id
                || mailbox.connection_generation != Some(receipt.connection_generation)
                || mailbox
                    .step_deadline
                    .observe(receipt.received_at, receipt.wait_ms)
                    .is_err())
        {
            mailbox.ended.get_or_insert(PublicClass::SessionUnavailable);
            mailbox.step_pending = false;
            return;
        }
        let older = std::mem::take(&mut mailbox.pushed);
        mailbox.before.absorb(older);
        mailbox.stepped = Some(Box::new(result));
    }

    fn push(&self, events: Vec<SessionEvent>) {
        let mut mailbox = lock(&self.0);
        for event in events {
            match event {
                SessionEvent::WorldSpatial(delta) => {
                    mailbox.pushed.own = Some(delta.value.actor_position);
                }
                SessionEvent::WorldSpatialEntities(delta) => {
                    mailbox.pushed.own = Some(delta.value.actor_position);
                }
                SessionEvent::WorldObjectOverlay(delta) => mailbox.pushed.set_marker(delta.value),
                SessionEvent::ChatLine(delta) => {
                    if mailbox.chat_receipts.len() == oteryn_session::MAX_CHAT_LOG_LINES {
                        mailbox.chat_receipts.pop_front();
                    }
                    mailbox.chat_receipts.push_back(ChatReceipt {
                        server_sequence: delta.server_sequence,
                        line: delta.value,
                        local_time: time::OffsetDateTime::now_local()
                            .ok()
                            .map(|now| [now.hour(), now.minute(), now.second()]),
                    });
                }
                _ => {}
            }
        }
    }

    fn entities(&self, entities: Vec<WorldSpatialEntity>) {
        lock(&self.0).pushed.entities = Some(entities);
    }

    fn map(&self, revision: u64, map: &MapView) {
        let mut mailbox = lock(&self.0);
        if mailbox.last_map_revision != Some(revision) {
            mailbox.pushed.map = Some(Box::new(map.clone()));
            mailbox.last_map_revision = Some(revision);
        }
    }

    fn combat(&self, report: CombatReport) {
        let mut mailbox = lock(&self.0);
        if mailbox.combat.len() == MAX_COMBAT_REPORTS {
            mailbox.combat.pop_front();
        }
        mailbox.combat.push_back(report);
    }

    /// The first end wins.
    fn end(&self, class: PublicClass) {
        let mut mailbox = lock(&self.0);
        mailbox.ended.get_or_insert(class);
        mailbox.step_deadline.reset();
    }
}

/// Read-only, capability-gated projections copied from the owning session.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameState {
    pub vitals: Option<oteryn_session::ActorVitals>,
    pub inventory: Option<oteryn_session::CharacterInventory>,
    pub container: Option<oteryn_session::OpenContainer>,
    pub chat: Option<oteryn_session::ChatLog>,
    /// Bounded received lines with client receipt times; these are not server
    /// timestamps or additional authoritative chat state.
    pub chat_receipts: Vec<ChatReceipt>,
    pub entities: Vec<oteryn_session::WorldSpatialEntity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatReceipt {
    pub server_sequence: u64,
    pub line: oteryn_session::ChatLine,
    pub local_time: Option<[u8; 3]>,
}

/// The shell's end of the session task: commands out, bounded projections in.
#[derive(Debug)]
pub struct PlayLink {
    commands: UnboundedSender<PlayCommand>,
    mailbox: SharedMailbox,
}

impl PlayLink {
    /// One pending action shared by mouse activation and physical shortcuts.
    pub fn send_action(&self, command: crate::action_bar::ActionBarCommand) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.ended.is_some() || mailbox.action_pending.is_some() {
            return false;
        }
        mailbox.action_pending = Some(command);
        mailbox.wake.notify_one();
        true
    }

    #[must_use]
    pub fn action_result(&self) -> Option<crate::action_bar::ActionBarOutcome> {
        lock(&self.mailbox).action_result.clone()
    }

    #[must_use]
    pub fn game_state(&self) -> GameState {
        let mailbox = lock(&self.mailbox);
        let mut game = mailbox.game.clone();
        game.chat_receipts = mailbox.chat_receipts.iter().cloned().collect();
        game
    }

    /// One pending chat intent: repeated UI submissions cannot grow an unbounded queue.
    pub fn send_chat(&self, intent: oteryn_session::ChatIntent) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.ended.is_some() || mailbox.chat_pending.is_some() {
            return false;
        }
        mailbox.chat_pending = Some(intent);
        mailbox.wake.notify_one();
        true
    }

    #[must_use]
    pub fn chat_result(&self) -> Option<oteryn_session::ChatOutcome> {
        lock(&self.mailbox).chat_result.clone()
    }

    /// A busy link rejects another step; no second outcome can overwrite an undrained one.
    pub fn request(&self, direction: StepDir) -> bool {
        self.request_step(|| Ok(Some(direction))).unwrap_or(false)
    }

    /// Reserve the single step before consuming view input. The same lock protects direct
    /// callers, the shell and the task; pushes and task completion do not release it.
    fn request_step(
        &self,
        next: impl FnOnce() -> Result<Option<StepDir>, BatchError>,
    ) -> Result<bool, BatchError> {
        self.request_step_at(Instant::now(), next)
    }

    fn request_step_at(
        &self,
        now: Instant,
        next: impl FnOnce() -> Result<Option<StepDir>, BatchError>,
    ) -> Result<bool, BatchError> {
        let mut mailbox = lock(&self.mailbox);
        if self.commands.is_closed() {
            mailbox.ended.get_or_insert(PublicClass::SessionUnavailable);
        }
        if mailbox.ended.is_some() || mailbox.step_pending || !mailbox.step_deadline.ready(now) {
            return Ok(false);
        }
        let Some(direction) = next()? else {
            return Ok(false);
        };
        mailbox.step_pending = true;
        if self.commands.send(PlayCommand::Step(direction)).is_err() {
            mailbox.step_pending = false;
            mailbox.ended.get_or_insert(PublicClass::SessionUnavailable);
            return Ok(false);
        }
        mailbox.wake.notify_one();
        Ok(true)
    }

    /// A future native wake-up, not a stale deadline that would busy-loop the shell.
    #[must_use]
    pub fn step_deadline(&self, now: Instant) -> Option<Instant> {
        let mailbox = lock(&self.mailbox);
        (!mailbox.step_pending && mailbox.ended.is_none())
            .then(|| mailbox.step_deadline.next())
            .flatten()
            .filter(|deadline| *deadline > now)
    }

    pub fn request_combat(&self, command: CombatCommand) {
        if self.commands.send(PlayCommand::Combat(command)).is_ok() {
            lock(&self.mailbox).wake.notify_one();
        }
    }

    /// The next event: the pushes older than the step outcome, the step outcome, then the coalesced pushes, then the end.
    pub fn poll(&self) -> Option<PlayEvent> {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.before != Pushed::default() {
            return Some(PlayEvent::Pushed(std::mem::take(&mut mailbox.before)));
        }
        if let Some(outcome) = mailbox.stepped.take() {
            mailbox.step_pending = false;
            return Some(PlayEvent::Stepped(outcome));
        }
        if mailbox.pushed != Pushed::default() {
            return Some(PlayEvent::Pushed(std::mem::take(&mut mailbox.pushed)));
        }
        if let Some(report) = mailbox.combat.pop_front() {
            return Some(PlayEvent::Combat(report));
        }
        if let Some(outcome) = mailbox.used.take() {
            return Some(PlayEvent::Used(outcome));
        }
        let class = mailbox.ended?;
        if mailbox.end_delivered {
            return None;
        }
        mailbox.end_delivered = true;
        Some(PlayEvent::Ended(class))
    }
}

/// The session calls the task needs; implemented by `Session`.
pub trait Stepper: Send {
    fn connection_generation(&self) -> Option<u64> {
        None
    }
    fn use_map(
        &mut self,
        _target: MapTarget,
    ) -> impl Future<Output = Result<Option<oteryn_session::UseOutcome>, SessionError>> + Send {
        async { Ok(None) }
    }
    fn map_view(&self) -> Option<(u64, &MapView)> {
        None
    }
    fn action(
        &mut self,
        _command: &crate::action_bar::ActionBarCommand,
    ) -> impl Future<Output = Result<Option<crate::action_bar::ActionBarOutcome>, SessionError>> + Send
    {
        async { Ok(None) }
    }

    fn game_state(&self) -> GameState {
        GameState::default()
    }

    fn chat(
        &mut self,
        _intent: &oteryn_session::ChatIntent,
    ) -> impl Future<Output = Result<Option<oteryn_session::ChatOutcome>, SessionError>> + Send
    {
        async { Ok(None) }
    }

    fn step(
        &mut self,
        direction: StepDirection,
    ) -> impl Future<Output = Result<StepOutcome, SessionError>> + Send;
    fn step_with_receipt(
        &mut self,
        direction: StepDirection,
    ) -> impl Future<Output = Result<StepReceiptOutcome, SessionError>> + Send {
        async move { self.step(direction).await.map(StepReceiptOutcome::from) }
    }
    fn serve(
        &mut self,
        duration: Duration,
    ) -> impl Future<Output = Result<(), SessionError>> + Send;
    fn serve_or_wake(
        &mut self,
        duration: Duration,
        _wake: &tokio::sync::Notify,
    ) -> impl Future<Output = Result<(), SessionError>> + Send {
        async move { self.serve(duration).await }
    }
    /// The pushed deltas applied since the last call; the session fails closed when this queue
    /// is left to fill.
    fn drain(&mut self) -> Vec<SessionEvent>;
    /// Runs one combat command (attack target, cast).
    fn combat(
        &mut self,
        command: CombatCommand,
    ) -> impl Future<Output = Result<CombatReport, SessionError>> + Send;
    /// The other visible entities when they differ from `held`.
    fn entities_changed(&self, _held: &[WorldSpatialEntity]) -> Option<Vec<WorldSpatialEntity>> {
        None
    }
}

impl<S: SessionStream + Send> Stepper for Session<S> {
    fn connection_generation(&self) -> Option<u64> {
        Some(Session::connection_generation(self))
    }
    async fn use_map(
        &mut self,
        target: MapTarget,
    ) -> Result<Option<oteryn_session::UseOutcome>, SessionError> {
        self.use_map_target(target).await
    }
    fn map_view(&self) -> Option<(u64, &MapView)> {
        self.world_map_view()
            .map(|map| (self.world_map_revision(), map))
    }
    async fn action(
        &mut self,
        command: &crate::action_bar::ActionBarCommand,
    ) -> Result<Option<crate::action_bar::ActionBarOutcome>, SessionError> {
        match command.execute(self).await {
            Err(SessionError::CapabilityNotSelected { .. }) => {
                Ok(Some(crate::action_bar::ActionBarOutcome::Unavailable))
            }
            result => result.map(Some),
        }
    }

    fn game_state(&self) -> GameState {
        GameState {
            vitals: self.actor_vitals().copied(),
            inventory: self.inventory().cloned(),
            container: self.open_container().cloned(),
            chat: self.chat_log().cloned(),
            chat_receipts: Vec::new(),
            entities: self
                .world_entities()
                .map(|world| world.iter().copied().collect())
                .unwrap_or_default(),
        }
    }

    async fn chat(
        &mut self,
        intent: &oteryn_session::ChatIntent,
    ) -> Result<Option<oteryn_session::ChatOutcome>, SessionError> {
        Session::chat(self, intent).await.map(Some)
    }

    fn step(
        &mut self,
        direction: StepDirection,
    ) -> impl Future<Output = Result<StepOutcome, SessionError>> + Send {
        self.step_retrying(direction)
    }

    fn step_with_receipt(
        &mut self,
        direction: StepDirection,
    ) -> impl Future<Output = Result<StepReceiptOutcome, SessionError>> + Send {
        self.step_retrying_with_receipt(direction)
    }

    fn serve(
        &mut self,
        duration: Duration,
    ) -> impl Future<Output = Result<(), SessionError>> + Send {
        self.service_liveness(duration)
    }

    fn serve_or_wake(
        &mut self,
        duration: Duration,
        wake: &tokio::sync::Notify,
    ) -> impl Future<Output = Result<(), SessionError>> + Send {
        self.service_liveness_interruptible(duration, wake)
    }

    fn drain(&mut self) -> Vec<SessionEvent> {
        self.take_events()
    }

    async fn combat(&mut self, command: CombatCommand) -> Result<CombatReport, SessionError> {
        run_combat(self, &mut SpellFeedback::default(), command).await
    }

    fn entities_changed(&self, held: &[WorldSpatialEntity]) -> Option<Vec<WorldSpatialEntity>> {
        let entities = self.world_entities()?;
        (!entities.others().eq(held.iter())).then(|| entities.others().copied().collect())
    }
}

/// The channel pair between the shell and `run_session`.
pub fn play_channel() -> (PlayLink, UnboundedReceiver<PlayCommand>, Outbox) {
    let (commands, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let mailbox = SharedMailbox::default();
    (
        PlayLink {
            commands,
            mailbox: Arc::clone(&mailbox),
        },
        command_rx,
        Outbox(mailbox),
    )
}

/// Hands the shell what the session pushed, and the visible entities when they changed.
fn publish<T: Stepper>(session: &mut T, outbox: &Outbox, held: &mut Vec<WorldSpatialEntity>) {
    outbox.push(session.drain());
    if let Some((revision, map)) = session.map_view() {
        outbox.map(revision, map);
    }
    if let Some(entities) = session.entities_changed(held) {
        held.clone_from(&entities);
        outbox.entities(entities);
    }
}

/// The session task, run on the client runtime: sends requested steps and, while none is
/// pending, keeps servicing liveness so an idle session is not dropped. It ends with the first
/// session error (reported with its public class) or when the shell drops its link.
pub async fn run_session<T: Stepper>(
    mut session: T,
    mut commands: UnboundedReceiver<PlayCommand>,
    outbox: Outbox,
) {
    let wake = Arc::clone(&lock(&outbox.0).wake);
    {
        let mut mailbox = lock(&outbox.0);
        mailbox.game = session.game_state();
        mailbox.connection_generation = session.connection_generation();
        mailbox.step_deadline.reset();
    }
    let mut held = Vec::new();
    loop {
        let action = lock(&outbox.0).action_pending.take();
        if let Some(command) = action {
            match session.action(&command).await {
                Ok(result) => {
                    lock(&outbox.0).action_result = result;
                    publish(&mut session, &outbox, &mut held);
                    lock(&outbox.0).game = session.game_state();
                }
                Err(error) => {
                    outbox.end(public_class(&error));
                    return;
                }
            }
        }
        let pending = lock(&outbox.0).chat_pending.take();
        if let Some(intent) = pending {
            match session.chat(&intent).await {
                Ok(result) => {
                    lock(&outbox.0).chat_result = result;
                    publish(&mut session, &outbox, &mut held);
                    lock(&outbox.0).game = session.game_state();
                }
                Err(error) => {
                    outbox.end(public_class(&error));
                    return;
                }
            }
        }
        let ended = match commands.try_recv() {
            Ok(PlayCommand::UseMap(target)) => match session.use_map(target).await {
                Ok(outcome) => {
                    lock(&outbox.0).used = Some(outcome.map(Box::new));
                    publish(&mut session, &outbox, &mut held);
                    lock(&outbox.0).game = session.game_state();
                    continue;
                }
                Err(error) => error,
            },
            Ok(PlayCommand::Step(direction)) => {
                match session.step_with_receipt(step_direction(direction)).await {
                    Ok(outcome) => {
                        outbox.stepped_with_receipt(outcome);
                        publish(&mut session, &outbox, &mut held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Ok(PlayCommand::Combat(command)) => match session.combat(command).await {
                Ok(report) => {
                    outbox.combat(report);
                    publish(&mut session, &outbox, &mut held);
                    lock(&outbox.0).game = session.game_state();
                    continue;
                }
                Err(error) => error,
            },
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                match session.serve_or_wake(IDLE_SLICE, &wake).await {
                    Ok(()) => {
                        publish(&mut session, &outbox, &mut held);
                        lock(&outbox.0).game = session.game_state();
                        continue;
                    }
                    Err(error) => error,
                }
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => return,
        };
        outbox.end(public_class(&ended));
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_client_runtime::ClientRuntime;
    use oteryn_session::{
        ActorPosition, AppliedDelta, AttackIntentDisposition, CommandStatus, MAX_QUEUED_EVENTS,
        SpellCastDisposition, WorldSpatialObservation,
    };
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn chat_receipts_keep_identical_lines_distinct_bounded_and_read_only() {
        let (link, _commands, outbox) = play_channel();
        let line = oteryn_session::ChatLine::Private {
            speaker_name: "Peer".into(),
            text: "same".into(),
        };
        for sequence in 1..=70 {
            outbox.push(vec![SessionEvent::ChatLine(AppliedDelta {
                server_sequence: sequence,
                base_revision: sequence - 1,
                new_revision: sequence,
                value: line.clone(),
            })]);
        }
        let mut displayed = link.game_state();
        assert_eq!(
            displayed.chat_receipts.len(),
            oteryn_session::MAX_CHAT_LOG_LINES
        );
        assert_eq!(displayed.chat_receipts[0].server_sequence, 7);
        assert_eq!(displayed.chat_receipts[63].server_sequence, 70);
        #[cfg(windows)]
        assert!(
            displayed
                .chat_receipts
                .iter()
                .all(|receipt| receipt.local_time.is_some())
        );
        assert!(
            displayed
                .chat_receipts
                .iter()
                .all(|receipt| receipt.line == line)
        );
        displayed.chat_receipts.clear();
        assert_eq!(link.game_state().chat_receipts.len(), 64);
    }

    #[test]
    fn chat_submission_is_bounded_and_terminal_sessions_reject_it() {
        let (link, _commands, outbox) = play_channel();
        let intent = oteryn_session::ChatIntent::Say {
            mode: oteryn_session::ChatSpeechMode::Say,
            text: "hello".into(),
        };
        assert!(link.send_chat(intent.clone()));
        assert!(!link.send_chat(intent.clone()));
        lock(&outbox.0).chat_pending.take();
        assert!(link.send_chat(intent.clone()));
        outbox.end(PublicClass::SessionUnavailable);
        assert!(!link.send_chat(intent));
    }

    #[test]
    fn hud_state_is_a_copy_and_cannot_mutate_session_projections()
    -> Result<(), Box<dyn std::error::Error>> {
        let (link, _commands, outbox) = play_channel();
        lock(&outbox.0).game.vitals = Some(oteryn_session::ActorVitals {
            health: 37,
            max_health: 100,
            mana: 12,
            max_mana: 50,
            ..Default::default()
        });
        let mut displayed = link.game_state();
        displayed.vitals.as_mut().ok_or("vitals")?.health = 100;
        assert_eq!(link.game_state().vitals.ok_or("server copy")?.health, 37);
        Ok(())
    }

    fn key(x: u32, y: u16, floor: i16, ordinal: u8) -> Vec<u8> {
        let floor_byte = u64::from(floor.unsigned_abs());
        let key = u64::from(x) << 32 | u64::from(y) << 16 | floor_byte << 8 | u64::from(ordinal);
        key.to_be_bytes().to_vec()
    }

    fn entry(placement: Vec<u8>) -> WorldObjectOverlayEntry {
        WorldObjectOverlayEntry {
            content_generation: [0; 32],
            placement,
            state: Vec::new(),
            revision: 1,
        }
    }

    fn moved_to(x: i32, y: i32, floor: i16) -> StepOutcome {
        StepOutcome {
            command_id: 1,
            status: CommandStatus::Accepted,
            disposition: StepDisposition::Moved,
            result_server_sequence: 1,
            world_spatial_delta: Some(AppliedDelta {
                server_sequence: 2,
                base_revision: 1,
                new_revision: 2,
                value: WorldSpatialObservation {
                    content_generation: [0; 32],
                    actor_position: ActorPosition { x, y, floor },
                },
            }),
            world_object_overlay_delta: None,
        }
    }

    fn timed(
        mut outcome: StepOutcome,
        received_at: Instant,
        wait_ms: u32,
        early: bool,
    ) -> StepReceiptOutcome {
        if early {
            outcome.disposition = StepDisposition::TooEarly;
            outcome.status = CommandStatus::Rejected;
            outcome.world_spatial_delta = None;
        }
        let receipt = Some(oteryn_session::StepReceipt {
            command_id: outcome.command_id,
            connection_generation: 1,
            received_at,
            wait_ms,
        });
        StepReceiptOutcome { outcome, receipt }
    }

    #[test]
    fn real_dispatch_gate_runs_ten_thousand_jittered_receipts_without_early_or_burst_steps()
    -> Result<(), Box<dyn std::error::Error>> {
        let (link, mut commands, outbox) = play_channel();
        lock(&outbox.0).connection_generation = Some(1);
        let mut now = Instant::now();
        let mut seed = 17u64;
        assert!(link.request_step_at(now, || Ok(Some(StepDir::East)))?);
        commands.try_recv()?;
        for index in 0..10_000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let received = now + Duration::from_millis(seed % 38);
            let early = index % 17 == 0;
            let wait = if early {
                37
            } else if index == 9999 {
                111150
            } else {
                50 * (1 + index % 5)
            };
            let mut outcome = moved_to(100, 200, 0);
            outcome.command_id = u64::from(index) + 7;
            outbox.stepped_with_receipt(timed(outcome, received, wait, early));
            // A completed but undrained outcome retains the one-flight reservation.
            assert!(!link.request_step_at(received, || Err(BatchError::InvalidView))?);
            assert!(matches!(link.poll(), Some(PlayEvent::Stepped(_))));
            let due = received + Duration::from_millis(u64::from(wait));
            now = received + Duration::from_millis((seed >> 32) % 20);
            while now < due {
                // An early attempt must not consume any queued view input.
                assert!(!link.request_step_at(now, || Err(BatchError::InvalidView))?);
                assert!(commands.try_recv().is_err());
                now += Duration::from_millis(20);
            }
            assert!(now.duration_since(due) <= Duration::from_millis(20));
            assert!(link.request_step_at(now, || Ok(Some(StepDir::East)))?);
            assert!(matches!(
                commands.try_recv()?,
                PlayCommand::Step(StepDir::East)
            ));
            assert!(!link.request_step_at(now, || Err(BatchError::InvalidView))?);
            assert!(commands.try_recv().is_err());
        }
        Ok(())
    }

    #[test]
    fn receipt_too_early_preserves_goal_latest_buffer_and_exact_remaining_deadline()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut view = view()?;
        let (link, mut commands, outbox) = play_channel();
        lock(&outbox.0).connection_generation = Some(1);
        let received = Instant::now();
        view.click(TileCoord::new(98, 200))?;
        view.tick_at(&link, received)
            .map_err(|_| "initial tick failed")?;
        assert!(matches!(
            commands.try_recv()?,
            PlayCommand::Step(StepDir::West)
        ));
        outbox.stepped_with_receipt(timed(moved_to(100, 200, 0), received, 37, true));
        view.tick_at(&link, received)
            .map_err(|_| "receipt tick failed")?;
        assert_eq!(view.walk.goal(), Some(TileCoord::new(98, 200)));
        assert!(commands.try_recv().is_err());
        view.arrow(StepDir::North);
        view.cast(NonZeroU32::MIN);
        view.arrow(StepDir::East);
        assert_eq!(
            view.inputs
                .iter()
                .filter(|input| matches!(input, Input::Arrow(_)))
                .count(),
            1
        );
        view.tick_at(&link, received + Duration::from_millis(36))
            .map_err(|_| "early tick failed")?;
        assert!(matches!(
            commands.try_recv()?,
            PlayCommand::Combat(CombatCommand::Cast(_))
        ));
        assert!(commands.try_recv().is_err());
        assert_eq!(view.inputs.front(), Some(&Input::Arrow(StepDir::East)));
        view.tick_at(&link, received + Duration::from_millis(37))
            .map_err(|_| "due tick failed")?;
        assert!(matches!(
            commands.try_recv()?,
            PlayCommand::Step(StepDir::East)
        ));
        assert!(commands.try_recv().is_err());
        // Stop is drained before the outstanding TOO_EARLY arrives: it must not resurrect.
        view.stop_all();
        view.tick_at(&link, received + Duration::from_millis(37))
            .map_err(|_| "stop tick failed")?;
        assert!(matches!(
            commands.try_recv()?,
            PlayCommand::Combat(CombatCommand::Stop)
        ));
        outbox.stepped_with_receipt(timed(
            moved_to(100, 200, 0),
            received + Duration::from_millis(37),
            37,
            true,
        ));
        view.tick_at(&link, received + Duration::from_millis(74))
            .map_err(|_| "cancelled retry tick failed")?;
        assert!(commands.try_recv().is_err());
        view.arrow(StepDir::North);
        view.tick_at(&link, received + Duration::from_millis(74))
            .map_err(|_| "fresh intent tick failed")?;
        assert!(matches!(
            commands.try_recv()?,
            PlayCommand::Step(StepDir::North)
        ));
        Ok(())
    }

    #[test]
    fn receipt_generation_fence_and_terminal_reset_do_not_survive_fresh_admission()
    -> Result<(), Box<dyn std::error::Error>> {
        let now = Instant::now();
        let (link, _commands, outbox) = play_channel();
        lock(&outbox.0).connection_generation = Some(1);
        let mut wrong = timed(moved_to(100, 200, 0), now, 100, false);
        wrong
            .receipt
            .as_mut()
            .ok_or("missing receipt")?
            .connection_generation = 2;
        outbox.stepped_with_receipt(wrong);
        assert!(!link.request(StepDir::East));
        assert!(matches!(
            link.poll(),
            Some(PlayEvent::Ended(PublicClass::SessionUnavailable))
        ));
        let (next, mut commands, outbox) = play_channel();
        assert!(next.request_step_at(now, || Ok(Some(StepDir::North)))?);
        commands.try_recv()?;
        lock(&outbox.0).connection_generation = Some(3);
        let mut result = timed(moved_to(100, 199, 0), now, 111150, false);
        result
            .receipt
            .as_mut()
            .ok_or("missing receipt")?
            .connection_generation = 3;
        outbox.stepped_with_receipt(result);
        outbox.end(PublicClass::SessionUnavailable);
        assert_eq!(next.step_deadline(now), None);
        assert!(
            !next.request_step_at(now + Duration::from_secs(200), || Ok(Some(StepDir::North)))?
        );
        let (fresh, _commands, _outbox) = play_channel();
        assert!(fresh.request_step_at(now, || Ok(Some(StepDir::North)))?);
        Ok(())
    }

    #[test]
    fn native_task_uses_timed_outcomes_and_command_wakes_instead_of_legacy_methods()
    -> Result<(), Box<dyn std::error::Error>> {
        struct TimedStepper {
            received_at: Instant,
            calls: Arc<AtomicUsize>,
        }
        impl Stepper for TimedStepper {
            fn connection_generation(&self) -> Option<u64> {
                Some(1)
            }
            async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
                Err(SessionError::SessionUnusable) // The native task must not discard timing here.
            }
            async fn step_with_receipt(
                &mut self,
                _: StepDirection,
            ) -> Result<StepReceiptOutcome, SessionError> {
                let index = self.calls.fetch_add(1, Ordering::SeqCst);
                Ok(timed(
                    moved_to(100, 199, 0),
                    self.received_at + Duration::from_millis(index as u64 * 100),
                    100,
                    false,
                ))
            }
            async fn serve(&mut self, _: Duration) -> Result<(), SessionError> {
                Err(SessionError::SessionUnusable) // Old idle polling must not hide the wake.
            }
            async fn serve_or_wake(
                &mut self,
                _: Duration,
                wake: &tokio::sync::Notify,
            ) -> Result<(), SessionError> {
                wake.notified().await;
                Ok(())
            }
            fn drain(&mut self) -> Vec<SessionEvent> {
                vec![]
            }
            async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
                Err(SessionError::SessionUnusable)
            }
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(async {
            let (link, commands, outbox) = play_channel();
            let calls = Arc::new(AtomicUsize::new(0));
            let received = Instant::now();
            let mut task = std::pin::pin!(run_session(
                TimedStepper {
                    received_at: received,
                    calls: Arc::clone(&calls),
                },
                commands,
                outbox
            ));
            let first =
                std::future::poll_fn(|cx| std::task::Poll::Ready(task.as_mut().poll(cx))).await;
            assert!(first.is_pending()); // Native idle future has registered its command wake.
            assert!(link.request_step_at(received, || Ok(Some(StepDir::North)))?);
            let next =
                std::future::poll_fn(|cx| std::task::Poll::Ready(task.as_mut().poll(cx))).await;
            assert!(next.is_pending());
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            assert!(
                matches!(link.poll(), Some(PlayEvent::Stepped(result)) if result.receipt.is_some())
            );
            assert!(
                !link.request_step_at(received + Duration::from_millis(99), || Err(
                    BatchError::InvalidView
                ))?
            );
            assert!(
                link.request_step_at(received + Duration::from_millis(100), || Ok(Some(
                    StepDir::North
                )))?
            );
            let next =
                std::future::poll_fn(|cx| std::task::Poll::Ready(task.as_mut().poll(cx))).await;
            assert!(next.is_pending());
            assert_eq!(calls.load(Ordering::SeqCst), 2);
            assert!(
                matches!(link.poll(), Some(PlayEvent::Stepped(result)) if result.receipt.is_some())
            );
            Ok::<_, Box<dyn std::error::Error>>(())
        })
    }

    #[test]
    fn receipt_animates_actual_streamed_actor_for_the_server_duration_and_resets_on_correction()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_session::map_view::{
            MapViewHeader, WorldMapViewSnapshot, encode_world_map_snapshot,
        };
        let mut view = PlayView::new(TileCoord::new(100, 200), -7, &[])?;
        let bytes = encode_world_map_snapshot(&WorldMapViewSnapshot {
            header: MapViewHeader {
                content_generation: [1; 32],
                bundle_digest: [2; 32],
                reset_epoch: 1,
                origin: ActorPosition {
                    x: 100,
                    y: 200,
                    floor: -7,
                },
                first_visible_floor: 0,
            },
            tiles: vec![],
        })
        .map_err(|e| format!("{e:?}"))?;
        let mut map = MapView::new([1; 32]).map_err(|e| format!("{e:?}"))?;
        map.replace_snapshot(&bytes).map_err(|e| format!("{e:?}"))?;
        view.apply_pushed(&Pushed {
            map: Some(Box::new(map)),
            ..Pushed::default()
        })?;
        let start = Instant::now();
        view.apply_receipt(&timed(moved_to(100, 199, -7), start, 100, false), start)?;
        assert_eq!(view.own(), TileCoord::new(100, 199)); // authority is never interpolated
        let (link, _commands, _outbox) = play_channel();
        for (elapsed, expected_y) in [(0, 288.0), (50, 264.0), (100, 240.0)] {
            view.tick_at(&link, start + Duration::from_millis(elapsed))
                .map_err(|_| "animation tick failed")?;
            let frame = view.scene().sprite_frame().ok_or("not streamed")?;
            assert_eq!(frame.quads().quad_count(), 1);
            assert_eq!(frame.quads().batches()[0][0].position, [336.0, expected_y]);
        }
        assert!(view.walking.is_none());
        view.apply_receipt(&timed(moved_to(101, 199, -7), start, 111150, false), start)?;
        assert_eq!(
            view.walking.ok_or("no animation")?.duration,
            Duration::from_millis(111150)
        );
        view.apply_pushed(&Pushed {
            own: Some(ActorPosition {
                x: 101,
                y: 200,
                floor: -7,
            }),
            ..Pushed::default()
        })?;
        assert!(view.walking.is_none());
        Ok(())
    }

    fn view() -> Result<PlayView, BatchError> {
        PlayView::new(
            TileCoord::new(100, 200),
            0,
            &[
                entry(key(102, 199, 0, 0)),
                entry(key(5, 5, 3, 0)),
                entry(b"x".to_vec()),
            ],
        )
    }

    fn actor(kind: EntityKind, ordinal: u8, x: i32) -> WorldSpatialEntity {
        WorldSpatialEntity {
            kind,
            entity: EntityRef {
                identity: [ordinal; 16],
                generation: u64::from(ordinal),
            },
            position: ActorPosition {
                x,
                y: 200,
                floor: 0,
            },
            detail: oteryn_session::EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        }
    }

    #[test]
    fn snapshot_builds_a_view_centred_on_the_own_position_with_overlay_markers()
    -> Result<(), BatchError> {
        let view = view()?;
        // Only the marker on the own floor and with a decodable placement is drawn.
        assert_eq!(view.marker_count(), 1);
        assert_eq!(view.markers.len(), 2);
        let scene = view.scene();
        assert_eq!(scene.sprites().len(), 2);
        let [x, y] = scene.view().tile_to_screen(TileCoord::new(100, 200));
        assert_eq!(
            scene.view().screen_to_tile(x + 1.0, y + 1.0),
            Some(TileCoord::new(100, 200))
        );
        assert_eq!(scene.view().origin(), TileCoord::new(93, 195));
        Ok(())
    }

    #[test]
    fn click_on_empty_ground_becomes_a_step_toward_it() -> Result<(), BatchError> {
        let mut view = view()?;
        view.click(TileCoord::new(98, 200))?;
        assert_eq!(view.next_step(), Some(StepDir::West));
        // One outstanding step: nothing more until its outcome.
        assert_eq!(view.next_step(), None);
        // A click on the marker selects it and sets no goal.
        let mut other = self::view()?;
        other.click(TileCoord::new(102, 199))?;
        assert_eq!(other.next_step(), None);
        Ok(())
    }

    #[test]
    fn battle_target_hotkeys_cycle_only_visible_attackable_entities() -> Result<(), BatchError> {
        let player = actor(EntityKind::Player, 1, 98);
        let first = actor(EntityKind::Creature, 2, 99);
        let second = actor(EntityKind::Creature, 3, 101);
        let npc = actor(EntityKind::Npc, 4, 102);
        let hidden_floor = WorldSpatialEntity {
            position: ActorPosition {
                floor: 1,
                ..first.position
            },
            ..actor(EntityKind::Creature, 5, 103)
        };
        let mut selected_view =
            view()?.with_entities(vec![player, first, second, npc, hidden_floor])?;

        assert!(selected_view.select_battle_target(BattleTargetSelection::First)?);
        assert_eq!(
            selected_view
                .scene()
                .target()
                .and_then(|target| target.entity),
            Some(first.entity)
        );
        assert!(selected_view.select_battle_target(BattleTargetSelection::Next)?);
        assert_eq!(
            selected_view
                .scene()
                .target()
                .and_then(|target| target.entity),
            Some(second.entity)
        );
        assert!(selected_view.select_battle_target(BattleTargetSelection::Previous)?);
        assert_eq!(
            selected_view
                .scene()
                .target()
                .and_then(|target| target.entity),
            Some(first.entity)
        );

        let mut previous = view()?.with_entities(vec![player, first, second, npc, hidden_floor])?;
        assert!(previous.select_battle_target(BattleTargetSelection::Previous)?);
        assert_eq!(
            previous.scene().target().and_then(|target| target.entity),
            Some(second.entity)
        );
        Ok(())
    }

    #[test]
    fn arrow_key_steps_without_a_goal() -> Result<(), BatchError> {
        let mut view = view()?;
        assert_eq!(view.facing(), StepDir::South);
        view.arrow(StepDir::North);
        assert_eq!(view.next_step(), Some(StepDir::North));
        assert_eq!(view.next_step(), None);
        // The player turns to the step sent, moved or not.
        assert_eq!(view.facing(), StepDir::North);
        Ok(())
    }

    #[test]
    fn inputs_keep_their_order_and_a_repeated_cast_is_one() -> Result<(), BatchError> {
        let mut view = view()?;
        let one = NonZeroU32::MIN;
        view.arrow(StepDir::North);
        view.cast(one);
        view.cast(one);
        view.arrow(StepDir::East);
        assert_eq!(
            view.inputs.iter().copied().collect::<Vec<_>>(),
            vec![
                Input::Arrow(StepDir::North),
                Input::Cast(one),
                Input::Arrow(StepDir::East)
            ]
        );
        // A cast waiting at the front holds the step behind it.
        view.inputs.pop_front();
        assert_eq!(view.next_step(), None);
        Ok(())
    }

    #[test]
    fn stop_all_replaces_queued_local_actions_with_one_stop_intent() -> Result<(), BatchError> {
        let mut view = view()?;
        view.click(TileCoord::new(98, 200))?;
        view.arrow(StepDir::North);
        view.cast(NonZeroU32::MIN);
        assert!(view.walk.goal().is_some());
        assert!(view.inputs.len() > 1);

        view.stop_all();
        assert_eq!(view.walk.goal(), None);
        assert_eq!(
            view.inputs.iter().copied().collect::<Vec<_>>(),
            vec![Input::Stop]
        );
        view.stop_all();
        assert_eq!(
            view.inputs.iter().copied().collect::<Vec<_>>(),
            vec![Input::Stop]
        );
        Ok(())
    }

    #[test]
    fn only_a_creature_click_queues_an_attack() -> Result<(), BatchError> {
        let actor = |kind, x, id: u8| WorldSpatialEntity {
            kind,
            entity: EntityRef {
                identity: [id; 16],
                generation: 1,
            },
            position: ActorPosition {
                x,
                y: 200,
                floor: 0,
            },
            detail: oteryn_session::EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        };
        let creature = actor(EntityKind::Creature, 101, 1);
        let mut view = view()?.with_entities(vec![creature, actor(EntityKind::Npc, 99, 2)])?;
        // The NPC is still drawn: player plus one glyph per visible entity.
        assert_eq!(view.scene().sprites().len(), 2 + 2);
        view.click(TileCoord::new(99, 200))?;
        assert!(view.inputs.is_empty());
        view.click(TileCoord::new(101, 200))?;
        assert_eq!(
            view.inputs.iter().copied().collect::<Vec<_>>(),
            vec![Input::Attack(creature.entity)]
        );
        Ok(())
    }

    #[test]
    fn sending_a_step_redraws_the_new_facing() -> Result<(), BatchError> {
        let mut view = view()?;
        view.click(view.own())?;
        let target = view.scene().target();
        assert!(target.is_some());
        view.arrow(StepDir::North);
        assert_eq!(view.send_step()?, Some(StepDir::North));
        assert_eq!(view.scene().facing(), StepDir::North);
        // Turning keeps the selection.
        assert_eq!(view.scene().target(), target);
        Ok(())
    }

    #[test]
    fn step_outcome_moves_the_view_and_a_refusal_ends_the_walk() -> Result<(), BatchError> {
        let mut view = view()?;
        view.click(TileCoord::new(98, 200))?;
        assert_eq!(view.next_step(), Some(StepDir::West));
        view.apply(&moved_to(99, 200, 0))?;
        assert_eq!(view.own(), TileCoord::new(99, 200));
        assert_eq!(view.scene().view().origin(), TileCoord::new(92, 195));
        assert_eq!(view.next_step(), Some(StepDir::West));
        let mut blocked = moved_to(99, 200, 0);
        blocked.disposition = StepDisposition::Blocked;
        blocked.world_spatial_delta = None;
        view.apply(&blocked)?;
        assert_eq!(view.own(), TileCoord::new(99, 200));
        assert_eq!(view.next_step(), None);
        Ok(())
    }

    #[test]
    fn overlay_delta_adds_a_marker() -> Result<(), BatchError> {
        let mut view = view()?;
        let mut outcome = moved_to(100, 199, 0);
        outcome.world_object_overlay_delta = Some(AppliedDelta {
            server_sequence: 3,
            base_revision: 1,
            new_revision: 2,
            value: entry(key(101, 199, 0, 1)),
        });
        view.apply(&outcome)?;
        assert_eq!(view.marker_count(), 2);
        Ok(())
    }

    #[test]
    fn any_session_error_returns_to_login_with_its_public_class() {
        let error = SessionError::SessionUnusable;
        assert_eq!(public_class(&error), PublicClass::SessionUnavailable);
    }

    #[test]
    fn markers_are_refiltered_when_the_floor_changes() -> Result<(), BatchError> {
        let mut view = view()?;
        assert_eq!(view.marker_count(), 1);
        view.apply(&moved_to(5, 5, -3))?;
        assert_eq!(view.marker_count(), 1);
        assert_eq!(view.scene().sprites().len(), 2);
        view.apply(&moved_to(100, 200, 0))?;
        assert_eq!(view.marker_count(), 1);
        view.apply(&moved_to(100, 200, -7))?;
        assert_eq!(view.marker_count(), 0);
        Ok(())
    }

    #[test]
    fn the_map_is_drawn_only_on_the_join_floor() -> Result<(), String> {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let world = Arc::new(World::load(&root)?);
        let mut view = view().map_err(|e| e.to_string())?;
        view.set_world(world).map_err(|e| e.to_string())?;
        let on_floor = view.scene().sprites().len();
        view.apply(&moved_to(100, 200, -1))
            .map_err(|e| e.to_string())?;
        let off_floor = view.scene().sprites().len();
        view.apply(&moved_to(100, 200, 0))
            .map_err(|e| e.to_string())?;
        assert!(off_floor < on_floor / 10, "{off_floor} of {on_floor}");
        assert_eq!(view.scene().sprites().len(), on_floor);
        Ok(())
    }

    #[test]
    fn one_step_is_in_flight_until_its_outcome() -> Result<(), BatchError> {
        let mut view = view()?;
        view.arrow(StepDir::North);
        assert_eq!(view.next_step(), Some(StepDir::North));
        view.arrow(StepDir::East);
        assert_eq!(view.next_step(), None);
        view.apply(&moved_to(100, 199, 0))?;
        assert_eq!(view.next_step(), Some(StepDir::East));
        Ok(())
    }

    #[test]
    fn direct_steps_wait_until_the_previous_outcome_is_consumed() {
        let (link, mut commands, outbox) = play_channel();
        link.request(StepDir::North);
        for _ in 0..100 {
            link.request(StepDir::East);
        }
        assert!(matches!(
            commands.try_recv(),
            Ok(PlayCommand::Step(StepDir::North))
        ));
        assert!(commands.try_recv().is_err());

        outbox.push(vec![spatial_event(1, 99)]);
        outbox.stepped(moved_to(100, 199, 0));
        outbox.push(vec![spatial_event(3, 101)]);
        // Neither task completion nor an older push releases the reservation.
        link.request(StepDir::West);
        assert!(matches!(link.poll(), Some(PlayEvent::Pushed(_))));
        link.request(StepDir::West);
        assert!(commands.try_recv().is_err());
        assert!(matches!(link.poll(), Some(PlayEvent::Stepped(outcome))
            if outcome.world_spatial_delta.as_ref().is_some_and(|delta| delta.value.actor_position.y == 199)));
        link.request(StepDir::West);
        assert!(matches!(
            commands.try_recv(),
            Ok(PlayCommand::Step(StepDir::West))
        ));
        assert!(matches!(link.poll(), Some(PlayEvent::Pushed(_))));
        link.request(StepDir::South);
        assert!(commands.try_recv().is_err());
    }

    #[test]
    fn consumed_end_stays_terminal_and_cannot_be_replaced() {
        let (link, mut commands, outbox) = play_channel();
        outbox.end(PublicClass::ClientUpdateRequired);
        assert!(matches!(
            link.poll(),
            Some(PlayEvent::Ended(PublicClass::ClientUpdateRequired))
        ));
        outbox.end(PublicClass::SessionUnavailable);
        assert!(link.poll().is_none());
        link.request(StepDir::North);
        assert!(commands.try_recv().is_err());
    }

    #[test]
    fn closed_step_channel_reports_end_without_consuming_input() -> Result<(), BatchError> {
        let (link, commands, _outbox) = play_channel();
        drop(commands);
        let mut view = view()?;
        view.arrow(StepDir::North);
        assert_eq!(view.tick(&link), Ok(()));
        assert_eq!(view.next_step(), Some(StepDir::North));
        assert!(matches!(
            link.poll(),
            Some(PlayEvent::Ended(PublicClass::SessionUnavailable))
        ));
        assert!(link.poll().is_none());
        Ok(())
    }

    #[test]
    fn a_busy_link_preserves_the_views_waiting_direction() -> Result<(), BatchError> {
        let (link, mut commands, outbox) = play_channel();
        let mut view = view()?;
        link.request(StepDir::East);
        assert!(matches!(
            commands.try_recv(),
            Ok(PlayCommand::Step(StepDir::East))
        ));
        view.arrow(StepDir::North);
        assert_eq!(view.tick(&link), Ok(()));
        assert!(commands.try_recv().is_err());
        outbox.stepped(moved_to(101, 200, 0));
        assert_eq!(view.tick(&link), Ok(()));
        assert_eq!(view.own(), TileCoord::new(101, 200));
        assert!(matches!(
            commands.try_recv(),
            Ok(PlayCommand::Step(StepDir::North))
        ));
        Ok(())
    }

    struct Fake {
        serves: Arc<AtomicUsize>,
        fail_step: bool,
        fail_serve_after: usize,
        queued: Vec<SessionEvent>,
        drained: Arc<AtomicUsize>,
        combats: Arc<Mutex<Vec<CombatCommand>>>,
        step_calls: Option<Arc<AtomicUsize>>,
        step_release: Option<tokio::sync::oneshot::Receiver<()>>,
    }

    impl Stepper for Fake {
        fn drain(&mut self) -> Vec<SessionEvent> {
            let events = std::mem::take(&mut self.queued);
            self.drained.fetch_add(events.len(), Ordering::SeqCst);
            events
        }

        async fn combat(&mut self, command: CombatCommand) -> Result<CombatReport, SessionError> {
            lock_combats(&self.combats).push(command);
            Ok(match command {
                CombatCommand::Cast(_) => CombatReport::Cast(SpellCastDisposition::NotEnoughMana),
                CombatCommand::Attack(_) => CombatReport::Attack(AttackIntentDisposition::Ok),
                CombatCommand::Stop => CombatReport::Stop(AttackIntentDisposition::Ok),
            })
        }

        async fn step(&mut self, _direction: StepDirection) -> Result<StepOutcome, SessionError> {
            if let Some(calls) = &self.step_calls {
                calls.fetch_add(1, Ordering::SeqCst);
            }
            if let Some(release) = self.step_release.take() {
                release.await.map_err(|_| SessionError::SessionUnusable)?;
            }
            if self.fail_step {
                Err(SessionError::SessionUnusable)
            } else {
                Ok(moved_to(100, 199, 0))
            }
        }

        async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
            let count = self.serves.fetch_add(1, Ordering::SeqCst) + 1;
            if count > self.fail_serve_after {
                return Err(SessionError::SessionUnusable);
            }
            tokio::time::sleep(duration).await;
            Ok(())
        }
    }

    fn lock_combats(combats: &Mutex<Vec<CombatCommand>>) -> MutexGuard<'_, Vec<CombatCommand>> {
        combats.lock().unwrap_or_else(PoisonError::into_inner)
    }

    type Task = (PlayLink, Arc<AtomicUsize>, ClientRuntime);

    fn task(fail_step: bool, fail_serve_after: usize) -> Result<Task, String> {
        let serves = Arc::new(AtomicUsize::new(0));
        let (link, commands, events) = play_channel();
        let runtime = ClientRuntime::new().map_err(|error| error.to_string())?;
        let fake = Fake {
            serves: Arc::clone(&serves),
            fail_step,
            fail_serve_after,
            queued: Vec::new(),
            drained: Arc::new(AtomicUsize::new(0)),
            combats: Arc::new(Mutex::new(Vec::new())),
            step_calls: None,
            step_release: None,
        };
        runtime
            .spawn(run_session(fake, commands, events))
            .map_err(|error| error.to_string())?;
        Ok((link, serves, runtime))
    }

    fn wait_for(link: &PlayLink) -> Option<PlayEvent> {
        for _ in 0..200 {
            if let Some(event) = link.poll() {
                return Some(event);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        None
    }

    #[test]
    fn a_delayed_task_runs_only_one_step_until_the_shell_consumes_it() -> Result<(), String> {
        let (link, commands, outbox) = play_channel();
        let calls = Arc::new(AtomicUsize::new(0));
        let (release, delayed) = tokio::sync::oneshot::channel();
        let runtime = ClientRuntime::new().map_err(|error| error.to_string())?;
        let fake = Fake {
            serves: Arc::new(AtomicUsize::new(0)),
            fail_step: false,
            fail_serve_after: usize::MAX,
            queued: Vec::new(),
            drained: Arc::new(AtomicUsize::new(0)),
            combats: Arc::new(Mutex::new(Vec::new())),
            step_calls: Some(Arc::clone(&calls)),
            step_release: Some(delayed),
        };
        runtime
            .spawn(run_session(fake, commands, outbox))
            .map_err(|error| error.to_string())?;
        assert!(link.request(StepDir::North));
        for _ in 0..200 {
            if calls.load(Ordering::SeqCst) == 1 {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        for _ in 0..100 {
            assert!(!link.request(StepDir::East));
        }
        release
            .send(())
            .map_err(|()| "task exited before release")?;
        for _ in 0..200 {
            if lock(&link.mailbox).stepped.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(lock(&link.mailbox).stepped.is_some());
        assert!(!link.request(StepDir::West));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(matches!(wait_for(&link), Some(PlayEvent::Stepped(_))));
        assert!(link.request(StepDir::West));
        assert!(matches!(wait_for(&link), Some(PlayEvent::Stepped(_))));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        runtime.shutdown(Duration::from_millis(250));
        Ok(())
    }

    #[test]
    fn enqueue_close_race_rolls_back_the_step_and_latches_end() -> Result<(), BatchError> {
        let (link, commands, _outbox) = play_channel();
        assert!(!link.request_step(|| {
            drop(commands);
            Ok(Some(StepDir::North))
        })?);
        assert!(!lock(&link.mailbox).step_pending);
        assert!(matches!(
            link.poll(),
            Some(PlayEvent::Ended(PublicClass::SessionUnavailable))
        ));
        let consumed = std::cell::Cell::new(false);
        assert!(!link.request_step(|| {
            consumed.set(true);
            Ok(Some(StepDir::East))
        })?);
        assert!(!consumed.get());
        assert!(link.poll().is_none());
        Ok(())
    }

    #[test]
    fn an_idle_session_keeps_servicing_liveness() -> Result<(), String> {
        let (link, serves, runtime) = task(false, usize::MAX)?;
        std::thread::sleep(Duration::from_millis(300));
        assert!(serves.load(Ordering::SeqCst) >= 5);
        drop(link);
        runtime.shutdown(Duration::from_millis(250));
        Ok(())
    }

    #[test]
    fn a_hotkey_reaches_the_session_task_in_press_order_and_its_answer_reaches_the_view()
    -> Result<(), String> {
        let serves = Arc::new(AtomicUsize::new(0));
        let combats = Arc::new(Mutex::new(Vec::new()));
        let (link, commands, events) = play_channel();
        let runtime = ClientRuntime::new().map_err(|error| error.to_string())?;
        let fake = Fake {
            serves,
            fail_step: false,
            fail_serve_after: usize::MAX,
            queued: Vec::new(),
            drained: Arc::new(AtomicUsize::new(0)),
            combats: Arc::clone(&combats),
            step_calls: None,
            step_release: None,
        };
        runtime
            .spawn(run_session(fake, commands, events))
            .map_err(|error| error.to_string())?;
        let mut view = view().map_err(|error| format!("{error:?}"))?;
        let spell = NonZeroU32::new(2).ok_or("spell")?;
        let first = NonZeroU32::new(1).ok_or("spell")?;
        view.cast(spell);
        view.cast(first);
        // Past the bar: ignored.
        view.cast(NonZeroU32::new(9).ok_or("spell")?);
        assert_eq!(view.tick(&link), Ok(()));
        for _ in 0..200 {
            assert_eq!(view.tick(&link), Ok(()));
            if lock_combats(&combats).len() == 2
                && view.combat_status().ends_with("Not enough mana")
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(
            *lock_combats(&combats),
            [CombatCommand::Cast(spell), CombatCommand::Cast(first)]
        );
        assert_eq!(
            view.combat_status(),
            "spells 1-4 Spell cast".replace("Spell cast", "Not enough mana")
        );
        runtime.shutdown(Duration::from_millis(250));
        Ok(())
    }

    #[test]
    fn a_tick_sends_a_step_and_applies_its_outcome_without_waiting() -> Result<(), String> {
        let (link, _serves, runtime) = task(false, usize::MAX)?;
        let mut view = view().map_err(|error| format!("{error:?}"))?;
        view.arrow(StepDir::North);
        assert_eq!(view.tick(&link), Ok(()));
        let mut moved = false;
        for _ in 0..200 {
            assert_eq!(view.tick(&link), Ok(()));
            if view.own() == TileCoord::new(100, 199) {
                moved = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(moved);
        runtime.shutdown(Duration::from_millis(250));
        Ok(())
    }

    #[test]
    fn a_step_error_ends_the_task_with_its_public_class() -> Result<(), String> {
        let (link, _serves, runtime) = task(true, usize::MAX)?;
        link.request(StepDir::North);
        assert!(matches!(
            wait_for(&link),
            Some(PlayEvent::Ended(PublicClass::SessionUnavailable))
        ));
        runtime.shutdown(Duration::from_millis(250));
        Ok(())
    }

    #[test]
    fn a_liveness_error_while_idle_returns_to_login() -> Result<(), String> {
        let (link, _serves, runtime) = task(false, 2)?;
        let mut view = view().map_err(|error| format!("{error:?}"))?;
        let mut ended = None;
        for _ in 0..200 {
            if let Err(class) = view.tick(&link) {
                ended = Some(class);
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(ended, Some(PublicClass::SessionUnavailable));
        runtime.shutdown(Duration::from_millis(250));
        Ok(())
    }

    /// Queues pushed events like `Session` does: past the cap, the session fails closed.
    struct Pushing {
        queue: Vec<SessionEvent>,
        sequence: u64,
    }

    impl Stepper for Pushing {
        async fn combat(&mut self, _command: CombatCommand) -> Result<CombatReport, SessionError> {
            Ok(CombatReport::Cast(SpellCastDisposition::NotEnoughMana))
        }

        async fn step(&mut self, _direction: StepDirection) -> Result<StepOutcome, SessionError> {
            Ok(moved_to(100, 199, 0))
        }

        async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
            for _ in 0..200 {
                self.sequence += 1;
                self.queue.push(SessionEvent::WorldSpatial(AppliedDelta {
                    server_sequence: self.sequence,
                    base_revision: self.sequence,
                    new_revision: self.sequence + 1,
                    value: WorldSpatialObservation {
                        content_generation: [0; 32],
                        actor_position: ActorPosition {
                            x: 100,
                            y: 200,
                            floor: 0,
                        },
                    },
                }));
            }
            if self.queue.len() > MAX_QUEUED_EVENTS {
                return Err(SessionError::EventQueueOverflow {
                    limit: MAX_QUEUED_EVENTS,
                });
            }
            tokio::time::sleep(duration).await;
            Ok(())
        }

        fn drain(&mut self) -> Vec<SessionEvent> {
            std::mem::take(&mut self.queue)
        }
    }

    #[test]
    fn pushed_events_are_drained_so_the_queue_never_overflows() -> Result<(), String> {
        let (link, commands, events) = play_channel();
        let runtime = ClientRuntime::new().map_err(|error| error.to_string())?;
        let pushing = Pushing {
            queue: Vec::new(),
            sequence: 0,
        };
        runtime
            .spawn(run_session(pushing, commands, events))
            .map_err(|error| error.to_string())?;
        let mut view = view().map_err(|error| format!("{error:?}"))?;
        let mut pushed = 0;
        for _ in 0..400 {
            while let Some(event) = link.poll() {
                match event {
                    PlayEvent::Pushed(batch) => {
                        pushed += 1;
                        view.apply_pushed(&batch)
                            .map_err(|error| format!("{error:?}"))?;
                    }
                    PlayEvent::Ended(class) => return Err(format!("session ended: {class:?}")),
                    PlayEvent::Stepped(_) | PlayEvent::Combat(_) | PlayEvent::Used(_) => {}
                }
            }
            if pushed > 4 {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(pushed > 4);
        runtime.shutdown(Duration::from_millis(250));
        Ok(())
    }

    fn spatial_event(sequence: u64, x: i32) -> SessionEvent {
        SessionEvent::WorldSpatial(AppliedDelta {
            server_sequence: sequence,
            base_revision: sequence,
            new_revision: sequence + 1,
            value: WorldSpatialObservation {
                content_generation: [0; 32],
                actor_position: ActorPosition {
                    x,
                    y: 200,
                    floor: 0,
                },
            },
        })
    }

    fn overlay_event(sequence: u64, placement: Vec<u8>) -> SessionEvent {
        SessionEvent::WorldObjectOverlay(AppliedDelta {
            server_sequence: sequence,
            base_revision: sequence,
            new_revision: sequence + 1,
            value: entry(placement),
        })
    }

    #[test]
    fn pushed_domain_1_and_2_deltas_move_the_view() -> Result<(), BatchError> {
        let mut view = view()?;
        let mut pushed = Pushed::default();
        pushed.set_marker(entry(key(101, 200, 0, 0)));
        pushed.own = Some(ActorPosition {
            x: 101,
            y: 200,
            floor: 0,
        });
        view.apply_pushed(&pushed)?;
        assert_eq!(view.own(), TileCoord::new(101, 200));
        assert_eq!(view.marker_count(), 2);
        Ok(())
    }

    #[test]
    fn the_entry_door_semantic_placement_draws_a_marker() -> Result<(), BatchError> {
        let door = entry(b"oteryn:cell/entry-door".to_vec());
        let view = PlayView::new(TileCoord::new(1, 0), 0, &[door])?;
        assert_eq!(view.marker_count(), 1);
        assert_eq!(view.scene().sprites().len(), 2);
        Ok(())
    }

    #[test]
    fn streamed_ground_click_walks_but_roof_click_does_not_guess_a_lower_floor()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_session::map_view::{
            MapDefinition, MapItem, MapOrigin, MapTile, MapViewHeader, WorldMapViewSnapshot,
            encode_world_map_snapshot,
        };
        let own = ActorPosition {
            x: 100,
            y: 100,
            floor: -7,
        };
        for (floor, terrain, appearance, expected) in [
            (-7, true, 0, Some(StepDir::East)),
            (-6, true, 0, None),
            (-7, false, 0, None),
            (-7, false, 104, Some(StepDir::East)),
        ] {
            let shift = i32::from(floor - own.floor);
            let payload = encode_world_map_snapshot(&WorldMapViewSnapshot {
                header: MapViewHeader {
                    content_generation: [1; 32],
                    bundle_digest: [2; 32],
                    reset_epoch: 1,
                    origin: own,
                    first_visible_floor: 0,
                },
                tiles: vec![MapTile {
                    position: ActorPosition {
                        x: 101 + shift,
                        y: 100 + shift,
                        floor,
                    },
                    more: false,
                    ground_speed: 150,
                    items: vec![MapItem {
                        definition: if terrain {
                            MapDefinition::Terrain(std::num::NonZeroU32::MIN)
                        } else {
                            MapDefinition::Item(std::num::NonZeroU32::MIN)
                        },
                        count: 1,
                        sub_type: 0,
                        appearance_id: appearance,
                        origin: MapOrigin::DisplayOnly,
                    }],
                }],
            })
            .map_err(|e| format!("{e:?}"))?;
            let mut map = MapView::new([1; 32]).map_err(|e| format!("{e:?}"))?;
            map.replace_snapshot(&payload)
                .map_err(|e| format!("{e:?}"))?;
            let mut view = PlayView::new(TileCoord::new(100, 100), -7, &[])?;
            view.apply_pushed(&Pushed {
                map: Some(Box::new(map)),
                ..Pushed::default()
            })?;
            if appearance == 104 {
                let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
                let mut draw = crate::map_draw::MapDraw::load(&root)?;
                assert!(draw.is_ground(appearance));
                let frame = draw.frame(
                    view.world_map().ok_or("no map")?,
                    &[],
                    &mut oteryn_renderer::AtlasPage::new(),
                    view.scene().view(),
                )?;
                let key = frame.uploads().first().ok_or("no appearance cell")?.0;
                let expected_pixels = draw.cell_rgba(key)?;
                assert_ne!(expected_pixels, crate::world::builtin_rgba()[8192..12288]);
                view.set_map_draw(Arc::new(Mutex::new(draw)))?;
                let uploads = view.take_page_uploads();
                assert_eq!(uploads.len(), 1);
                assert_eq!(uploads[0].rgba, expected_pixels);
                view.reset_page()?;
                let reset = view.take_page_uploads();
                assert_eq!(
                    reset.iter().map(|u| (u.slot, &u.rgba)).collect::<Vec<_>>(),
                    uploads
                        .iter()
                        .map(|u| (u.slot, &u.rgba))
                        .collect::<Vec<_>>()
                );
            }
            view.click(TileCoord::new(101, 100))?;
            assert_eq!(view.next_step(), expected);
            assert_eq!(
                view.combat_line,
                expected.is_none().then_some("That item is out of reach")
            );
            if appearance == 104 {
                view.set_map_draw(Arc::new(Mutex::new(
                    crate::map_draw::MapDraw::placeholders(),
                )))?;
                assert!(view.take_page_uploads().is_empty());
                assert!(
                    view.scene()
                        .sprite_frame()
                        .ok_or("not streamed")?
                        .quads()
                        .batches()
                        .iter()
                        .flatten()
                        .all(|quad| quad.uv == oteryn_renderer::AtlasPage::placeholder_uv())
                );
            }
        }
        Ok(())
    }

    #[test]
    fn streamed_play_uses_server_coordinates_and_not_the_local_start_map()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_session::map_view::{
            MapViewHeader, WorldMapViewSnapshot, encode_world_map_snapshot,
        };
        let own = ActorPosition {
            x: 100,
            y: 100,
            floor: -7,
        };
        let payload = encode_world_map_snapshot(&WorldMapViewSnapshot {
            header: MapViewHeader {
                content_generation: [1; 32],
                bundle_digest: [2; 32],
                reset_epoch: 1,
                origin: own,
                first_visible_floor: 0,
            },
            tiles: vec![],
        })
        .map_err(|e| format!("{e:?}"))?;
        let mut map = MapView::new([1; 32]).map_err(|e| format!("{e:?}"))?;
        map.replace_snapshot(&payload)
            .map_err(|e| format!("{e:?}"))?;
        let mut view = PlayView::new(TileCoord::new(100, 100), -7, &[])?;
        view.apply_pushed(&Pushed {
            map: Some(Box::new(map)),
            ..Pushed::default()
        })?;
        assert_eq!(
            view.scene()
                .sprite_frame()
                .ok_or("not streamed")?
                .quads()
                .quad_count(),
            1
        );
        assert_eq!(view.scene().atlas().width(), 2048);
        view.set_world(Arc::new(World::builtin()?))?;
        assert_eq!(view.anchor, TileCoord::new(0, 0));
        assert!(view.map_floor.is_none());
        assert!(view.minimap_image().is_none());
        assert_eq!(
            view.scene()
                .sprite_frame()
                .ok_or("lost streamed view")?
                .quads()
                .quad_count(),
            1
        );
        view.scene.select_tile(TileCoord::new(100, 100))?;
        assert_eq!(view.take_page_uploads().len(), 1);
        view.reset_page()?;
        assert_eq!(view.take_page_uploads().len(), 1);
        Ok(())
    }

    #[test]
    fn undrained_map_mailbox_keeps_complete_latest_state_across_step_ordering()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_session::map_view::{
            MapViewHeader, WorldMapViewSnapshot, encode_world_map_snapshot,
        };
        let (link, _commands, outbox) = play_channel();
        let map = |x| -> Result<MapView, String> {
            let snapshot = WorldMapViewSnapshot {
                header: MapViewHeader {
                    content_generation: [1; 32],
                    bundle_digest: [2; 32],
                    reset_epoch: 1,
                    origin: ActorPosition {
                        x,
                        y: 100,
                        floor: -7,
                    },
                    first_visible_floor: 0,
                },
                tiles: vec![],
            };
            let bytes = encode_world_map_snapshot(&snapshot).map_err(|e| format!("{e:?}"))?;
            let mut map = MapView::new([1; 32]).map_err(|e| format!("{e:?}"))?;
            map.replace_snapshot(&bytes).map_err(|e| format!("{e:?}"))?;
            Ok(map)
        };
        for revision in 1..=1000 {
            outbox.map(revision, &map(revision as i32)?);
        }
        let before = std::mem::take(&mut lock(&outbox.0).pushed);
        lock(&outbox.0).before.absorb(before);
        outbox.map(1001, &map(1001)?);
        // Same revision must not copy over an already published projection.
        outbox.map(1001, &map(2000)?);
        for expected in [1000, 1001] {
            let Some(PlayEvent::Pushed(pushed)) = link.poll() else {
                return Err("missing map".into());
            };
            assert_eq!(
                pushed.map.and_then(|map| map.header()).map(|h| h.origin.x),
                Some(expected)
            );
        }
        assert!(link.poll().is_none());
        Ok(())
    }

    #[test]
    fn an_undrained_mailbox_stays_bounded_and_keeps_the_latest_state() {
        let (link, _commands, outbox) = play_channel();
        for n in 0..5_000u64 {
            let events = vec![
                spatial_event(n, i32::try_from(n).unwrap_or(0)),
                overlay_event(n, n.to_be_bytes().to_vec()),
                overlay_event(n, b"same".to_vec()),
            ];
            outbox.push(events);
        }
        let event = link.poll();
        assert!(matches!(event, Some(PlayEvent::Pushed(_))));
        let Some(PlayEvent::Pushed(pushed)) = event else {
            return;
        };
        assert_eq!(pushed.own.map(|position| position.x), Some(4_999));
        assert!(pushed.markers.len() <= MAX_PUSHED_MARKERS);
        assert!(link.poll().is_none());
    }

    #[test]
    fn pushes_older_than_a_step_outcome_are_delivered_before_it() {
        let (link, _commands, outbox) = play_channel();
        outbox.push(vec![spatial_event(1, 50)]);
        outbox.stepped(moved_to(100, 199, 0));
        outbox.push(vec![spatial_event(3, 60)]);
        assert!(matches!(
            link.poll(),
            Some(PlayEvent::Pushed(Pushed { own: Some(position), .. })) if position.x == 50
        ));
        assert!(matches!(link.poll(), Some(PlayEvent::Stepped(_))));
        assert!(matches!(
            link.poll(),
            Some(PlayEvent::Pushed(Pushed { own: Some(position), .. })) if position.x == 60
        ));
        assert!(link.poll().is_none());
    }
}
