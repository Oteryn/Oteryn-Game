//! Resolve the inspected incarnation against the current visible appearance dictionary.
use super::*;
use oteryn_client_ui::inspection::OutfitPortrait;
#[derive(Component)]
pub(super) struct Portrait(oteryn_session::EntityRef);
#[derive(Component)]
pub(super) struct Unavailable;
pub(super) fn spawn(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    entity: oteryn_session::EntityRef,
) {
    let image = commands
        .spawn((
            Portrait(entity),
            ImageNode {
                color: Color::NONE,
                ..default()
            },
            Node {
                width: px(96.),
                height: px(96.),
                ..default()
            },
        ))
        .id();
    let status = commands
        .spawn((
            Unavailable,
            oteryn_client_ui_bevy::text(
                "Outfit unavailable",
                oteryn_client_ui_bevy::font(skin.font.clone(), false),
            ),
        ))
        .id();
    commands.entity(parent).add_children(&[image, status]);
}
type Artwork = super::super::outfit::preview::Assets;
type RenderTask = bevy_tasks::Task<(Option<Artwork>, Option<Image>)>;
#[derive(Resource, Default)]
pub(super) struct Cache {
    pub(super) artwork: Option<Artwork>,
    requested: Option<(Entity, OutfitPortrait)>,
    task: Option<RenderTask>,
}

fn render(mut artwork: Option<Artwork>, look: OutfitPortrait) -> (Option<Artwork>, Option<Image>) {
    if artwork.is_none() {
        artwork = std::env::var_os(crate::map::ASSET_DIR_ENV)
            .and_then(|root| Artwork::load(std::path::Path::new(&root)).ok());
    }
    let image = artwork
        .as_mut()
        .and_then(|art| {
            art.draw_portrait(look.look_type, look.addons, look.colours)
                .ok()
        })
        .map(|mut pixels| {
            for pixel in pixels.as_chunks_mut::<4>().0 {
                pixel[3] = 255;
            }
            use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
            Image::new(
                Extent3d {
                    width: 96,
                    height: 96,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                pixels,
                TextureFormat::Bgra8UnormSrgb,
                bevy::asset::RenderAssetUsages::default(),
            )
        });
    (artwork, image)
}
pub(super) fn project(
    owner: NonSend<HostOwner>,
    mut cache: ResMut<Cache>,
    mut images: ResMut<Assets<Image>>,
    mut portraits: Query<(Entity, &Portrait, &mut ImageNode)>,
    mut status: Query<&mut Node, With<Unavailable>>,
) {
    let completed = cache
        .task
        .as_mut()
        .and_then(bevy_tasks::futures::check_ready)
        .map(|(artwork, image)| {
            cache.task = None;
            cache.artwork = artwork;
            image
        });
    let Ok((entity, portrait, mut node)) = portraits.single_mut() else {
        return;
    };
    let requested = owner
        .play
        .as_ref()
        .and_then(|play| play.view.inspection_preview(&play.link, portrait.0))
        .map(|look| (entity, look));
    if requested == cache.requested {
        if let Some(Some(image)) = completed {
            *node = ImageNode::new(images.add(image));
            for mut status in &mut status {
                status.display = Display::None;
            }
        }
        return;
    }
    node.color = Color::NONE;
    for mut status in &mut status {
        status.display = Display::Flex;
    }
    if cache.task.is_none() {
        if let Some((_, look)) = &requested {
            let look = look.clone();
            let artwork = cache.artwork.take();
            cache.task =
                Some(bevy_tasks::IoTaskPool::get().spawn(async move { render(artwork, look) }));
        }
        cache.requested = requested;
    }
}
