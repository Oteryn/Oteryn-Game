//! Deferred Bevy pixels feed the same event and bounded backlog owner as native capture.
use super::*;
use crate::play::screenshot::CaptureSource;
use std::time::Instant;
#[derive(Default)]
pub(in crate::bevy_shell) struct State {
    generation: Option<u64>,
}
#[derive(Component)]
pub(in crate::bevy_shell) struct Capture {
    generation: u64,
}
struct Deferred;
impl CaptureSource for Deferred {
    fn capture(&mut self, _: bool) -> io::Result<Pixels> {
        Err(io::ErrorKind::WouldBlock.into())
    }
}
struct Rendered<'a> {
    image: &'a Image,
    viewport: WorldViewport,
}
impl CaptureSource for Rendered<'_> {
    fn capture(&mut self, game_only: bool) -> io::Result<Pixels> {
        let mut image = self
            .image
            .clone()
            .try_into_dynamic()
            .map_err(io::Error::other)?;
        if game_only {
            let end = self
                .viewport
                .origin
                .checked_add(self.viewport.size)
                .ok_or_else(|| io::Error::other("Invalid game viewport"))?;
            if self.viewport.size.min_element() == 0
                || end.x > image.width()
                || end.y > image.height()
            {
                return Err(io::Error::other("Game viewport is unavailable"));
            }
            image = image.crop_imm(
                self.viewport.origin.x,
                self.viewport.origin.y,
                self.viewport.size.x,
                self.viewport.size.y,
            );
        }
        let rgba = image.to_rgba8();
        Pixels::new(rgba.width(), rgba.height(), rgba.into_raw())
    }
}
pub(in crate::bevy_shell) fn observe(
    commands: &mut Commands,
    owner: &mut HostOwner,
    viewport: WorldViewport,
    state: &mut State,
    captures: &Query<&Capture>,
) {
    let generation = owner.play.as_ref().map(|_| owner.generation());
    if state.generation != generation {
        owner.settings.reset_screenshot_session();
        state.generation = generation;
    }
    let Some(generation) = generation else {
        return;
    };
    // One readback at a time; a late callback cannot write into another session.
    if captures
        .iter()
        .any(|capture| capture.generation == generation)
    {
        return;
    }
    let HostOwner { settings, play, .. } = owner;
    let result = settings.process_screenshots(
        Instant::now(),
        play.as_ref().map(|p| &p.link),
        &mut Deferred,
    );
    if !result.is_err_and(|error| error.kind() == io::ErrorKind::WouldBlock) {
        return;
    }
    let scope = settings.preferences.screenshots.effective().game_only;
    commands
        .spawn((Screenshot::primary_window(), Capture { generation }))
        .observe(
            move |event: On<ScreenshotCaptured>, mut owner: NonSendMut<HostOwner>| {
                if owner.generation() != generation
                    || owner.play.is_none()
                    || owner.settings.preferences.screenshots.effective().game_only != scope
                {
                    return;
                }
                let HostOwner {
                    settings,
                    play,
                    status,
                    ..
                } = &mut *owner;
                let mut pixels = Rendered {
                    image: &event.image,
                    viewport,
                };
                match settings.process_screenshots(
                    Instant::now(),
                    play.as_ref().map(|p| &p.link),
                    &mut pixels,
                ) {
                    Ok(paths) if !paths.is_empty() => {
                        *status = format!("Saved {} screenshot(s)", paths.len())
                    }
                    Err(error) => {
                        settings.cancel_screenshot();
                        *status = format!("Screenshot failed: {error}");
                    }
                    _ => {}
                }
            },
        );
}
