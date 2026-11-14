use super::*;
use bevy::{input_focus::tab_navigation::TabIndex, text::TextCursorStyle};
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    generation: u64,
    serial: u64,
    p: &Panel,
    images: &mut Assets<Image>,
) -> Entity {
    let root = commands
        .spawn((
            GlobalZIndex(164),
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            crate::bevy_shell::window_drag::DraggableDialog,
            Node {
                position_type: PositionType::Absolute,
                left: percent(12.),
                top: percent(8.),
                width: percent(76.),
                max_height: percent(85.),
                padding: UiRect::all(px(14.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    source::dialog_caption(commands, root, "Character Cyclopedia", skins.font.clone());
    let row = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(3.),
            row_gap: px(3.),
            ..default()
        })
        .id();
    commands.entity(root).add_child(row);
    for (id, label) in [
        (101, "General stats"),
        (102, "Recent deaths"),
        (103, "Recent PvP kills"),
        (104, "Achievements"),
        (106, "Store summary"),
        (107, "Inspection"),
        (108, "Offence"),
        (109, "Defence"),
        (110, "Misc"),
        (111, "Badges"),
        (112, "Previous page"),
        (113, "Next page"),
        (114, "All PvP results"),
        (115, "Unjustified"),
        (116, "Guild war"),
        (180, "Spell augments"),
        (181, "All sources"),
        (182, "Equipment"),
        (183, "Proficiency"),
        (184, "Wheel"),
        (185, "Sort augments"),
        (145, "All badges"),
        (146, "Earned"),
        (147, "Locked"),
        (148, "Hidden locally"),
        (152, "Public"),
        (155, "Hidden publicly"),
        (154, "Public badge visibility"),
    ] {
        button(
            commands,
            skins,
            row,
            generation,
            serial,
            Action::Native(id, Default::default()),
            label,
        );
    }
    for (intent, label) in [
        (Intent::Appearances, "Appearances"),
        (Intent::GalleryPrevious, "Previous appearances"),
        (Intent::GalleryNext, "Next appearances"),
        (Intent::Items, "Item summary"),
        (Intent::ItemsPrevious, "Previous items"),
        (Intent::ItemsNext, "Next items"),
        (Intent::Protection, "Death loss preview"),
    ] {
        button(
            commands,
            skins,
            row,
            generation,
            serial,
            Action::Request(intent),
            label,
        );
    }
    button(
        commands,
        skins,
        row,
        generation,
        serial,
        Action::Refresh,
        "Refresh",
    );
    button(
        commands,
        skins,
        row,
        generation,
        serial,
        Action::Close,
        "Close",
    );
    let target = p
        .target
        .map(|t| t.iter().map(|b| format!("{b:02x}")).collect::<String>())
        .unwrap_or_default();
    for (index, label, value) in [
        (0, "Page offset", p.offset.to_string()),
        (1, "Page size (1-50)", p.limit.to_string()),
        (2, "Target UUID (blank = self)", target),
    ] {
        text(commands, skins, root, label);
        let mut edit = EditableText::new(value);
        edit.max_characters = Some(128);
        let field = commands
            .spawn((
                Field(index),
                TextInput,
                edit,
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::no_wrap(),
                TextCursorStyle::default(),
                TabIndex(0),
                Node {
                    height: px(24.),
                    width: percent(100.),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.12, 0.12, 0.12)),
            ))
            .id();
        commands.entity(root).add_child(field);
    }
    if p.gallery.open || p.items.open {
        text(commands, skins, root, "Search (apply with a filter button)");
        let value = if p.gallery.open {
            &p.gallery.search
        } else {
            &p.items.search
        };
        let mut edit = EditableText::new(value);
        edit.max_characters = Some(128);
        let field = commands
            .spawn((
                Field(3),
                TextInput,
                edit,
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::no_wrap(),
                TextCursorStyle::default(),
                TabIndex(0),
                Node {
                    height: px(24.),
                    ..default()
                },
            ))
            .id();
        commands.entity(root).add_child(field);
    }
    for badge in ui::badge_rows(p).into_iter().filter(|b| b.earned) {
        button(
            commands,
            skins,
            row,
            generation,
            serial,
            Action::LocalBadge(badge.id),
            &format!("Hide/show {} locally", badge.name),
        );
        if p.target.is_none() && p.badge_visibility_known {
            button(
                commands,
                skins,
                row,
                generation,
                serial,
                Action::Request(Intent::SetBadgeVisibility {
                    badge: badge.id,
                    public: p.server_hidden_badges.contains(&badge.id),
                }),
                &format!("Hide/show {} publicly", badge.name),
            );
        }
    }
    if p.gallery.open {
        use oteryn_client_runtime::cyclopedia_character::gallery::Kind;
        for (kind, label) in [
            (None, "All appearances"),
            (Some(Kind::Outfit), "Outfits"),
            (Some(Kind::Mount), "Mounts"),
            (Some(Kind::Familiar), "Familiars"),
        ] {
            button(
                commands,
                skins,
                row,
                generation,
                serial,
                Action::GalleryFilter(kind, p.gallery.search.clone()),
                label,
            );
        }
    }
    if p.items.open {
        use oteryn_client_runtime::cyclopedia_character::items::Location;
        for (location, label) in [
            (None, "All locations"),
            (Some(Location::Inventory), "Inventory"),
            (Some(Location::Depot), "Depot"),
            (Some(Location::Stash), "Stash"),
            (Some(Location::Inbox), "Inbox"),
        ] {
            button(
                commands,
                skins,
                row,
                generation,
                serial,
                Action::ItemFilter(location, p.items.search.clone()),
                label,
            );
        }
    }
    let scroll = commands
        .spawn((
            ScrollArea,
            Node {
                height: px(320.),
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    let browser_open = p.gallery.open || p.items.open || p.protection.open;
    let content_parent = if browser_open {
        let browser = commands
            .spawn((
                source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
                Node {
                    width: percent(100.),
                    padding: UiRect::all(px(8.)),
                    flex_direction: FlexDirection::Column,
                    min_height: px(0.),
                    ..default()
                },
            ))
            .id();
        commands.entity(root).add_child(browser);
        let title = if p.gallery.open {
            "Appearance gallery"
        } else if p.items.open {
            "Item summary"
        } else {
            "Death loss preview"
        };
        source::dialog_caption(commands, browser, title, skins.font.clone());
        button(
            commands,
            skins,
            browser,
            generation,
            serial,
            Action::CloseBrowser,
            "Close subwindow",
        );
        browser
    } else {
        root
    };
    commands.entity(content_parent).add_child(scroll);
    text(commands, skins, scroll, &body(p));
    let bars = if p.tab == oteryn_client_runtime::cyclopedia_character::CharacterTab::Misc {
        ui::misc_bars(p)
    } else {
        ui::overview_bars(p)
    };
    if let Some(bars) = bars {
        for ((value, max), label) in bars.into_iter().zip(["Stamina", "Offline training"]) {
            text(
                commands,
                skins,
                scroll,
                &format!("{label}: {value}/{max} minutes"),
            );
            let track = commands
                .spawn((
                    source::frame(skins.inset.clone(), [1.; 4]),
                    Node {
                        width: percent(100.),
                        height: px(12.),
                        min_height: px(12.),
                        ..default()
                    },
                ))
                .id();
            let fill = commands
                .spawn((
                    Node {
                        width: percent(f32::from(value) * 100. / f32::from(max)),
                        height: percent(100.),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.25, 0.55, 0.35)),
                ))
                .id();
            commands.entity(track).add_child(fill);
            commands.entity(scroll).add_child(track);
        }
    }
    if p.gallery.open {
        match gallery_pixels::Art::load() {
            Ok(mut art) => {
                for entry in p.gallery.rows() {
                    match art.pixels(entry.look_type) {
                        Ok(pixels) => {
                            let image = images.add(Image::new(
                                bevy::render::render_resource::Extent3d {
                                    width: 96,
                                    height: 96,
                                    depth_or_array_layers: 1,
                                },
                                bevy::render::render_resource::TextureDimension::D2,
                                pixels,
                                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                                bevy::asset::RenderAssetUsages::default(),
                            ));
                            let preview = commands
                                .spawn((
                                    ImageNode::new(image),
                                    Node {
                                        width: px(96.),
                                        height: px(96.),
                                        min_height: px(96.),
                                        ..default()
                                    },
                                ))
                                .id();
                            commands.entity(scroll).add_child(preview);
                            text(commands, skins, scroll, &entry.key);
                        }
                        Err(error) => text(commands, skins, scroll, &error),
                    }
                }
            }
            Err(error) => text(commands, skins, scroll, &error),
        }
    }
    root
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: &str) {
    let child = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(child);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    generation: u64,
    serial: u64,
    action: Action,
    label: &str,
) {
    let child = commands
        .spawn((
            source::button(skins.buttons.clone(), 120.),
            Control {
                generation,
                serial,
                action,
            },
        ))
        .observe(activate)
        .id();
    text(commands, skins, child, label);
    commands.entity(parent).add_child(child);
}
