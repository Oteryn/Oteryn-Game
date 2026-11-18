//! Readbacks are scoped to the current play session and share the ordered screenshot writer.
use super::*;
use crate::play::screenshot::{CaptureAction, ScreenshotBatch};
use std::time::Instant;

#[derive(Default)]
pub(in crate::bevy_shell) struct State {
    generation: Option<u64>,
}

pub(super) struct Completed {
    generation: u64,
    game_only: bool,
    at: Instant,
    result: AutomaticResult,
}

enum AutomaticResult {
    Buffered(io::Result<Pixels>),
    Saved(io::Result<Vec<PathBuf>>, ScreenshotBatch),
}

impl Completed {
    pub(super) fn apply(self, owner: &mut HostOwner) {
        if !current(owner, self.generation, self.game_only) {
            return;
        }
        match self.result {
            AutomaticResult::Buffered(Ok(frame)) => {
                owner.settings.buffer_screenshot(self.at, frame)
            }
            AutomaticResult::Saved(Ok(paths), _) => {
                owner.status = format!("Saved {} screenshot(s)", paths.len())
            }
            AutomaticResult::Saved(Err(error), batch) => {
                owner.settings.restore_screenshot_batch(batch);
                owner.status = format!("Screenshot failed: {error}");
            }
            AutomaticResult::Buffered(Err(error)) => {
                owner.status = format!("Screenshot failed: {error}");
            }
        }
    }
}

fn current(owner: &HostOwner, generation: u64, game_only: bool) -> bool {
    owner.generation() == generation
        && owner.play.is_some()
        && owner.settings.preferences.screenshots.effective().game_only == game_only
}

fn prepare(owner: &mut HostOwner, at: Instant) -> CaptureAction {
    owner
        .settings
        .prepare_screenshots(at, owner.play.as_ref().map(|play| &play.link))
}

#[allow(
    clippy::too_many_arguments,
    reason = "Shares the frame readback with the F12 consumer"
)]
pub(in crate::bevy_shell) fn observe(
    commands: &mut Commands,
    owner: &mut HostOwner,
    viewport: WorldViewport,
    state: &mut State,
    captures: &Query<Entity, With<Screenshot>>,
    clock: &Time<Real>,
    writes: &Writes,
    manual: bool,
) -> Option<Entity> {
    let generation = owner.play.as_ref().map(|_| owner.generation());
    if state.generation != generation {
        owner.settings.reset_screenshot_session();
        state.generation = generation;
    }
    let existing = captures.iter().next();
    let Some(generation) = generation else {
        return existing;
    };
    if manual {
        owner.settings.request_screenshot();
        owner.status = "Capturing screenshot…".into();
    }
    let at = clock.last_update().unwrap_or(clock.startup());
    let action = prepare(owner, at);
    if matches!(action, CaptureAction::Idle)
        || writes.automatic
        || writes.is_full()
        || existing.is_some()
    {
        return existing;
    }
    let game_only = owner.settings.preferences.screenshots.effective().game_only;
    let entity = commands
        .spawn(Screenshot::primary_window())
        .observe(
            move |event: On<ScreenshotCaptured>,
                  mut owner: NonSendMut<HostOwner>,
                  mut writes: NonSendMut<Writes>,
                  clock: Res<Time<Real>>| {
                if !current(&owner, generation, game_only) || writes.is_full() {
                    return;
                }
                let at = clock.last_update().unwrap_or(clock.startup());
                let batch = match prepare(&mut owner, at) {
                    CaptureAction::Idle => return,
                    CaptureAction::Buffer => None,
                    CaptureAction::Save => Some(owner.settings.submit_screenshot(at)),
                };
                let image = event.image.clone();
                writes.automatic = true;
                writes.queued.push_back(Box::new(move || {
                    let pixels = pixels(image, game_only.then_some(viewport));
                    let result = if let Some(batch) = batch {
                        AutomaticResult::Saved(pixels.and_then(|pixels| batch.save(&pixels)), batch)
                    } else {
                        AutomaticResult::Buffered(pixels)
                    };
                    Completion::Automatic(Completed {
                        generation,
                        game_only,
                        at,
                        result,
                    })
                }));
            },
        )
        .id();
    Some(entity)
}
