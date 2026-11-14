use super::*;
use bevy::{
    asset::RenderAssetUsages,
    input_focus::tab_navigation::{TabGroup, TabIndex},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    text::TextCursorStyle,
    ui::InteractionDisabled,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui_bevy as source;
pub(super) struct Widgets {
    pub overlay: Entity,
    pub panel: Entity,
}
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    key: Key,
    snapshot: &AppearanceSnapshot,
    images: &mut Assets<Image>,
) -> Widgets {
    let panel = &snapshot.panel;
    let overlay = commands
        .spawn((
            Overlay,
            GameModal,
            GlobalZIndex(165),
            TabGroup {
                order: 2,
                modal: true,
            },
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ))
        .id();
    let root = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(650.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect {
                    left: px(12.),
                    right: px(12.),
                    top: px(24.),
                    bottom: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(root);
    source::dialog_caption(commands, root, "Outfit and Mount", skins.font.clone());
    let body = commands
        .spawn((
            ScrollArea,
            Node {
                max_height: px(500.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(body);
    // Confirmed appearance is always rendered separately from the editable draft.
    if let Some(current) = &panel.confirmed {
        label(
            commands,
            skins,
            body,
            &format!(
                "Current outfit: {}; addons {}; mount {}; colours {:?}",
                current.selection.outfit_key,
                current.selection.addons,
                current.selection.mount_key.as_deref().unwrap_or("None"),
                current.selection.colours,
            ),
        );
    } else {
        label(commands, skins, body, "No outfit selected");
    }
    label(
        commands,
        skins,
        body,
        &format!(
            "Preview: {}; addons {}; mount {}",
            panel.fields[0],
            panel.fields[1],
            if panel.fields[2].is_empty() {
                "None"
            } else {
                &panel.fields[2]
            }
        ),
    );
    let artwork = std::env::var_os("OTERYN_ASSET_DIR")
        .ok_or_else(|| "Appearance artwork unavailable".to_owned())
        .and_then(|root| snapshot.preview_pixels(std::path::Path::new(&root)));
    match artwork {
        Ok(pixels) => {
            let handle = images.add(Image::new(
                Extent3d {
                    width: 128,
                    height: 128,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                pixels,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            ));
            let image = commands
                .spawn((
                    ImageNode::new(handle),
                    Node {
                        width: px(128.),
                        height: px(128.),
                        ..default()
                    },
                ))
                .id();
            commands.entity(body).add_child(image);
        }
        Err(message) => label(commands, skins, body, &message),
    }
    if let Some(catalogue) = &panel.catalogue {
        label(commands, skins, body, "Outfits");
        let outfits = row(commands, body);
        for outfit in &catalogue.outfits {
            button(
                commands,
                skins,
                outfits,
                key,
                Kind::Outfit(outfit.key.clone()),
                &outfit.key,
                panel.pending,
            );
        }
        if let Some(outfit) = catalogue
            .outfits
            .iter()
            .find(|outfit| outfit.key == panel.fields[0])
        {
            let addons = row(commands, body);
            for shift in 0..8 {
                let bit = 1_u8 << shift;
                if outfit.permitted_addons & bit != 0 {
                    button(
                        commands,
                        skins,
                        addons,
                        key,
                        Kind::Addon(bit),
                        &format!(
                            "Addon {}{}",
                            shift + 1,
                            if panel.fields[1].parse::<u8>().unwrap_or(0) & bit != 0 {
                                " ✓"
                            } else {
                                ""
                            }
                        ),
                        panel.pending,
                    );
                }
            }
        }
        label(commands, skins, body, "Mounts");
        let mounts = row(commands, body);
        button(
            commands,
            skins,
            mounts,
            key,
            Kind::Mount(None),
            "None",
            panel.pending,
        );
        for mount in &catalogue.mounts {
            button(
                commands,
                skins,
                mounts,
                key,
                Kind::Mount(Some(mount.clone())),
                mount,
                panel.pending,
            );
        }
        for (part, name) in ["Head", "Body", "Legs", "Feet"].into_iter().enumerate() {
            label(commands, skins, body, name);
            let palette = row(commands, body);
            for (index, rgb) in catalogue.palette.iter().enumerate() {
                let Ok(index) = u16::try_from(index) else {
                    break;
                };
                let swatch = button(
                    commands,
                    skins,
                    palette,
                    key,
                    Kind::Colour { part, index },
                    &index.to_string(),
                    panel.pending,
                );
                let colour = commands
                    .spawn((
                        Node {
                            width: px(12.),
                            height: px(12.),
                            ..default()
                        },
                        BackgroundColor(Color::srgb_u8(rgb[0], rgb[1], rgb[2])),
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(swatch).add_child(colour);
            }
        }
    } else {
        label(
            commands,
            skins,
            body,
            "Waiting for the appearance catalogue",
        );
    }
    label(commands, skins, body, "Content revision");
    let mut editable = EditableText::new(&panel.fields[4]);
    editable.max_characters = Some(128);
    let field = commands
        .spawn((
            Revision,
            TextInput,
            editable,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(0),
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: px(240.),
                height: px(25.),
                padding: px(4.).all(),
                ..default()
            },
        ))
        .id();
    if panel.pending {
        commands.entity(field).insert(InteractionDisabled);
    }
    commands.entity(body).add_child(field);
    if panel.pending {
        label(commands, skins, body, "Waiting for the server");
    }
    if !panel.feedback.is_empty() {
        label(commands, skins, body, &panel.feedback);
    }
    let footer = row(commands, root);
    button(
        commands,
        skins,
        footer,
        key,
        Kind::Apply(String::new()),
        "Apply",
        panel.pending || panel.catalogue.is_none(),
    );
    button(commands, skins, footer, key, Kind::Close, "Close", false);
    Widgets {
        overlay,
        panel: root,
    }
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let row = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            row_gap: px(3.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(row);
    row
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, value: &str) {
    let text = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(text);
}
#[allow(clippy::too_many_arguments)] // Controls carry the exact snapshot key and their server-owned choice.
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    kind: Kind,
    caption: &str,
    disabled: bool,
) -> Entity {
    let mut button = commands.spawn((
        source::button(skins.buttons.clone(), 110.),
        Control { key, kind },
    ));
    if disabled {
        button.insert(InteractionDisabled);
    }
    let button = button.observe(activate).id();
    let text = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(button).add_child(text);
    commands.entity(parent).add_child(button);
    button
}
