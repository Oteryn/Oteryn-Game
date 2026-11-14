//! Bevy supplies pixels; the retained screenshot owner supplies policy, backlog and BMP persistence.
use super::*;
use crate::play::screenshot::{CaptureSource, Pixels};
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use std::{
    io,
    time::{Duration, Instant},
};
#[derive(Resource, Default)]
struct Capture {
    pending: Option<(u64, ReferencePreferences)>,
    last: Option<Instant>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<Capture>()
        .add_systems(PostUpdate, request);
}
fn request(
    mut commands: Commands,
    mut owner: NonSendMut<HostOwner>,
    mut capture: ResMut<Capture>,
    viewport: Res<super::super::viewport::WorldViewport>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
) {
    if owner.phase == crate::bevy_shell::owner::Phase::Closed {
        return;
    }
    let generation = owner.generation();
    owner.settings.bind_screenshot_session(generation);
    if capture.pending.is_some() {
        return;
    }
    let preferences = &owner.settings.preferences.screenshots;
    if !owner.settings.screenshot_pending()
        && (owner.play.is_none()
            || (preferences.backlog_frames == 0
                && preferences.backlog_seconds == 0
                && !preferences.on_death
                && !preferences.on_level
                && !preferences.on_skill
                && !preferences.on_achievement))
    {
        return;
    }
    let now = Instant::now();
    if !owner.settings.screenshot_pending()
        && capture
            .last
            .is_some_and(|at| now.duration_since(at) < Duration::from_secs(1))
    {
        return;
    }
    capture.last = Some(now);
    capture.pending = Some((owner.generation(), owner.settings.preferences.clone()));
    let (screenshot, camera) = if preferences.effective().game_only {
        let Ok(window) = windows.single() else {
            capture.pending = None;
            return;
        };
        if viewport.size.min_element() == 0 {
            capture.pending = None;
            owner.settings.cancel_screenshot();
            owner.settings.feedback = "Game viewport is unavailable".into();
            return;
        }
        let mut target = Image::new_target_texture(
            viewport.size.x,
            viewport.size.y,
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            None,
        );
        target.texture_descriptor.usage |= bevy::render::render_resource::TextureUsages::COPY_SRC;
        let image = images.add(target);
        let camera = commands
            .spawn((
                Camera2d,
                Camera {
                    order: -1,
                    ..default()
                },
                bevy::camera::RenderTarget::Image(bevy::camera::ImageRenderTarget {
                    handle: image.clone(),
                    scale_factor: window.scale_factor(),
                }),
                bevy::camera::visibility::RenderLayers::layer(1),
            ))
            .id();
        (Screenshot::image(image), Some(camera))
    } else {
        (Screenshot::primary_window(), None)
    };
    commands.spawn(screenshot).observe(
        move |event: On<ScreenshotCaptured>,
              mut owner: NonSendMut<HostOwner>,
              mut capture: ResMut<Capture>,
              mut commands: Commands| {
            if let Some(camera) = camera {
                commands.entity(camera).despawn();
            }
            let Some((generation, expected)) = capture.pending.take() else {
                return;
            };
            if owner.phase == crate::bevy_shell::owner::Phase::Closed
                || owner.generation() != generation
                || owner.settings.preferences != expected
            {
                return;
            }
            let result = event
                .image
                .clone()
                .try_into_dynamic()
                .map_err(|error| io::Error::other(error.to_string()))
                .and_then(|image| {
                    let rgba = image.to_rgba8();
                    let mut source = Source {
                        width: rgba.width(),
                        height: rgba.height(),
                        rgba: rgba.into_raw(),
                    };
                    let HostOwner { settings, play, .. } = &mut *owner;
                    settings.process_screenshots(
                        Instant::now(),
                        play.as_ref().map(|p| &p.link),
                        &mut source,
                    )
                });
            match result {
                Ok(paths) if !paths.is_empty() => {
                    owner.settings.feedback = format!(
                        "Saved {} screenshot(s): {}",
                        paths.len(),
                        paths
                            .last()
                            .map_or_else(String::new, |p| p.display().to_string())
                    )
                }
                Err(error) => {
                    owner.settings.cancel_screenshot();
                    owner.settings.feedback =
                        format!("Screenshot failed: {error}. Capture now retries.")
                }
                Ok(_) => {
                    capture.last = Some(Instant::now());
                }
            }
        },
    );
}
struct Source {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}
impl CaptureSource for Source {
    fn capture(&mut self, _game_only: bool) -> io::Result<Pixels> {
        Pixels::new(self.width, self.height, self.rgba.clone())
    }
}
