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
    manual: bool,
}

enum AutomaticResult {
    Buffered(io::Result<Pixels>),
    Saved(io::Result<Vec<PathBuf>>, ScreenshotBatch, bool),
}

impl Completed {
    pub(super) fn apply(self, owner: &mut HostOwner) {
        let current = current(owner, self.generation, self.game_only);
        if owner.generation() != self.generation || (!self.manual && !current) {
            return;
        }
        match self.result {
            AutomaticResult::Buffered(Ok(frame)) => {
                owner.settings.buffer_screenshot(self.at, frame)
            }
            AutomaticResult::Saved(Ok(paths), _, _) => {
                owner.status = format!("Saved {} screenshot(s)", paths.len())
            }
            AutomaticResult::Saved(Err(error), batch, consumed) => {
                if current && consumed {
                    owner.settings.restore_screenshot_batch(batch);
                }
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
    occupied: bool,
    clock: &Time<Real>,
    writes: &Writes,
    manual: Option<&PlayerCapture>,
) -> Option<Entity> {
    let generation = owner.play.as_ref().map(|_| owner.generation());
    if state.generation != generation {
        owner.settings.reset_screenshot_session();
        state.generation = generation;
    }
    let generation = generation?;
    let at = clock.last_update().unwrap_or(clock.startup());
    let automatic = prepare(owner, at);
    let action = if manual.is_some() {
        CaptureAction::Save
    } else {
        automatic
    };
    if matches!(action, CaptureAction::Idle) || writes.automatic || writes.is_full() || occupied {
        return None;
    }
    let game_only = owner.settings.preferences.screenshots.effective().game_only;
    let crop = game_only.then_some(viewport);
    let directory = manual.map(|request| request.directory.clone());
    if directory.is_some() {
        owner.status = "Capturing screenshot…".into();
    }
    let entity = commands
        .spawn(Screenshot::primary_window())
        .observe(
            move |event: On<ScreenshotCaptured>,
                  mut owner: NonSendMut<HostOwner>,
                  mut writes: NonSendMut<Writes>,
                  clock: Res<Time<Real>>| {
                if owner.generation() != generation {
                    return;
                }
                let manual = directory.is_some();
                let same_crop = current(&owner, generation, game_only);
                if writes.is_full() {
                    if manual {
                        owner.status = "Screenshot failed: write queue is full".into();
                    }
                    return;
                }
                if !manual && !same_crop {
                    return;
                }
                let at = clock.last_update().unwrap_or(clock.startup());
                let batch = match action {
                    CaptureAction::Idle => return,
                    CaptureAction::Buffer => None,
                    CaptureAction::Save => {
                        if let Some(directory) = &directory {
                            if !same_crop {
                                let image = event.image.clone();
                                let directory = directory.clone();
                                writes.push(Box::new(move || {
                                    Completion::Manual(generation, save(image, &directory, crop))
                                }));
                                return;
                            }
                            Some(
                                owner
                                    .settings
                                    .manual_screenshot_batch(at, directory.clone()),
                            )
                        } else {
                            Some((owner.settings.submit_screenshot(at), true))
                        }
                    }
                };
                let image = event.image.clone();
                writes.automatic = true;
                writes.queued.push_back(Box::new(move || {
                    let pixels = pixels(image, crop);
                    let result = if let Some((batch, consumed)) = batch {
                        AutomaticResult::Saved(
                            pixels.and_then(|pixels| batch.save(&pixels)),
                            batch,
                            consumed,
                        )
                    } else {
                        AutomaticResult::Buffered(pixels)
                    };
                    Completion::Automatic(Completed {
                        generation,
                        game_only,
                        at,
                        result,
                        manual,
                    })
                }));
            },
        )
        .id();
    Some(entity)
}
