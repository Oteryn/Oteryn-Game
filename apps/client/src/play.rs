//! Play view after admission (N2N3-1): the map around the own actor, one marker per overlay
//! object, and the walk over `Session::step`.
//!
//! The view holds no authority. It is built from the join snapshot, and only a step outcome's
//! domain-1 and domain-2 deltas move it. The map is the start area anchored at the join
//! position (CLIENT-VIS-1 milestone 1) until positions are map positions.

use crate::combat_input::{CombatCommand, CombatReport};
use crate::input::{ClickWalk, StepDir, StepResult, TargetKind, Targetable};
use crate::map::World;
use crate::scene::Scene;
use crate::spell::SPELL_SLOTS;
use oteryn_platform_client::native_login::PublicClass;
use oteryn_renderer::{BatchError, TileCoord};
use oteryn_session::{
    ActorPosition, EntityKind, EntityRef, JoinSnapshot, LogoutResult, SessionError, SessionEvent,
    StepDirection, StepDisposition, StepOutcome, WorldObjectOverlayEntry, WorldSpatialEntity,
};
use std::collections::{BTreeMap, VecDeque};
use std::num::NonZeroU32;
use std::sync::Arc;

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

/// How a play session ended: the server ended it (back to login with this public class), or the
/// player logged out and the server committed it (back to character selection).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayEnd {
    Ended(PublicClass),
    LoggedOut,
}

/// The status line of a refused logout. `None` (the server refused the command before its logout
/// authority) and `BUSY` both ask the player to try again; `IN_FIGHT` names the whole seconds
/// left, rounded up.
#[must_use]
pub fn logout_refusal_text(result: Option<LogoutResult>) -> String {
    match result {
        Some(LogoutResult::InFight { retry_after_ms }) => format!(
            "You may not logout during a fight ({} s)",
            retry_after_ms.div_ceil(1_000)
        ),
        Some(LogoutResult::Busy | LogoutResult::Accepted) | None => {
            "You cannot logout now, try again".to_owned()
        }
    }
}

/// One input waiting for its turn on the session task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Input {
    Arrow(StepDir),
    Cast(NonZeroU32),
    Attack(EntityRef),
    Logout,
}

/// Most inputs held while the session task is busy; a later one is dropped.
const MAX_QUEUED_INPUTS: usize = 16;

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
    /// Arrow, hotkey and click input not yet sent, in the order it was given.
    inputs: VecDeque<Input>,
    /// The other entities the server reports as visible.
    entities: Vec<WorldSpatialEntity>,
    /// The last combat line the server's answer produced, shown until the next one.
    combat_line: Option<&'static str>,
    /// The last refused logout's line, shown until the next combat line or logout answer.
    logout_line: Option<String>,
}

