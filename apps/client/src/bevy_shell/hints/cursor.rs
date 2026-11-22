//! The map cursor has a dark outline so it stays visible on bright terrain.
use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

#[derive(Resource)]
pub(super) struct CursorImages([[Handle<Image>; 2]; 2]);

#[derive(Component)]
pub(super) struct GameCursor;
#[derive(Component)]
pub(super) struct TargetHighlight;

pub(super) fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    // Each source pixel scales as a square to preserve the tip at the window hotspot.
    let arrow = [
        "#               ",
        "##              ",
        "#+#             ",
        "#++#            ",
        "#+++#           ",
        "#++++#          ",
        "#+++++#         ",
        "#++++++#        ",
        "#+++++++#       ",
        "#++++++++#      ",
        "#++++#####      ",
        "#++#+#          ",
        "#+# #+#         ",
        "##  #+#         ",
        "     #+#        ",
        "      #         ",
    ];
    let handles = std::array::from_fn(|large| {
        std::array::from_fn(|frame| {
            let scale = 1 + large;
            let size = 16 * scale;
            let mut pixels = Vec::with_capacity(size * size * 4);
            for y in 0..size {
                for x in 0..size {
                    pixels.extend(match arrow[y / scale].as_bytes()[x / scale] {
                        b'#' => [32, 26, 19, 255],
                        b'+' if frame == 1 => [245, 208, 120, 255],
                        b'+' => [255, 247, 218, 255],
                        _ => [0; 4],
                    });
                }
            }
            images.add(Image::new(
                Extent3d {
                    width: size as u32,
                    height: size as u32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                pixels,
                TextureFormat::Rgba8UnormSrgb,
                default(),
            ))
        })
    });
    commands.spawn((
        TargetHighlight,
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            border: px(2.).all(),
            ..default()
        },
        BorderColor::all(Color::srgb(1., 0.85, 0.3)),
        Pickable::IGNORE,
    ));
    commands.spawn((
        GameCursor,
        ImageNode::default(),
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            ..default()
        },
        GlobalZIndex(i32::MAX),
        Pickable::IGNORE,
    ));
    commands.insert_resource(CursorImages(handles));
}
impl CursorImages {
    pub(super) fn handle(&self, large: bool, frame: bool) -> Handle<Image> {
        self.0[usize::from(large)][usize::from(frame)].clone()
    }
}

pub(super) fn present(
    owner: NonSend<super::HostOwner>,
    state: Res<super::HintState>,
    images: Res<CursorImages>,
    (time, scale): (Res<Time<Real>>, Res<UiScale>),
    mut windows: Query<
        (&Window, &mut bevy::window::CursorOptions),
        With<bevy::window::PrimaryWindow>,
    >,
    mut cursors: Query<(&mut Node, &mut ImageNode), With<GameCursor>>,
    mut highlights: Query<&mut Node, (With<TargetHighlight>, Without<GameCursor>)>,
) {
    let Ok((window, mut options)) = windows.single_mut() else {
        return;
    };
    let Ok((mut node, mut image)) = cursors.single_mut() else {
        return;
    };
    let factor = window.scale_factor() * scale.0;
    if let Ok(mut node) = highlights.single_mut() {
        if let Some((pos, size)) = state.highlight {
            node.left = px(pos.x / factor);
            node.top = px(pos.y / factor);
            node.width = px(size / factor);
            node.height = px(size / factor);
            node.display = Display::Flex;
        } else {
            node.display = Display::None;
        }
    }
    let prefs = &owner.settings.preferences;
    let pointer = state.pointer.filter(|_| {
        !prefs.system_cursor && state.cursor == bevy::window::SystemCursorIcon::Default
    });
    options.visible = pointer.is_none();
    if let Some(point) = pointer {
        let factor = window.scale_factor() * scale.0;
        node.display = Display::Flex;
        node.left = px(point.x / factor);
        node.top = px(point.y / factor);
        let size = if prefs.large_cursor { 32. } else { 16. };
        node.width = px(size / factor);
        node.height = px(size / factor);
        image.image = images.handle(
            prefs.large_cursor,
            prefs.animated_cursor && (time.elapsed().as_millis() / 250) % 2 == 1,
        );
    } else {
        node.display = Display::None;
    }
}
