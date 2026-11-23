//! Local preview playback; it never edits the appearance draft or sends a command.
use super::*;
use std::time::Duration;

#[derive(Clone)]
pub(super) enum Control {
    Character,
    Floor,
    Mount,
    Left,
    Right,
    Movement(bool),
}
pub(super) struct Playback {
    pub(super) direction: u32,
    pub(super) moving: bool,
    pub(super) layers: preview::Layers,
    elapsed: Duration,
    sample: Option<u128>,
}
impl Default for Playback {
    fn default() -> Self {
        Self {
            direction: 2,
            moving: true,
            layers: Default::default(),
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
            Control::Character => self.layers.character = !self.layers.character,
            Control::Floor => self.layers.floor = !self.layers.floor,
            Control::Mount => self.layers.mount = !self.layers.mount,
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

type DrawResult = (Option<preview::Assets>, Result<Vec<u8>, String>);

pub(super) struct Draw {
    requested: (u64, Panel),
    layers: preview::Layers,
    direction: u32,
    moving: bool,
    task: bevy_tasks::Task<DrawResult>,
}

pub(super) fn animate(
    time: Res<Time<Real>>,
    mut state: ResMut<State>,
    mut images: ResMut<Assets<Image>>,
) {
    if let Some(job) = &mut state.draw {
        let Some((artwork, pixels)) = bevy_tasks::futures::check_ready(&mut job.task) else {
            return;
        };
        let Some(job) = state.draw.take() else { return };
        let metadata = artwork.as_ref().and_then(|art| art.mounts.clone());
        state.dirty |= state.metadata.is_none() && metadata.is_some();
        state.metadata = metadata;
        state.artwork = artwork;
        if state.rendered.as_ref() == Some(&job.requested)
            && state.playback.layers == job.layers
            && state.playback.moving == job.moving
            && state.playback.direction == job.direction
        {
            let error = pixels.as_ref().err().cloned();
            state.dirty |= state.art_error != error;
            state.art_error = error;
            if let Some(mut target) = state
                .image
                .as_ref()
                .and_then(|handle| images.get_mut(handle))
            {
                *target = pixels.map(image).unwrap_or_default();
            }
            return;
        }
        state.playback.invalidate();
    }
    if state.rendered.is_none() {
        return;
    }
    let Some((direction, elapsed, moving)) = state.playback.next(time.delta()) else {
        return;
    };
    let Some(requested) = state
        .rendered
        .as_ref()
        .filter(|(_, panel)| panel.preview().is_some())
        .cloned()
    else {
        return;
    };
    let layers = state.playback.layers;
    let artwork = state.artwork.take();
    let root = std::env::var_os(crate::map::ASSET_DIR_ENV);
    if artwork.is_none() && root.is_none() {
        if state.art_error.is_none() {
            state.art_error = Some("Local outfit artwork unavailable".into());
            state.dirty = true;
        }
        return;
    }
    let panel = requested.1.clone();
    state.draw = Some(Draw {
        requested,
        direction,
        moving,
        layers,
        task: bevy_tasks::IoTaskPool::get_or_init(|| {
            bevy_tasks::TaskPoolBuilder::new().num_threads(1).build()
        })
        .spawn(async move {
            let mut artwork = artwork.or_else(|| {
                root.and_then(|root| preview::Assets::load(std::path::Path::new(&root)).ok())
            });
            let pixels = artwork
                .as_mut()
                .ok_or_else(|| "Local outfit artwork unavailable".to_owned())
                .and_then(|art| art.draw_layers(&panel, direction, elapsed, moving, layers));
            (artwork, pixels)
        }),
    });
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
