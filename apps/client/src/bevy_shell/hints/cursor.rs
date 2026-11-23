//! The map cursor has a dark outline so it stays visible on bright terrain.
use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use bevy::window::SystemCursorIcon as Icon;

#[derive(Resource)]
pub(super) struct CursorImages(std::collections::HashMap<Icon, [[Handle<Image>; 2]; 2]>);

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
    let shapes = [
        Icon::Default,
        Icon::Crosshair,
        Icon::Help,
        Icon::Text,
        Icon::NsResize,
        Icon::EwResize,
        Icon::NwseResize,
        Icon::NeswResize,
    ];
    let handles = shapes
        .into_iter()
        .map(|icon| {
            let frames = std::array::from_fn(|large| {
                std::array::from_fn(|frame| {
                    let scale = 1 + large;
                    let size = 16 * scale;
                    let mut pixels = Vec::with_capacity(size * size * 4);
                    for y in 0..size {
                        for x in 0..size {
                            let (x, y) = ((x / scale) as i32, (y / scale) as i32);
                            let ink = if icon == Icon::Default {
                                arrow[y as usize].as_bytes()[x as usize]
                            } else if stroke(icon, x, y) {
                                b'+'
                            } else if [(1, 0), (-1, 0), (0, 1), (0, -1)]
                                .into_iter()
                                .any(|(dx, dy)| stroke(icon, x + dx, y + dy))
                            {
                                b'#'
                            } else {
                                b' '
                            };
                            pixels.extend(match ink {
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
            (icon, frames)
        })
        .collect();
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
fn stroke(icon: Icon, x: i32, y: i32) -> bool {
    let (x, y) = (x - 7, y - 7);
    let arrow = |along: i32, across: i32| {
        along.abs() <= 6 && (across == 0 || (along.abs() >= 3 && across.abs() == 6 - along.abs()))
    };
    match icon {
        Icon::Crosshair => (x == 0 || y == 0) && (2..=6).contains(&(x.abs() + y.abs())),
        Icon::Text => (x == 0 && y.abs() <= 6) || (y.abs() == 6 && x.abs() <= 2),
        Icon::Help => {
            ((y == -5 || y == -1) && (0..=2).contains(&x))
                || (x == -1 && y == -4)
                || (x == 3 && (-4..=-2).contains(&y))
                || (x == 0 && (0..=2).contains(&y))
                || (x == 0 && y == 5)
        }
        Icon::NsResize => arrow(y, x),
        Icon::EwResize => arrow(x, y),
        Icon::NwseResize => {
            (x == y && x.abs() <= 5)
                || (x.signum() == y.signum()
                    && x.abs().max(y.abs()) == 5
                    && x.abs().min(y.abs()) >= 2)
        }
        Icon::NeswResize => stroke(Icon::NwseResize, 7 - x, 7 + y),
        _ => false,
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
    let pointer = window
        .physical_cursor_position()
        .filter(|_| window.focused && !prefs.system_cursor);
    options.visible = pointer.is_none();
    if let Some(point) = pointer {
        node.display = Display::Flex;
        let size = if prefs.large_cursor { 32. } else { 16. };
        let offset = if state.cursor == Icon::Default {
            0.
        } else {
            size * 7. / 16.
        };
        node.left = px((point.x - offset) / factor);
        node.top = px((point.y - offset) / factor);
        node.width = px(size / factor);
        node.height = px(size / factor);
        let frames = images
            .0
            .get(&state.cursor)
            .unwrap_or(&images.0[&Icon::Default]);
        let animated = prefs.animated_cursor && (time.elapsed().as_millis() / 250) % 2 == 1;
        image.image = frames[usize::from(prefs.large_cursor)][usize::from(animated)].clone();
    } else {
        node.display = Display::None;
    }
}
