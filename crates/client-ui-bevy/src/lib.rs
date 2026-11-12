//! Native presentation primitives reconstructed from the Global QML controls.
//! Callers own assets, focus scopes, actions and all application state.

use bevy::{
    input_focus::tab_navigation::TabIndex,
    prelude::*,
    text::{FontHinting, FontSize, FontSmoothing, LineHeight},
    ui::{Checked, InteractionDisabled, Pressed, UiSystems},
    ui_widgets::{Button, Checkbox},
};

mod frames;
pub use frames::{Frame, frame};

pub const TEXT_COLOR: Color = Color::srgb_u8(192, 192, 192);
pub const DISABLED_TEXT_COLOR: Color = Color::srgb_u8(112, 112, 112);

/// TibiaStyle defaultTextFont / smallFont. Supply a verified bold face.
/// Qt's smallFont letterSpacing=0.1 has no TextFont equivalent in Bevy 0.20.1.
pub fn font(bold_face: Handle<Font>, small: bool) -> TextFont {
    TextFont {
        font: bold_face.into(),
        font_size: FontSize::Px(if small { 8. } else { 11. }),
        font_smoothing: if small {
            FontSmoothing::AntiAliased
        } else {
            FontSmoothing::None
        },
        ..default()
    }
}

pub fn text(value: impl Into<String>, font: TextFont) -> impl Bundle {
    let small = font.font_size == FontSize::Px(8.);
    (
        Text::new(value),
        font,
        FontHinting::Enabled,
        if small {
            LineHeight::RelativeToFont(1.)
        } else {
            LineHeight::Px(13.)
        },
        TextColor(TEXT_COLOR),
        TextLayout::no_wrap(),
        Pickable::IGNORE,
    )
}

#[derive(Component, Clone)]
pub struct ButtonImages {
    pub up: Handle<Image>,
    pub down: Handle<Image>,
}

/// Native Activate events are routed by the caller; Checked is projection state.
pub fn button(images: ButtonImages, width: f32) -> impl Bundle {
    (
        Button,
        TabIndex(0),
        frame(images.up.clone(), [1.; 4]),
        images,
        Node {
            width: px(width),
            height: px(20.),
            border: px(1.).all(),
            flex_shrink: 0.,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
    )
}

#[derive(Component)]
pub struct ButtonCaption;

/// Source icon anchors in logical pixels: images shift right/down; arrows shift right.
#[derive(Component)]
pub enum ButtonIcon {
    Left(f32),
    Right(f32),
}

pub fn button_caption(value: impl Into<String>, bold_face: Handle<Font>) -> impl Bundle {
    (
        text(value, font(bold_face, true)),
        ButtonCaption,
        Node {
            flex_shrink: 0.,
            ..default()
        },
    )
}

/// A framed option has a 1px border plus 4px padding: 5px to the 12px indicator.
pub fn checkbox_row(framed: bool) -> Node {
    Node {
        min_height: px(if framed { 22. } else { 16. }),
        border: px(if framed { 1. } else { 0. }).all(),
        padding: if framed {
            px(4.).horizontal()
        } else {
            UiRect::default()
        },
        column_gap: px(4.),
        align_items: AlignItems::Center,
        flex_shrink: 0.,
        ..default()
    }
}

pub fn checkbox(node: Node) -> impl Bundle {
    (Checkbox, TabIndex(0), node)
}

pub fn checkbox_indicator(image: Handle<Image>) -> impl Bundle {
    (
        ImageNode::new(image),
        Node {
            width: px(12.),
            height: px(12.),
            flex_shrink: 0.,
            ..default()
        },
        Pickable::IGNORE,
    )
}

/// OptionsMenuDialog: one 129x470 frame; 2px border + 5px margin leaves 115px.
pub fn navigation_outer() -> Node {
    Node {
        width: px(129.),
        height: px(470.),
        border: px(2.).all(),
        padding: px(5.).all(),
        flex_direction: FlexDirection::Column,
        flex_shrink: 0.,
        row_gap: px(10.),
        ..default()
    }
}

pub fn navigation_group(visible_rows: usize) -> Node {
    Node {
        width: px(115.),
        height: px(visible_rows as f32 * 20. + 2.),
        border: px(1.).all(),
        flex_direction: FlexDirection::Column,
        flex_shrink: 0.,
        row_gap: px(0.),
        ..default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationArrow {
    Down,
    Right,
}

/// MenuWithSubMenu with indentSubMenuEntries=false (Options' source default).
pub fn navigation_arrow(
    first: bool,
    has_children: bool,
    active: bool,
    checked: bool,
) -> Option<NavigationArrow> {
    if checked {
        Some(NavigationArrow::Right)
    } else if first && has_children && !active {
        Some(NavigationArrow::Down)
    } else {
        None
    }
}

/// Updates presentation only. No reducer, preference store, input capture or window.
pub struct GlobalUiPlugin;
impl Plugin for GlobalUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Assets<Image>>()
            .add_systems(PostUpdate, (button_skins, button_captions, button_icons))
            .add_systems(
                PostUpdate,
                frames::project
                    .after(button_skins)
                    .after(UiSystems::Layout)
                    .before(UiSystems::Stack),
            );
    }
}

fn button_icons(
    buttons: Query<(Has<Pressed>, Has<Checked>), With<ButtonImages>>,
    mut icons: Query<(&ChildOf, &ButtonIcon, &mut Node)>,
) {
    for (parent, anchor, mut node) in &mut icons {
        if let Ok((pressed, checked)) = buttons.get(parent.parent()) {
            let offset = if pressed || checked { 1. } else { 0. };
            match anchor {
                ButtonIcon::Left(x) => node.left = px(x + offset),
                ButtonIcon::Right(x) => node.right = px(x - offset),
            }
            // A top margin of 2px shifts the centered image by 1px. Menu
            // arrows only change their right margin in MenuWithSubMenu.
            node.margin.top = px(match anchor {
                ButtonIcon::Left(_) => offset * 2.,
                ButtonIcon::Right(_) => 0.,
            });
        }
    }
}

fn button_skins(mut buttons: Query<(&ButtonImages, Has<Pressed>, Has<Checked>, &mut ImageNode)>) {
    for (images, pressed, checked, mut image) in &mut buttons {
        let selected = pressed || checked;
        let handle = if selected { &images.down } else { &images.up };
        if image.image != *handle {
            image.image = handle.clone();
        }
        // This image holds the projected source; cropped child images draw the
        // chrome. The source has no invented hover brightness multiplier.
        if image.color != Color::NONE {
            image.color = Color::NONE;
        }
    }
}

type ButtonState = (Has<Pressed>, Has<Checked>, Has<InteractionDisabled>);

fn button_captions(
    buttons: Query<ButtonState, With<ButtonImages>>,
    mut captions: Query<(&ChildOf, &mut Node, &mut TextColor), With<ButtonCaption>>,
) {
    for (parent, mut node, mut color) in &mut captions {
        if let Ok((pressed, checked, disabled)) = buttons.get(parent.parent()) {
            // Qt's 2px left/top pressed padding moves a centered caption by 1px.
            let offset = px(if pressed || checked { 1. } else { 0. });
            if node.left != offset {
                node.left = offset;
            }
            if node.top != offset {
                node.top = offset;
            }
            let next = if disabled {
                DISABLED_TEXT_COLOR
            } else {
                TEXT_COLOR
            };
            if color.0 != next {
                color.0 = next;
            }
        }
    }
}
