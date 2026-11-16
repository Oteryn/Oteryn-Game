//! The house podium's artwork preview: once the server resolves a Preview request, the podium
//! window shows the composed outfit, mount or creature graphic beside its controls.
use super::*;
use crate::play::feature_0150_house_podium::view::{TITLE, preview_assets::Assets as Artwork};
use oteryn_client_runtime::cui3::house_podium::preview::Artwork as Resolved;

/// The pinned artwork, loaded on the first preview of the session.
#[derive(Resource, Default)]
pub(super) struct PodiumArtwork {
    pub(super) loaded: Option<Result<Artwork, String>>,
}

/// The preview beside one shown podium window, built from one resolved artwork.
#[derive(Component)]
pub(super) struct PodiumPreview {
    root: Entity,
    artwork: Resolved,
}

/// One preview image of the shown podium window.
#[derive(Component)]
pub(super) struct PodiumImage;

pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    state: Res<FormWindow>,
    mut artwork: ResMut<PodiumArtwork>,
    mut images: ResMut<Assets<Image>>,
    previews: Query<(Entity, &PodiumPreview)>,
) {
    let wanted = owner.play.as_ref().and_then(|play| {
        let shown = state.shown.as_ref()?;
        let form = FeatureForm::all().get(shown.layout.form)?;
        if form.title(&play.link) != TITLE {
            return None;
        }
        Some((shown.root, play.link.house_podium_panel().artwork?))
    });
    let mut kept = false;
    for (entity, preview) in &previews {
        if !kept && Some((preview.root, preview.artwork)) == wanted {
            kept = true;
        } else {
            commands.entity(entity).try_despawn();
        }
    }
    let Some((root, resolved)) = wanted.filter(|_| !kept) else {
        return;
    };
    let rendered = artwork
        .loaded
        .get_or_insert_with(|| {
            std::env::var_os(crate::map::ASSET_DIR_ENV)
                .ok_or_else(|| {
                    "Set OTERYN_ASSET_DIR to the existing pinned artwork root".to_owned()
                })
                .and_then(|root| Artwork::load(std::path::Path::new(&root)))
        })
        .as_mut()
        .map_err(|error| error.clone())
        .and_then(|assets| assets.render(resolved));
    let panel = commands
        .spawn((
            PodiumPreview {
                root,
                artwork: resolved,
            },
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                margin: UiRect::left(px(8.)),
                padding: UiRect {
                    top: px(24.),
                    bottom: px(10.),
                    left: px(12.),
                    right: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    source::dialog_caption(
        &mut commands,
        panel,
        "Podium artwork preview",
        skin.font.clone(),
    );
    match rendered {
        Ok(pictures) => {
            for pixels in pictures {
                let handle = images.add(Image::new(
                    bevy::render::render_resource::Extent3d {
                        width: 128,
                        height: 128,
                        depth_or_array_layers: 1,
                    },
                    bevy::render::render_resource::TextureDimension::D2,
                    pixels,
                    bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                    bevy::asset::RenderAssetUsages::default(),
                ));
                let picture = commands
                    .spawn((
                        PodiumImage,
                        ImageNode::new(handle),
                        Node {
                            width: px(128.),
                            height: px(128.),
                            ..default()
                        },
                    ))
                    .id();
                commands.entity(panel).add_child(picture);
            }
        }
        Err(error) => {
            let text = commands
                .spawn(source::text(error, source::font(skin.font.clone(), false)))
                .id();
            commands.entity(panel).add_child(text);
        }
    }
}
