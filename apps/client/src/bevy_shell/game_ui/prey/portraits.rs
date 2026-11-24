//! Load pinned appearance pixels away from the game schedule.
use super::*;
use crate::play::feature_0150_house_podium::view::preview_assets::Assets as Artwork;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_tasks::{IoTaskPool, Task, futures::check_ready};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[derive(Component, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub(super) struct Portrait {
    object: bool,
    pub look: u32,
}
type Batch = (Option<Artwork>, Vec<(Portrait, Option<Vec<u8>>)>);
#[derive(Resource, Default)]
pub(super) struct Library {
    pub root: Option<PathBuf>,
    assets: Option<Artwork>,
    task: Option<Task<Batch>>,
    images: BTreeMap<Portrait, Option<Handle<Image>>>,
}
pub(super) fn install(app: &mut App) {
    app.insert_resource(Library {
        root: std::env::var_os(crate::map::ASSET_DIR_ENV).map(PathBuf::from),
        ..default()
    })
    .init_resource::<Assets<Image>>()
    .add_systems(Update, project.after(super::project));
}
pub(super) fn attach(commands: &mut Commands, parent: Entity, key: &str) {
    let Some(species) = catalogue::species(key) else {
        return;
    };
    let entity = commands
        .spawn((
            Name::new(format!("prey.portrait.{key}")),
            Portrait {
                object: species.object,
                look: species.look,
            },
            Pickable::IGNORE,
            Node {
                width: px(32.),
                height: px(32.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).insert_children(0, &[entity]);
}
fn project(
    mut commands: Commands,
    mut library: ResMut<Library>,
    mut images: ResMut<Assets<Image>>,
    portraits: Query<(Entity, &Portrait), Without<ImageNode>>,
) {
    if let Some((assets, batch)) = library.task.as_mut().and_then(check_ready) {
        library.assets = assets;
        library.task = None;
        for (portrait, pixels) in batch {
            let image = pixels.map(|rgba| {
                images.add(Image::new(
                    Extent3d {
                        width: 128,
                        height: 128,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    rgba,
                    TextureFormat::Rgba8UnormSrgb,
                    bevy::asset::RenderAssetUsages::default(),
                ))
            });
            library.images.insert(portrait, image);
        }
    }
    let mut missing = BTreeSet::new();
    for (entity, portrait) in &portraits {
        match library.images.get(portrait) {
            Some(Some(image)) => {
                commands
                    .entity(entity)
                    .insert(ImageNode::new(image.clone()));
            }
            Some(None) => {
                commands.entity(entity).insert(Visibility::Hidden);
            }
            None => {
                missing.insert(*portrait);
            }
        }
    }
    let Some(root) = library
        .root
        .as_ref()
        .filter(|_| library.task.is_none() && !missing.is_empty())
        .cloned()
    else {
        return;
    };
    let assets = library.assets.take();
    library.task = Some(
        IoTaskPool::get_or_init(|| bevy_tasks::TaskPoolBuilder::new().num_threads(1).build())
            .spawn(async move {
                let mut assets = assets.or_else(|| Artwork::load(&root).ok());
                let batch = missing
                    .into_iter()
                    .map(|portrait| {
                        let pixels = assets
                            .as_mut()
                            .and_then(|a| {
                                a.render(
                                    oteryn_client_runtime::cui3::house_podium::preview::Artwork {
                                        object: portrait.object,
                                        direction: 2,
                                        looks: [portrait.look; 2],
                                    },
                                )
                                .ok()
                            })
                            .and_then(|mut pictures| pictures.pop());
                        (portrait, pixels)
                    })
                    .collect();
                (assets, batch)
            }),
    );
}