impl PlayView {
    pub fn from_join(snapshot: &JoinSnapshot) -> Result<Self, BatchError> {
        let position = snapshot.world_spatial.actor_position;
        Self::new(
            TileCoord::new(position.x, position.y),
            position.floor,
            &snapshot.world_object_overlay,
        )
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
            inputs: VecDeque::new(),
            entities: Vec::new(),
            combat_line: None,
            logout_line: None,
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

    pub const fn map_center(&self) -> (i32, i32, i16) {
        (self.own.x, self.own.y, self.floor)
    }

    #[must_use]
    pub const fn facing(&self) -> StepDir {
        self.facing
    }

    #[must_use]
    pub const fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Draws `world` with the current own position on the start tile and the current floor on
    /// the decoded map floor.
    pub fn set_world(&mut self, world: Arc<World>) -> Result<(), BatchError> {
        self.world = world;
        self.map_floor = Some(self.world.center().2);
        self.anchor = TileCoord::new(0, 0);
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
        match self.scene.select_tile(tile)? {
            None => self.walk.set_goal(tile),
            Some(Targetable {
                entity: Some(target),
                ..
            }) => self.queue(Input::Attack(target)),
            Some(_) => {}
        }
        Ok(())
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

    /// The logout chord: at most one logout waits for the next tick.
    pub fn logout(&mut self) {
        self.queue(Input::Logout);
    }

    /// The minimal spell bar: its hotkeys and the last combat, cast or refused logout line.
    #[must_use]
    pub fn combat_status(&self) -> String {
        let line = self
            .logout_line
            .as_deref()
            .or(self.combat_line)
            .unwrap_or("");
        format!("spells 1-{SPELL_SLOTS} {line}")
            .trim_end()
            .to_owned()
    }

    /// Shows the server's answer to a combat command.
    pub fn apply_combat(&mut self, report: CombatReport) {
        self.combat_line = report.text();
        self.logout_line = None;
    }

    /// Shows why the server refused a logout; the session keeps playing.
    pub fn apply_logout_refusal(&mut self, result: Option<LogoutResult>) {
        self.logout_line = Some(logout_refusal_text(result));
    }

    /// An arrow key: the latest press wins over an arrow still waiting at the end of the queue.
    pub fn arrow(&mut self, direction: StepDir) {
        if matches!(self.inputs.back(), Some(Input::Arrow(_))) {
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

    /// Applies what the server pushed outside any step, coalesced: the latest own position and
    /// the latest entry per placement. Other domains are not drawn by this view.
    pub fn apply_pushed(&mut self, pushed: &Pushed) -> Result<(), BatchError> {
        if let Some(position) = pushed.own {
            self.place(position);
        }
        if let Some(entities) = &pushed.entities {
            self.entities.clone_from(entities);
        }
        for entry in &pushed.markers {
            self.set_marker(entry);
        }
        self.rebuild()
    }

    fn place(&mut self, position: ActorPosition) {
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
        self.scene = Scene::centered_with(
            Arc::clone(&self.world),
            (self.map_floor == Some(self.floor)).then_some(self.anchor),
            self.own,
            self.facing,
            &markers,
            &entities,
        )?;
        Ok(())
    }

    /// One shell tick, never waiting on the network: applies every outcome the session task has
    /// delivered, then hands it the next step if one is due. A session end returns to login with
    /// its public class, a committed logout to character selection; a view error is a render
    /// failure.
    pub fn tick(&mut self, link: &PlayLink) -> Result<(), PlayEnd> {
        const UNAVAILABLE: PlayEnd = PlayEnd::Ended(PublicClass::SessionUnavailable);
        while let Some(event) = link.poll() {
            match event {
                PlayEvent::Stepped(outcome) => {
                    self.apply(&outcome).map_err(|_error| UNAVAILABLE)?;
                }
                PlayEvent::Pushed(pushed) => {
                    self.apply_pushed(&pushed).map_err(|_error| UNAVAILABLE)?;
                }
                PlayEvent::Combat(report) => self.apply_combat(report),
                PlayEvent::LogoutRefused(result) => self.apply_logout_refusal(result),
                PlayEvent::LoggedOut => return Err(PlayEnd::LoggedOut),
                PlayEvent::Ended(class) => return Err(PlayEnd::Ended(class)),
            }
        }
        // One chronological queue: each input goes out in the order it was given. A step still
        // in flight holds back the arrow and what was given after it.
        while let Some(&input) = self.inputs.front() {
            match input {
                Input::Cast(spell) => link.request_combat(CombatCommand::Cast(spell)),
                Input::Attack(target) => link.request_combat(CombatCommand::Attack(target)),
                Input::Logout => link.request_logout(),
                Input::Arrow(_) => match self.send_step().map_err(|_error| UNAVAILABLE)? {
                    Some(direction) => link.request(direction),
                    None => break,
                },
            }
            if !matches!(input, Input::Arrow(_)) {
                self.inputs.pop_front();
            }
        }
        if self.inputs.is_empty()
            && let Some(direction) = self.send_step().map_err(|_error| UNAVAILABLE)?
        {
            link.request(direction);
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

mod mailbox;
mod session_loop;

// Event reduction remains in the play facade until the mechanic-by-mechanic split. The mailbox
// phase only owns bounded delivery and calls this reducer with the batch it received.
fn coalesce_pushed(pushed: &mut Pushed, events: Vec<SessionEvent>) {
    for event in events {
        match event {
            SessionEvent::WorldSpatial(delta) => {
                pushed.own = Some(delta.value.actor_position);
            }
            SessionEvent::WorldSpatialEntities(delta) => {
                pushed.own = Some(delta.value.actor_position);
            }
            SessionEvent::WorldObjectOverlay(delta) => pushed.set_marker(delta.value),
            _ => {}
        }
    }
}

pub use mailbox::{Outbox, PlayEvent, PlayLink, Pushed, play_channel};
pub use session_loop::{Stepper, run_session};

#[cfg(test)]
mod tests;
