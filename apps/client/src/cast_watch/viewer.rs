//! Standalone Bevy Watch shell: no PlayView, character login or private panels. The public
//! picture is composed into one image; the cast text and the chat draft sit below it.
use super::{
    WatchConfig, WatchLink,
    picture::{Assets as Pictures, Drawn},
};
use bevy::{
    asset::{AssetPlugin, RenderAssetUsages},
    input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
        mouse::{MouseScrollUnit, MouseWheel},
    },
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    window::PrimaryWindow,
};
use oteryn_client_runtime::ClientRuntime;
use oteryn_client_ui::cast_view::{ChatDraft, Panel, panel_text};
use oteryn_renderer::QuadInstance;
use oteryn_session::world_map::{VIEW_HEIGHT, VIEW_WIDTH};
use std::time::{Duration, Instant};

/// The public map window in pixels: the picture's tile view at 32 px per tile.
const WIDTH: u32 = VIEW_WIDTH as u32 * 32;
const HEIGHT: u32 = VIEW_HEIGHT as u32 * 32;

/// `oteryn-client --watch`: joins with the one-use account observer ticket and shows the
/// cast until it ends, the viewer presses Escape or closes the window.
pub fn run() -> Result<(), String> {
    let options = crate::bevy_shell::Options::parse(std::env::args_os().skip(1))?;
    let config = WatchConfig::from_env(|key| std::env::var(key).ok()).map_err(str::to_owned)?;
    let runtime = ClientRuntime::new().map_err(|e| e.to_string())?;
    let link = super::spawn(&runtime, config).map_err(|e| e.to_string())?;
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: options.ui_assets.to_string_lossy().into_owned(),
                ..default()
            })
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: tr!("Oteryn Watch — connecting").into(),
                    resolution: (WIDTH, 700).into(),
                    ..default()
                }),
                ..default()
            }),
    )
    .insert_resource(ClearColor(Color::BLACK));
    crate::bevy_shell::install_font_fallback(&mut app, &options.ui_assets);
    install(&mut app, link.clone());
    let exit = app.run();
    link.leave();
    runtime.shutdown(Duration::from_millis(500));
    match exit {
        AppExit::Success => Ok(()),
        AppExit::Error(code) => Err(format!("Bevy event loop exited with {code}")),
    }
}

struct Viewer {
    link: WatchLink,
    pictures: Pictures,
    panel: Panel,
    draft: ChatDraft,
    version: u64,
    image: Handle<Image>,
}
/// The cast text and chat draft below the picture.
#[derive(Component)]
struct WatchText;

#[derive(Component)]
struct WatchChat;

