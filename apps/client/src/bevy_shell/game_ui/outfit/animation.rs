//! Local preview playback; it never edits the appearance draft or sends a command.
use super::*;
use std::time::Duration;

#[derive(Clone)]
pub(super) enum Control {
    Left,
    Right,
    Movement(bool),
}
pub(super) struct Playback {
    pub(super) direction: u32,
    pub(super) moving: bool,
    elapsed: Duration,
    sample: Option<u128>,
}
impl Default for Playback {
    fn default() -> Self {
        Self {
            direction: 2,
            moving: true,
            elapsed: Duration::ZERO,
            sample: None,
        }
    }
}
impl Playback {
    pub(super) fn invalidate(&mut self) {
        self.sample = None;
    }
    pub(super) fn edit(&mut self, control: &Control) {
        match control {
            Control::Left => self.direction = (self.direction + 3) % 4,
            Control::Right => self.direction = (self.direction + 1) % 4,
            Control::Movement(moving) => {
                self.moving = *moving;
                self.elapsed = Duration::ZERO;
            }
        }
        self.sample = None;
    }
    fn next(&mut self, delta: Duration) -> Option<(u32, u64, bool)> {
        if self.moving {
            self.elapsed = self.elapsed.saturating_add(delta);
        }
        let sample = self.elapsed.as_millis() / 50;
        if self.sample == Some(sample) {
            return None;
        }
        self.sample = Some(sample);
        Some((
            self.direction,
            self.elapsed.as_millis().min(u128::from(u64::MAX)) as u64,
            self.moving,
        ))
    }
}

pub(super) fn animate(
    time: Res<Time<Real>>,
    mut state: ResMut<State>,
    mut images: ResMut<Assets<Image>>,
) {
    let State {
        rendered,
        playback,
        artwork,
        image: handle,
    } = &mut *state;
    let Some((_, panel)) = rendered.as_ref() else {
        *playback = Playback::default();
        return;
    };
    let Some((direction, elapsed, moving)) = playback.next(time.delta()) else {
        return;
    };
    let Some(artwork) = artwork.as_mut() else {
        return;
    };
    let Ok(pixels) = artwork.draw_panel(panel, direction, elapsed, moving) else {
        return;
    };
    if let Some(mut target) = handle.as_ref().and_then(|handle| images.get_mut(handle)) {
        *target = image(pixels);
    }
}

pub(super) fn image(mut pixels: Vec<u8>) -> Image {
    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
        pixel[3] = 255;
    }
    Image::new(
        bevy::render::render_resource::Extent3d {
            width: 96,
            height: 96,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        pixels,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    )
}
