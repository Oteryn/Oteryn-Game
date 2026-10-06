//! Play view after admission (N2N3-1): a placeholder ground grid centred on the own actor, one
//! marker per overlay object, and the walk over `Session::step`.
//!
//! The view holds no authority. It is built from the join snapshot, and only a step outcome's
//! domain-1 and domain-2 deltas move it. Real map tiles arrive with the map track.

use crate::input::{ClickWalk, StepDir, StepResult};
use crate::scene::PlaceholderScene;
use oteryn_platform_client::native_login::PublicClass;
use oteryn_renderer::{BatchError, TileCoord};
use oteryn_session::{
    JoinSnapshot, Session, SessionError, SessionStream, StepDirection, StepDisposition,
    StepOutcome, WorldObjectOverlayEntry,
};
use std::collections::BTreeMap;
use std::future::Future;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// How long the session task serves liveness probes while no step is pending; well inside the
/// server's probe cadence.
const IDLE_SLICE: Duration = Duration::from_millis(20);

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
pub const fn public_class(_error: &SessionError) -> PublicClass {
    PublicClass::SessionUnavailable
}

#[derive(Debug, Clone)]
pub struct PlayView {
    own: TileCoord,
    floor: i16,
    /// Every decodable overlay entry with its floor; the current floor selects what is drawn.
    markers: BTreeMap<Vec<u8>, (TileCoord, i16)>,
    scene: PlaceholderScene,
    walk: ClickWalk,
    /// A step sent to the session task whose outcome has not arrived.
    in_flight: bool,
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
            in_flight: false,
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
        if self.in_flight {
            return None;
        }
        let direction = self.pending_step();
        self.in_flight = direction.is_some();
        direction
    }

    fn pending_step(&mut self) -> Option<StepDir> {
        if let Some(direction) = self.key_step.take() {
            return Some(direction);
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
        self.scene = PlaceholderScene::centered_on(self.own, &markers)?;
        Ok(())
    }

    /// One shell tick, never waiting on the network: applies every outcome the session task has
    /// delivered, then hands it the next step if one is due. A session end returns to login with
    /// its public class; a view error is a render failure.
    pub fn tick(&mut self, link: &PlayLink) -> Result<(), PublicClass> {
        while let Some(event) = link.poll() {
            match event {
                PlayEvent::Stepped(outcome) => self
                    .apply(&outcome)
                    .map_err(|_error| PublicClass::SessionUnavailable)?,
                PlayEvent::Ended(class) => return Err(class),
            }
        }
        if let Some(direction) = self.next_step() {
            link.request(direction);
        }
        Ok(())
    }
}

/// What the session task reports to the shell.
#[derive(Debug)]
pub enum PlayEvent {
    Stepped(Box<StepOutcome>),
    /// The session ended; return to login with this public class.
    Ended(PublicClass),
}

/// The shell's end of the session task: steps out, events in, neither ever blocking.
#[derive(Debug)]
pub struct PlayLink {
    commands: UnboundedSender<StepDir>,
    events: Receiver<PlayEvent>,
}

impl PlayLink {
    pub fn request(&self, direction: StepDir) {
        // A closed task has already reported or will report `Ended`.
        let _ = self.commands.send(direction);
    }

    pub fn poll(&self) -> Option<PlayEvent> {
        match self.events.try_recv() {
            Ok(event) => Some(event),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                Some(PlayEvent::Ended(PublicClass::SessionUnavailable))
            }
        }
    }
}

/// The session calls the task needs; implemented by `Session`.
pub trait Stepper: Send {
    fn step(
        &mut self,
        direction: StepDirection,
    ) -> impl Future<Output = Result<StepOutcome, SessionError>> + Send;
    fn serve(
        &mut self,
        duration: Duration,
    ) -> impl Future<Output = Result<(), SessionError>> + Send;
}

impl<S: SessionStream + Send> Stepper for Session<S> {
    fn step(
        &mut self,
        direction: StepDirection,
    ) -> impl Future<Output = Result<StepOutcome, SessionError>> + Send {
        self.step_retrying(direction)
    }

    fn serve(
        &mut self,
        duration: Duration,
    ) -> impl Future<Output = Result<(), SessionError>> + Send {
        self.service_liveness(duration)
    }
}

/// The channel pair between the shell and `run_session`.
pub fn play_channel() -> (
    PlayLink,
    UnboundedReceiver<StepDir>,
    std::sync::mpsc::Sender<PlayEvent>,
) {
    let (commands, command_rx) = tokio::sync::mpsc::unbounded_channel();
    let (event_tx, events) = std::sync::mpsc::channel();
    (PlayLink { commands, events }, command_rx, event_tx)
}

/// The session task, run on the client runtime: sends requested steps and, while none is
/// pending, keeps servicing liveness so an idle session is not dropped. It ends with the first
/// session error (reported with its public class) or when the shell drops its link.
pub async fn run_session<T: Stepper>(
    mut session: T,
    mut commands: UnboundedReceiver<StepDir>,
    events: std::sync::mpsc::Sender<PlayEvent>,
) {
    loop {
        let ended = match commands.try_recv() {
            Ok(direction) => match session.step(step_direction(direction)).await {
                Ok(outcome) => {
                    if events.send(PlayEvent::Stepped(Box::new(outcome))).is_err() {
                        return;
                    }
                    continue;
                }
                Err(error) => error,
            },
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                match session.serve(IDLE_SLICE).await {
                    Ok(()) => continue,
                    Err(error) => error,
                }
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => return,
        };
        let _ = events.send(PlayEvent::Ended(public_class(&ended)));
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_client_runtime::ClientRuntime;
    use oteryn_session::{ActorPosition, AppliedDelta, CommandStatus, WorldSpatialObservation};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

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

    struct Fake {
        serves: Arc<AtomicUsize>,
        fail_step: bool,
        fail_serve_after: usize,
    }

    impl Stepper for Fake {
        async fn step(&mut self, _direction: StepDirection) -> Result<StepOutcome, SessionError> {
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

    type Task = (PlayLink, Arc<AtomicUsize>, ClientRuntime);

    fn task(fail_step: bool, fail_serve_after: usize) -> Result<Task, String> {
        let serves = Arc::new(AtomicUsize::new(0));
        let (link, commands, events) = play_channel();
        let runtime = ClientRuntime::new().map_err(|error| error.to_string())?;
        let fake = Fake {
            serves: Arc::clone(&serves),
            fail_step,
            fail_serve_after,
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
    fn an_idle_session_keeps_servicing_liveness() -> Result<(), String> {
        let (link, serves, runtime) = task(false, usize::MAX)?;
        std::thread::sleep(Duration::from_millis(300));
        assert!(serves.load(Ordering::SeqCst) >= 5);
        drop(link);
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
}
