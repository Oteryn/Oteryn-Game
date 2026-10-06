//! Play view after admission (N2N3-1): a placeholder ground grid centred on the own actor, one
//! marker per overlay object, and the walk over `Session::step`.
//!
//! The view holds no authority. It is built from the join snapshot, and only a step outcome's
//! domain-1 and domain-2 deltas move it. Real map tiles arrive with the map track.

use crate::GameplayEntryError;
use crate::input::{ClickWalk, StepDir, StepResult};
use crate::scene::PlaceholderScene;
use oteryn_platform_client::native_login::PublicClass;
use oteryn_renderer::{BatchError, TileCoord};
use oteryn_session::{
    JoinSnapshot, Session, SessionError, SessionStream, StepDirection, StepDisposition,
    StepOutcome, WorldObjectOverlayEntry,
};
use std::collections::BTreeMap;

/// Byte length of a bundle placement key (`OTERYN_WORLD_BUNDLE_FORMAT_V1.md` §7), and of the key
/// prefixed by the 32-byte bundle digest.
const KEY_BYTES: usize = 8;
const DIGEST_AND_KEY_BYTES: usize = 32 + KEY_BYTES;

/// The tile and floor a placement names: the last eight bytes as the big-endian key
/// `x << 32 | y << 16 | (-floor) << 8 | ordinal`. Other shapes name no tile and draw no marker.
fn placement_tile(placement: &[u8]) -> Option<(TileCoord, i16)> {
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
pub const fn login_error(_error: &SessionError) -> GameplayEntryError {
    GameplayEntryError::Rejected(PublicClass::SessionUnavailable)
}

#[derive(Debug, Clone)]
pub struct PlayView {
    own: TileCoord,
    floor: i16,
    markers: BTreeMap<Vec<u8>, TileCoord>,
    scene: PlaceholderScene,
    walk: ClickWalk,
    /// An arrow key waiting for the next step; the latest press wins.
    key_step: Option<StepDir>,
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
        let mut view = Self {
            own,
            floor,
            markers: BTreeMap::new(),
            scene: PlaceholderScene::centered_on(own, &[])?,
            walk: ClickWalk::new(),
            key_step: None,
        };
        for entry in overlay {
            view.set_marker(entry);
        }
        view.rebuild()?;
        Ok(view)
    }

    #[must_use]
    pub const fn own(&self) -> TileCoord {
        self.own
    }

    #[must_use]
    pub const fn scene(&self) -> &PlaceholderScene {
        &self.scene
    }

    #[must_use]
    pub fn marker_count(&self) -> usize {
        self.markers.len()
    }

    /// A click on `tile`: a drawn object or actor is selected, any other tile becomes the walk
    /// goal.
    pub fn click(&mut self, tile: TileCoord) -> Result<(), BatchError> {
        if self.scene.select_tile(tile)?.is_none() {
            self.walk.set_goal(tile);
        }
        Ok(())
    }

    pub const fn arrow(&mut self, direction: StepDir) {
        self.key_step = Some(direction);
    }

    /// The step to send now: an arrow key first, else the next step toward the click goal.
    pub fn next_step(&mut self) -> Option<StepDir> {
        if let Some(direction) = self.key_step.take() {
            return Some(direction);
        }
        self.walk.next_step(self.own)
    }

    /// Applies one step outcome: the domain-1 delta moves the own actor, the domain-2 delta
    /// moves a marker. A refused step ends the click walk.
    pub fn apply(&mut self, outcome: &StepOutcome) -> Result<(), BatchError> {
        self.walk
            .on_result(if outcome.disposition == StepDisposition::Moved {
                StepResult::Moved
            } else {
                StepResult::Refused
            });
        if let Some(delta) = &outcome.world_spatial_delta {
            let position = delta.value.actor_position;
            self.own = TileCoord::new(position.x, position.y);
            self.floor = position.floor;
        }
        if let Some(delta) = &outcome.world_object_overlay_delta {
            self.set_marker(&delta.value);
        }
        self.rebuild()
    }

    fn set_marker(&mut self, entry: &WorldObjectOverlayEntry) {
        match placement_tile(&entry.placement) {
            Some((tile, floor)) if floor == self.floor => {
                self.markers.insert(entry.placement.clone(), tile);
            }
            _ => {
                self.markers.remove(&entry.placement);
            }
        }
    }

    fn rebuild(&mut self) -> Result<(), BatchError> {
        let markers = self.markers.values().copied().collect::<Vec<_>>();
        self.scene = PlaceholderScene::centered_on(self.own, &markers)?;
        Ok(())
    }

    /// Sends the next step, if one is due, and applies its outcome; `true` when the view
    /// changed. Any session error returns to login with its public class; a view error is a
    /// render failure.
    pub async fn pump<S: SessionStream>(
        &mut self,
        session: &mut Session<S>,
    ) -> Result<bool, GameplayEntryError> {
        let Some(direction) = self.next_step() else {
            return Ok(false);
        };
        let outcome = session
            .step_retrying(step_direction(direction))
            .await
            .map_err(|error| login_error(&error))?;
        self.apply(&outcome)
            .map_err(|_error| GameplayEntryError::Rejected(PublicClass::SessionUnavailable))?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_session::{ActorPosition, AppliedDelta, CommandStatus, WorldSpatialObservation};

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

    #[test]
    fn snapshot_builds_a_view_centred_on_the_own_position_with_overlay_markers()
    -> Result<(), BatchError> {
        let view = view()?;
        // Only the marker on the own floor and with a decodable placement is drawn.
        assert_eq!(view.marker_count(), 1);
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
    fn arrow_key_steps_without_a_goal() -> Result<(), BatchError> {
        let mut view = view()?;
        view.arrow(StepDir::North);
        assert_eq!(view.next_step(), Some(StepDir::North));
        assert_eq!(view.next_step(), None);
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
        assert_eq!(
            login_error(&error),
            GameplayEntryError::Rejected(PublicClass::SessionUnavailable)
        );
    }
}
