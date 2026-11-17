//! The Tibia Global popup menu: anchored at the cursor, sized to its entries, kept inside the
//! window. A click outside closes it through the caller's scrim observer; there is no Cancel.
use bevy::{picking::hover::Hovered, prelude::*};
use oteryn_client_ui_bevy as source;

// Tibia Global PopupMenu QML: entry height 16, spacing 3, shortcut at least 40 px after the label.
pub(in crate::bevy_shell) const ENTRY_HEIGHT: f32 = 16.;
pub(in crate::bevy_shell) const ENTRY_SPACING: f32 = 3.;
pub(in crate::bevy_shell) const SHORTCUT_GAP: f32 = 40.;
const PADDING: f32 = 4.;
const CHECK_SIZE: f32 = 12.;

/// One row of a popup menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::bevy_shell) enum Item {
    Entry {
        label: String,
        /// The modifier of the default control scheme that runs this entry by click.
        shortcut: Option<&'static str>,
        /// `Some` for a checkable entry.
        checked: Option<bool>,
    },
    Separator,
}
impl Item {
    pub(in crate::bevy_shell) fn entry(label: impl Into<String>) -> Self {
        Self::Entry {
            label: label.into(),
            shortcut: None,
            checked: None,
        }
    }
    pub(in crate::bevy_shell) fn shortcut(
        label: impl Into<String>,
        shortcut: &'static str,
    ) -> Self {
        Self::Entry {
            label: label.into(),
            shortcut: Some(shortcut),
            checked: None,
        }
    }
    pub(in crate::bevy_shell) fn check(label: impl Into<String>, checked: bool) -> Self {
        Self::Entry {
            label: label.into(),
            shortcut: None,
            checked: Some(checked),
        }
    }
}

/// The images a popup draws with.
pub(in crate::bevy_shell) struct Look {
    pub(in crate::bevy_shell) font: Handle<Font>,
    pub(in crate::bevy_shell) frame: Handle<Image>,
    pub(in crate::bevy_shell) checked: Option<Handle<Image>>,
    pub(in crate::bevy_shell) unchecked: Option<Handle<Image>>,
}

/// The menu's frame; `anchor` is the cursor position it opened at, in logical window pixels.
#[derive(Component, Debug)]
pub(in crate::bevy_shell) struct PopupMenu {
    pub(in crate::bevy_shell) anchor: Vec2,
}
/// The full-window layer under the menu that takes the click outside it.
#[derive(Component)]
pub(in crate::bevy_shell) struct PopupScrim;
/// A selectable entry; the index counts entries only, separators excluded.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::bevy_shell) struct PopupEntry(pub(in crate::bevy_shell) usize);
#[derive(Component)]
pub(in crate::bevy_shell) struct PopupShortcut;

/// The spawned menu: its scrim, its frame and one entity per entry in order.
pub(in crate::bevy_shell) struct Popup {
    pub(in crate::bevy_shell) scrim: Entity,
    pub(in crate::bevy_shell) root: Entity,
    pub(in crate::bevy_shell) entries: Vec<Entity>,
}
/// The top-left corner that keeps a menu of `size` opened at `anchor` inside `window`.
pub(in crate::bevy_shell) fn clamp(anchor: Vec2, size: Vec2, window: Vec2) -> Vec2 {
    Vec2::new(
        anchor.x.min(window.x - size.x).max(0.),
        anchor.y.min(window.y - size.y).max(0.),
    )
}