fn install(app: &mut App, link: WatchLink) {
    app.add_message::<AppExit>()
        .add_message::<MouseWheel>()
        .insert_non_send(Viewer {
            link,
            pictures: Pictures::new(),
            panel: Panel::default(),
            draft: ChatDraft::default(),
            version: u64::MAX,
            image: Handle::default(),
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (keyboard, refresh, scroll_chat).chain());
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    assets: Res<AssetServer>,
    fallback: Option<Res<crate::bevy_shell::BuiltInFont>>,
    mut viewer: NonSendMut<Viewer>,
) {
    viewer.image = images.add(canvas(None));
    commands.spawn(Camera2d);
    commands.spawn((
        Node {
            width: percent(100.),
            height: percent(100.),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        children![
            (
                ImageNode::new(viewer.image.clone()),
                Node {
                    width: px(WIDTH as f32),
                    height: px(HEIGHT as f32),
                    flex_shrink: 0.,
                    ..default()
                },
            ),
            (
                WatchChat,
                // Layout clamps this to the end, including when wrapped text grows on resize.
                ScrollPosition(Vec2::new(0., f32::MAX)),
                Node {
                    flex_grow: 1.,
                    flex_basis: px(0.),
                    min_height: px(0.),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                children![(
                    WatchText,
                    Text::new(String::new()),
                    TextFont {
                        font: crate::bevy_shell::ui_font(&assets, fallback.is_some()).into(),
                        font_size: 12.0.into(),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    Node {
                        flex_shrink: 0.,
                        margin: UiRect::all(px(8.)),
                        ..default()
                    },
                )],
            ),
        ],
    ));
}

/// Escape leaves, Enter sends the draft as cast chat, other keys edit the draft.
fn keyboard(
    mut keys: MessageReader<KeyboardInput>,
    mut viewer: NonSendMut<Viewer>,
    mut exit: MessageWriter<AppExit>,
) {
    let viewer = &mut *viewer;
    for key in keys.read() {
        if key.state != ButtonState::Pressed || key.repeat {
            continue;
        }
        match &key.logical_key {
            Key::Escape => {
                viewer.link.leave();
                exit.write(AppExit::Success);
            }
            Key::Enter => {
                if let Some(text) = viewer.draft.take() {
                    viewer.link.chat(text);
                }
            }
            Key::Backspace => viewer.draft.backspace(),
            _ => {
                if let Some(text) = &key.text {
                    viewer.draft.append(text);
                }
            }
        }
    }
}

/// Redraws the picture when the cast changed or an effect cue expired, and keeps the title
/// and text current.
fn refresh(
    mut viewer: NonSendMut<Viewer>,
    mut images: ResMut<Assets<Image>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut texts: Query<&mut Text, With<WatchText>>,
    mut chat: Query<&mut ScrollPosition, With<WatchChat>>,
    mut pictures: Query<&mut Node, With<ImageNode>>,
) {
    if let Ok(window) = windows.single()
        && let Ok(mut picture) = pictures.single_mut()
    {
        let scale = (window.width() / WIDTH as f32)
            .min(window.height() * 0.65 / HEIGHT as f32)
            .min(1.);
        let width = px(WIDTH as f32 * scale);
        let height = px(HEIGHT as f32 * scale);
        if (picture.width, picture.height) != (width, height) {
            picture.width = width;
            picture.height = height;
        }
    }
    let viewer = &mut *viewer;
    let Some((version, panel)) = viewer.link.snapshot() else {
        viewer.link.fail();
        return;
    };
    let expiring = viewer
        .panel
        .picture
        .effects
        .iter()
        .any(|(at, _)| at.elapsed() >= Duration::from_millis(800));
    if version != viewer.version || expiring {
        let now = Instant::now();
        viewer.panel = panel;
        viewer.panel.picture.expire(now);
        viewer.version = version;
        let drawn = match viewer.pictures.draw(&viewer.panel.picture, now) {
            Ok(drawn) => Some(drawn),
            Err(_) => {
                viewer.panel.picture.clear();
                viewer.panel.picture.ended = true;
                viewer.panel.feedback = tr!("Cast picture unavailable").into();
                viewer.link.leave();
                viewer.pictures.draw(&viewer.panel.picture, now).ok()
            }
        };
        if let Some(mut image) = images.get_mut(&viewer.image) {
            *image = canvas(drawn.as_ref());
        }
        for mut window in &mut windows {
            window.title = lfmt!("Oteryn Watch — {}", viewer.panel.picture.status());
        }
    }
    let text = lfmt!(
        "{}\nCast chat: {}",
        panel_text(&viewer.panel),
        viewer.draft.0
    );
    for mut shown in &mut texts {
        if shown.0 != text {
            shown.0.clone_from(&text);
            if let Ok(mut position) = chat.single_mut() {
                position.0.y = f32::MAX;
            }
        }
    }
}

fn scroll_chat(
    mut wheel: MessageReader<MouseWheel>,
    mut chat: Query<(&ComputedNode, &mut ScrollPosition), With<WatchChat>>,
) {
    let delta: f32 = wheel
        .read()
        .map(|event| match event.unit {
            MouseScrollUnit::Line => event.y * 24.,
            MouseScrollUnit::Pixel => event.y,
        })
        .sum();
    if delta != 0.
        && let Ok((node, mut position)) = chat.single_mut()
    {
        let scale = node.inverse_scale_factor();
        position.0.y = (node.scroll_position.y * scale - delta)
            .clamp(0., (node.content_size().y - node.size().y).max(0.) * scale);
    }
}

/// The picture's terrain and sprite quads composed in draw order over black.
fn canvas(drawn: Option<&Drawn>) -> Image {
    let mut rgba = [0, 0, 0, 255].repeat((WIDTH * HEIGHT) as usize);
    if let Some(drawn) = drawn {
        for quad in drawn
            .tiles
            .instances()
            .iter()
            .chain(drawn.sprites.instances())
        {
            blend(&mut rgba, quad, drawn);
        }
    }
    Image::new(
        Extent3d {
            width: WIDTH,
            height: HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

/// Samples the quad's atlas rectangle at the nearest texel and blends it by its alpha.
fn blend(rgba: &mut [u8], quad: &QuadInstance, drawn: &Drawn) {
    let atlas = &drawn.atlas;
    let (atlas_width, atlas_height) = (atlas.width() as f32, atlas.height() as f32);
    let [x, y] = quad.position;
    let [width, height] = quad.size;
    let [u0, v0, u1, v1] = quad.uv;
    if width <= 0. || height <= 0. {
        return;
    }
    let left = x.max(0.) as u32;
    let top = y.max(0.) as u32;
    let right = (x + width).clamp(0., WIDTH as f32) as u32;
    let bottom = (y + height).clamp(0., HEIGHT as f32) as u32;
    for row in top..bottom {
        let v = v0 + (v1 - v0) * ((row as f32 + 0.5 - y) / height);
        let source_y = ((v * atlas_height) as u32).min(atlas.height().saturating_sub(1));
        for column in left..right {
            let u = u0 + (u1 - u0) * ((column as f32 + 0.5 - x) / width);
            let source_x = ((u * atlas_width) as u32).min(atlas.width().saturating_sub(1));
            let from = ((source_y * atlas.width() + source_x) * 4) as usize;
            let to = ((row * WIDTH + column) * 4) as usize;
            let (Some(source), Some(target)) =
                (atlas.rgba().get(from..from + 4), rgba.get_mut(to..to + 3))
            else {
                continue;
            };
            let alpha = u32::from(source[3]);
            for (target, source) in target.iter_mut().zip(source) {
                *target =
                    ((u32::from(*source) * alpha + u32::from(*target) * (255 - alpha)) / 255) as u8;
            }
        }
    }
}

#[cfg(test)]
#[path = "viewer_tests.rs"]
mod tests;
