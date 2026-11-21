//! Gallery portraits use the existing pinned appearance loader and server look IDs.
use bevy::prelude::*;
#[path = "../../../features/0199-cyclopedia-character/view/gallery_pixels.rs"]
mod pixels;
#[derive(Component)]
pub(in crate::bevy_shell::game_ui) struct Portrait(pub u32);
#[derive(Resource, Default)]
struct State {
    assets: Option<Result<pixels::Art, String>>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Update, project.after(super::project));
}
fn project(
    mut commands: Commands,
    portraits: Query<(Entity, &Portrait), Added<Portrait>>,
    mut state: ResMut<State>,
    mut images: ResMut<Assets<Image>>,
    skin: Res<crate::bevy_shell::ui::Skins>,
) {
    for (entity, portrait) in &portraits {
        let result = match state.assets.get_or_insert_with(pixels::Art::load) {
            Ok(assets) => assets.pixels(portrait.0),
            Err(error) => Err(error.clone()),
        };
        match result {
            Ok(mut pixels) => {
                // The shared loader returns opaque BGR pixels for the native frontend.
                for pixel in pixels.as_chunks_mut::<4>().0 {
                    pixel.swap(0, 2);
                    pixel[3] = 255;
                }
                let image = Image::new(
                    bevy::render::render_resource::Extent3d {
                        width: 96,
                        height: 96,
                        depth_or_array_layers: 1,
                    },
                    bevy::render::render_resource::TextureDimension::D2,
                    pixels,
                    bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                    bevy::asset::RenderAssetUsages::default(),
                );
                commands
                    .entity(entity)
                    .insert(ImageNode::new(images.add(image)));
            }
            Err(error) => {
                let caption = commands
                    .spawn(oteryn_client_ui_bevy::text(
                        format!("Artwork unavailable: {error}"),
                        oteryn_client_ui_bevy::font(skin.font.clone(), false),
                    ))
                    .id();
                commands.entity(entity).add_child(caption);
            }
        }
    }
}
