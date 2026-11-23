//! Rasterize visible hash-pinned satellite/minimap chunks off the UI thread.
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, block_on, futures_lite::future},
};
use oteryn_client_assets::{
    AssetStore,
    world::{MANIFEST_PATH, MANIFEST_SHA256},
};
use std::path::PathBuf;

#[derive(Component, Clone, Copy, Eq, PartialEq)]
pub(super) struct Terrain {
    pub bounds: [u16; 4],
    pub floor: i8,
    pub surface: bool,
    pub opacity: u8,
}
#[derive(Resource, Default)]
pub(super) struct Cache {
    pub root: Option<PathBuf>,
    key: Option<Terrain>,
    task: Option<Task<Result<(Image, bool), String>>>,
    image: Option<Handle<Image>>,
    error: Option<String>,
}
pub(super) fn project(
    mut commands: Commands,
    mut cache: ResMut<Cache>,
    mut state: ResMut<super::State>,
    mut images: ResMut<Assets<Image>>,
    canvases: Query<(Entity, &Terrain, Option<&ImageNode>)>,
    skin: Res<crate::bevy_shell::ui::Skins>,
) {
    let Ok((entity, &key, shown)) = canvases.single() else {
        return;
    };
    if cache.key != Some(key) {
        cache.key = Some(key);
        cache.image = None;
        cache.error = None;
        let root = cache
            .root
            .clone()
            .or_else(|| std::env::var_os(crate::map::ASSET_DIR_ENV).map(PathBuf::from));
        cache.task = Some(AsyncComputeTaskPool::get().spawn(async move {
            let root = root.ok_or("Set the asset directory to load the world map")?;
            let store = AssetStore::open_pinned(
                &root.join("content/assets/files"),
                &root.join(MANIFEST_PATH),
                MANIFEST_SHA256,
            )
            .map_err(|e| e.to_string())?;
            key.image(&store)
        }));
    }
    if let Some(result) = cache
        .task
        .as_mut()
        .and_then(|task| block_on(future::poll_once(task)))
    {
        cache.task = None;
        match result {
            Ok((image, satellite)) => {
                if key.surface && !satellite {
                    state.no_satellite = Some((key.bounds, key.floor));
                    state.surface = false;
                    state.snapshot = None;
                    cache.key = Some(Terrain {
                        surface: false,
                        ..key
                    });
                }
                cache.image = Some(images.add(image));
            }
            Err(message) => cache.error = Some(message),
        }
    }
    if shown.is_none() {
        if let Some(handle) = &cache.image {
            commands
                .entity(entity)
                .insert(ImageNode::new(handle.clone()));
        } else if let Some(error) = &cache.error {
            commands.spawn((
                ChildOf(entity),
                oteryn_client_ui_bevy::text(
                    error.clone(),
                    oteryn_client_ui_bevy::font(skin.font.clone(), false),
                ),
            ));
            commands.entity(entity).remove::<Terrain>();
        }
    }
}
impl Terrain {
    fn image(self, store: &AssetStore) -> Result<(Image, bool), String> {
        let [left, top, right, bottom] = self.bounds.map(i32::from);
        let w = right - left + 1;
        let h = bottom - top + 1;
        let scale = (600.0 / w as f64).min(400.0 / h as f64);
        let ox = 16 + (600 - (w as f64 * scale) as i32) / 2;
        let oy = 66 + (400 - (h as f64 * scale) as i32) / 2;
        let best = if scale >= 2. && self.surface {
            16
        } else if scale >= 1. {
            32
        } else {
            64
        };
        let coarser = (best * 2).min(64);
        let mut chunks = Vec::new();
        for name in store.file_names() {
            let mut fields = name.split('-');
            if fields.next() != Some(if self.surface { "satellite" } else { "minimap" }) {
                continue;
            }
            let mut number = || fields.next().and_then(|s| s.parse::<i32>().ok());
            let (Some(lod), Some(x), Some(y), Some(z)) = (number(), number(), number(), number())
            else {
                continue;
            };
            let target = i32::from(self.floor);
            if !(lod == best || lod == coarser || !self.surface && lod == 32)
                || z < target
                || z > if self.surface { target.max(7) } else { target }
                || z != target && self.opacity == 0
            {
                continue;
            }
            let (x, y, span) = (32 * x, 32 * y, 16 * lod);
            if x <= right && y <= bottom && x + span > left && y + span > top {
                chunks.push((std::cmp::Reverse(z), std::cmp::Reverse(lod), x, y, name));
            }
        }
        if self.surface
            && !chunks
                .iter()
                .any(|(floor, ..)| floor.0 == i32::from(self.floor))
        {
            return Self {
                surface: false,
                ..self
            }
            .image(store);
        }
        chunks.sort_unstable();
        if chunks.is_empty() {
            return Err("No map artwork for this area and floor".into());
        }
        let mut pixels = vec![0; 640 * 480 * 4];
        let mut layer = new_floor_layer(pixels.len());
        for floor in chunks.chunk_by(|a, b| a.0 == b.0) {
            layer.fill(0);
            for &(std::cmp::Reverse(_), std::cmp::Reverse(lod), x, y, name) in floor {
                let data = store.map_chunk(name).map_err(|e| e.to_string())?;
                let span = 16 * lod;
                let screen = |v: i32, origin: i32| ((v - origin) as f64 * scale).ceil() as i32;
                for py in oy + screen(y.max(top), top)..oy + screen((y + span).min(bottom + 1), top)
                {
                    let sy = (((py - oy) as f64 / scale + f64::from(top - y)) * 32.
                        / f64::from(lod)) as usize;
                    for px in
                        ox + screen(x.max(left), left)..ox + screen((x + span).min(right + 1), left)
                    {
                        let sx = (((px - ox) as f64 / scale + f64::from(left - x)) * 32.
                            / f64::from(lod)) as usize;
                        let src = &data[(sy.min(511) * 512 + sx.min(511)) * 4..][..4];
                        let dst = &mut layer[((py * 640 + px) * 4) as usize..][..4];
                        let alpha = u32::from(src[3]);
                        for c in 0..3 {
                            dst[c] = ((u32::from(src[c]) * alpha
                                + u32::from(dst[c]) * (255 - alpha))
                                / 255) as u8;
                        }
                        dst[3] = (alpha + u32::from(dst[3]) * (255 - alpha) / 255) as u8;
                    }
                }
            }
            let opacity = if floor[0].0.0 == i32::from(self.floor) {
                100
            } else {
                u32::from(self.opacity)
            };
            for (source, dest) in layer
                .as_chunks::<4>()
                .0
                .iter()
                .zip(pixels.as_chunks_mut::<4>().0.iter_mut())
            {
                if source[3] == 0 {
                    continue;
                }
                let alpha = u32::from(source[3]) * opacity / 100;
                for c in 0..3 {
                    dest[c] = (u32::from(source[c]) * opacity / 100
                        + u32::from(dest[c]) * (255 - alpha) / 255)
                        .min(255) as u8;
                }
                dest[3] = 255;
            }
        }
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        Ok((
            Image::new(
                Extent3d {
                    width: 640,
                    height: 480,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                pixels,
                TextureFormat::Rgba8UnormSrgb,
                bevy::asset::RenderAssetUsages::default(),
            ),
            self.surface,
        ))
    }
}

fn new_floor_layer(bytes: usize) -> Vec<u8> {
    #[cfg(test)]
    FLOOR_LAYERS.set(FLOOR_LAYERS.get() + 1);
    vec![0; bytes]
}

#[cfg(test)]
std::thread_local! {
    static FLOOR_LAYERS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[test]
fn stacked_satellite_floors_share_one_layer() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let store = AssetStore::open_pinned(
        &root.join("content/assets/files"),
        &root.join(MANIFEST_PATH),
        MANIFEST_SHA256,
    )?;
    FLOOR_LAYERS.set(0);
    let (_, satellite) = Terrain {
        bounds: [31872, 30976, 31935, 31039],
        floor: 0,
        surface: true,
        opacity: 50,
    }
    .image(&store)?;
    assert!(satellite);
    assert_eq!(FLOOR_LAYERS.get(), 1);
    Ok(())
}
