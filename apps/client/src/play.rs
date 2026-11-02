//! Play view after admission (N2N3-1): the map around the own actor, one marker per overlay
//! object, and the walk over `Session::step`.
//!
//! The view holds no authority. It is built from the join snapshot, and only a step outcome's
//! domain-1 and domain-2 deltas move it. The map is the start area anchored at the join
//! position (CLIENT-VIS-1 milestone 1) until positions are map positions.

use crate::input::{ClickWalk, StepDir};
use crate::map::World;
use crate::scene::Scene;
use feature_local_reference_preferences::settings::LocalPlayState;
use oteryn_platform_client::native_login::PublicClass;
use oteryn_renderer::{BatchError, TileCoord};
use oteryn_session::{
    EntityRef, JoinSnapshot, SessionError, SessionEvent, WorldObjectOverlayEntry,
    WorldSpatialEntity,
};
use std::collections::{BTreeMap, VecDeque};
use std::num::NonZeroU32;

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

/// One input waiting for its turn on the session task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Input {
    Arrow(StepDir),
    Cast(NonZeroU32),
    Attack(EntityRef),
    Logout,
}

const MAX_QUEUED_INPUTS: usize = 16;

#[derive(Debug, Clone)]
pub struct PlayView {
    own: TileCoord,
    /// The direction of the latest step sent; the player is drawn facing it.
    facing: StepDir,
    floor: i16,
    /// Every decodable overlay entry with its floor; the current floor selects what is drawn.
    markers: BTreeMap<Vec<u8>, (TileCoord, i16)>,
    local: LocalPlayState,
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
    /// Recent server messages in their validated delivery order, owned by this connection.
    server_messages: VecDeque<oteryn_session::AppliedDelta<oteryn_session::ServerMessage>>,
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
        let local = LocalPlayState::new(World::builtin()?);
        let anchor = TileCoord::new(0, 0);
        let mut view = Self {
            own,
            facing: StepDir::South,
            floor,
            markers: BTreeMap::new(),
            scene: Scene::centered_on(local.world(), None, own, StepDir::South, &[])?,
            local,
            anchor,
            map_floor: None,
            walk: ClickWalk::new(),
            in_flight: false,
            inputs: VecDeque::new(),
            entities: Vec::new(),
            combat_line: None,
            logout_line: None,
            server_messages: VecDeque::new(),
        };
        for entry in overlay {
            view.set_marker(entry);
        }
        view.rebuild()?;
        Ok(view)
    }

    /// Appends `input` unless the same one already waits (a repeated press of a slot or click of
    /// a target is one command) or the queue is full.
    fn queue(&mut self, input: Input) {
        if self.inputs.len() < MAX_QUEUED_INPUTS && !self.inputs.contains(&input) {
            self.inputs.push_back(input);
        }
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
                Input::Cast(spell) => feature_local_spell_action_bar::input::send_cast(link, spell),
                Input::Attack(target) => {
                    feature_0011_world_interaction::input::send_attack(link, target)
                }
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
}

mod mailbox;
mod session_loop;

include!(concat!(env!("OUT_DIR"), "/play_features.rs"));

use feature_0013_paced_movement::protocol::step_direction;
pub use feature_0021_logout::protocol::logout_refusal_text;
pub use feature_0092_zones_nologout::state::ServerMessageInbox;
pub use mailbox::{Outbox, PlayEvent, PlayLink, Pushed, play_channel};
pub use session_loop::{Stepper, run_session};

#[cfg(test)]
mod tests;