/// Spawns a menu at `anchor` above everything but other popups. The caller attaches its
/// actions to the entry entities and its close observer to the scrim.
pub(in crate::bevy_shell) fn spawn(
    commands: &mut Commands,
    look: &Look,
    anchor: Vec2,
    items: &[Item],
) -> Popup {
    let scrim = commands
        .spawn((
            PopupScrim,
            GlobalZIndex(260),
            Node {
                position_type: PositionType::Absolute,
                left: px(0.),
                top: px(0.),
                width: percent(100.),
                height: percent(100.),
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.01)),
        ))
        .id();
    let root = commands
        .spawn((
            PopupMenu { anchor },
            GlobalZIndex(261),
            source::frame(look.frame.clone(), [PADDING; 4]),
            Node {
                position_type: PositionType::Absolute,
                left: px(anchor.x),
                top: px(anchor.y),
                padding: UiRect::all(px(PADDING)),
                flex_direction: FlexDirection::Column,
                row_gap: px(ENTRY_SPACING),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(54, 54, 54)),
        ))
        .id();
    let mut entries = Vec::new();
    for item in items {
        match item {
            Item::Separator => {
                let line = commands
                    .spawn((
                        Node {
                            height: px(1.),
                            width: percent(100.),
                            ..default()
                        },
                        BackgroundColor(Color::srgb_u8(112, 112, 112)),
                    ))
                    .id();
                commands.entity(root).add_child(line);
            }
            Item::Entry {
                label,
                shortcut,
                checked,
            } => {
                let entry = commands
                    .spawn((
                        PopupEntry(entries.len()),
                        bevy::ui_widgets::Button,
                        Hovered::default(),
                        Node {
                            height: px(ENTRY_HEIGHT),
                            min_height: px(ENTRY_HEIGHT),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: px(4.),
                            padding: UiRect::horizontal(px(2.)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                    ))
                    .id();
                if let Some(checked) = checked {
                    let image = if *checked {
                        look.checked.clone()
                    } else {
                        look.unchecked.clone()
                    };
                    let check = commands
                        .spawn((
                            Pickable::IGNORE,
                            Node {
                                width: px(CHECK_SIZE),
                                height: px(CHECK_SIZE),
                                flex_shrink: 0.,
                                ..default()
                            },
                        ))
                        .id();
                    match image {
                        Some(image) => {
                            commands.entity(check).insert(ImageNode::new(image));
                        }
                        None => {
                            let mark = commands
                                .spawn((source::text(
                                    if *checked { "x" } else { "" },
                                    source::font(look.font.clone(), false),
                                ),))
                                .id();
                            commands.entity(check).add_child(mark);
                        }
                    }
                    commands.entity(entry).add_child(check);
                }
                let caption = commands
                    .spawn((
                        source::text(label.as_str(), source::font(look.font.clone(), false)),
                        Node {
                            flex_grow: 1.,
                            ..default()
                        },
                    ))
                    .id();
                commands.entity(entry).add_child(caption);
                if let Some(shortcut) = shortcut {
                    let hint = commands
                        .spawn((
                            PopupShortcut,
                            source::text(*shortcut, source::font(look.font.clone(), false)),
                            Node {
                                margin: UiRect::left(px(SHORTCUT_GAP)),
                                flex_shrink: 0.,
                                ..default()
                            },
                        ))
                        .id();
                    commands.entity(entry).add_child(hint);
                }
                commands.entity(root).add_child(entry);
                entries.push(entry);
            }
        }
    }
    Popup {
        scrim,
        root,
        entries,
    }
}

/// Keeps every open menu inside the window once its size is known, and lights the hovered
/// entry.
pub(in crate::bevy_shell) fn layout(
    windows: Query<&Window>,
    scale: Option<Res<UiScale>>,
    mut menus: Query<(&PopupMenu, &ComputedNode, &mut Node)>,
    mut entries: Query<(&Hovered, &mut BackgroundColor), With<PopupEntry>>,
) {
    if let Ok(window) = windows.single() {
        let scale = scale.as_deref().map_or(1., |scale| scale.0);
        let bounds = Vec2::new(window.width(), window.height()) / scale;
        for (menu, computed, mut node) in &mut menus {
            let size = computed.size() * computed.inverse_scale_factor;
            let at = clamp(menu.anchor / scale, size, bounds);
            if node.left != px(at.x) {
                node.left = px(at.x);
            }
            if node.top != px(at.y) {
                node.top = px(at.y);
            }
        }
    }
    for (hovered, mut background) in &mut entries {
        let color = if hovered.get() {
            Color::srgb_u8(98, 98, 98)
        } else {
            Color::NONE
        };
        if background.0 != color {
            background.0 = color;
        }
    }
}

/// The cursor position in logical window pixels, or the window centre when it is outside.
pub(in crate::bevy_shell) fn cursor(windows: &Query<&Window>) -> Vec2 {
    windows.single().map_or(Vec2::ZERO, |window| {
        window
            .cursor_position()
            .unwrap_or_else(|| Vec2::new(window.width(), window.height()) / 2.)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_menu_clamps_in_ui_units() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new();
        app.insert_resource(UiScale(2.)).add_systems(Update, layout);
        app.world_mut().spawn(Window {
            resolution: (1024, 768).into(),
            ..default()
        });
        let root = app
            .world_mut()
            .spawn((
                PopupMenu {
                    anchor: Vec2::new(1000., 100.),
                },
                Node::default(),
                ComputedNode {
                    size: Vec2::new(240., 80.),
                    inverse_scale_factor: 0.5,
                    ..default()
                },
            ))
            .id();
        app.update();
        let node = app.world().get::<Node>(root).ok_or("menu")?;
        assert_eq!((node.left, node.top), (px(392.), px(50.)));
        Ok(())
    }

    #[test]
    fn a_menu_near_the_edges_stays_inside_the_window() {
        let window = Vec2::new(800., 600.);
        let size = Vec2::new(120., 80.);
        assert_eq!(
            clamp(Vec2::new(10., 20.), size, window),
            Vec2::new(10., 20.)
        );
        assert_eq!(
            clamp(Vec2::new(790., 590.), size, window),
            Vec2::new(680., 520.)
        );
        // A menu larger than the window keeps its top-left corner visible.
        assert_eq!(
            clamp(Vec2::new(50., 50.), Vec2::new(900., 700.), window),
            Vec2::ZERO
        );
    }

    #[test]
    fn entries_are_sixteen_high_with_shortcut_after_a_forty_pixel_gap()
    -> Result<(), Box<dyn std::error::Error>> {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        let font = Handle::default();
        let look = Look {
            font,
            frame: Handle::default(),
            checked: None,
            unchecked: None,
        };
        let popup = app
            .world_mut()
            .run_system_once(move |mut commands: Commands| {
                spawn(
                    &mut commands,
                    &look,
                    Vec2::new(5., 6.),
                    &[
                        Item::shortcut("Look", "Shift"),
                        Item::Separator,
                        Item::check("Lock Action Bar", true),
                    ],
                )
            })
            .map_err(|error| format!("popup fixture: {error:?}"))?;
        assert_eq!(popup.entries.len(), 2);
        let world = app.world();
        for entry in &popup.entries {
            let node = world.get::<Node>(*entry);
            assert_eq!(node.map(|node| node.height), Some(px(ENTRY_HEIGHT)));
        }
        let root = world.get::<Node>(popup.root);
        assert_eq!(root.map(|node| node.row_gap), Some(px(ENTRY_SPACING)));
        assert_eq!(root.map(|node| node.left), Some(px(5.)));
        let hints: Vec<_> = world
            .iter_entities()
            .filter(|entity| entity.contains::<PopupShortcut>())
            .filter_map(|entity| entity.get::<Node>().map(|node| node.margin.left))
            .collect();
        assert_eq!(hints, [px(SHORTCUT_GAP)]);
        assert_eq!(
            world.get::<PopupEntry>(popup.entries[1]),
            Some(&PopupEntry(1))
        );
        Ok(())
    }
}
