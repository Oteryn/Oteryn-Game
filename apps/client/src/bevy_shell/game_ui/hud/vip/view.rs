use super::*;
use bevy::picking::hover::Hovered;
use oteryn_session::VipPresenceStatus;
pub(super) fn setup(
    mut commands: Commands,
    regions: Res<super::super::super::GameRegions>,
    skins: Res<Skins>,
) {
    let header = commands
        .spawn(Node {
            width: percent(100.),
            column_gap: px(3.),
            align_items: AlignItems::Center,
            ..default()
        })
        .id();
    let title = commands
        .spawn((
            source::text("VIP", source::font(skins.font.clone(), false)),
            Node {
                flex_grow: 1.,
                ..default()
            },
        ))
        .id();
    commands.entity(header).add_child(title);
    for (label, command, id) in [
        ("Add", Action::Add, "vip.add"),
        ("↻", Action::Refresh, "vip.refresh"),
        ("Next", Action::Next(0), "vip.next"),
        ("×", Action::Close, "vip.close"),
    ] {
        let button = button(
            &mut commands,
            &skins,
            Control {
                generation: 0,
                revision: 0,
                command,
            },
            label,
            if matches!(command, Action::Close | Action::Refresh) {
                18.
            } else {
                30.
            },
            id,
        );
        commands.entity(header).add_child(button);
    }
    let list = commands
        .spawn((
            List,
            ScrollArea,
            Node {
                width: percent(100.),
                min_height: px(0.),
                max_height: px(200.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(3.),
                padding: UiRect::all(px(2.)),
                ..default()
            },
        ))
        .id();
    groups::setup(&mut commands, &skins, list);
    let message = commands
        .spawn((
            Message,
            source::text("", source::font(skins.font.clone(), true)),
            Node {
                width: percent(100.),
                ..default()
            },
        ))
        .insert(TextLayout::default())
        .id();
    commands
        .entity(regions.vip)
        .add_children(&[header, list, message]);
}
// The retained entry list, action state and labels are independently mutable ECS roles.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    lists: Query<Entity, With<List>>,
    rows: Query<(Entity, &Row, &Hovered, &ComputedNode, &UiGlobalTransform)>,
    mut names: Query<(&VipNameLabel, &mut Text, &mut TextColor), Without<Message>>,
    mut messages: Query<&mut Text, With<Message>>,
    mut controls: Query<(Entity, &mut Control)>,
    mut entered: Local<Option<u64>>,
    metadata: Query<(Entity, &VipMetadata, &Text), (Without<VipNameLabel>, Without<Message>)>,
    guard: ModalGuard,
) {
    let generation = owner.generation();
    if state.generation != Some(generation) || owner.play.is_none() {
        state.generation = Some(generation);
        state.draft = None;
        state.pending = None;
    }
    let Some(play) = &owner.play else {
        for (entity, _, _, _, _) in &rows {
            commands.entity(entity).despawn();
        }
        for mut message in &mut messages {
            message.0 = "VIP unavailable in this session".into();
        }
        for (entity, role, _) in &metadata {
            if role.description {
                commands.entity(entity).despawn();
            }
        }
        *entered = None;
        return;
    };
    if *entered != Some(generation) {
        play.link.vip_request(Intent::Query(0));
        *entered = Some(generation);
    }
    let panel = play.link.vip_panel();
    let snapshot = Snapshot::read(&panel);
    let entries = snapshot
        .as_ref()
        .map(|value| value.entries.as_slice())
        .unwrap_or(&[]);
    let presence = play.link.vip_presence_panel();
    let message = if !panel.selected {
        "VIP unavailable in this session".to_owned()
    } else if let Some(feedback) = &panel.feedback {
        feedback.clone()
    } else if panel.pending {
        "Updating…".to_owned()
    } else {
        snapshot.as_ref().err().copied().unwrap_or("").to_owned()
    };
    for mut text in &mut messages {
        if text.0 != message {
            text.0.clone_from(&message);
        }
    }
    let mut retained = std::collections::HashSet::new();
    for (entity, row, _, _, _) in &rows {
        if row.generation != generation || !entries.iter().any(|entry| entry.entry == row.entry) {
            commands.entity(entity).despawn();
        } else {
            retained.insert(row.entry);
        }
    }
    if let Ok(list) = lists.single() {
        for entry in entries
            .iter()
            .filter(|entry| !retained.contains(&entry.entry))
        {
            let row = commands
                .spawn((
                    Row {
                        generation,
                        entry: entry.entry,
                    },
                    Control {
                        generation,
                        revision: panel.revision,
                        command: Action::Edit(entry.entry),
                    },
                    Pickable::default(),
                    Hovered::default(),
                    Name::new("vip.entry"),
                    TabIndex(0),
                    Node {
                        width: percent(100.),
                        min_height: px(16.),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .observe(input::activate)
                .observe(presentation::click)
                .observe(presentation::key)
                .observe(row_click)
                .id();
            let glyph = commands
                .spawn((
                    VipMetadata {
                        generation,
                        entry: entry.entry,
                        description: false,
                    },
                    Name::new(format!("vip.icon.{}", entry.entry)),
                    source::text(
                        vip_icon_text(entry.icon),
                        source::font(skins.font.clone(), false),
                    ),
                    // Text alternative of the artwork icon; the ImageNode below is what is drawn.
                    Node {
                        display: Display::None,
                        ..default()
                    },
                ))
                .id();
            commands.spawn((
                VipMetadata {
                    generation,
                    entry: entry.entry,
                    description: true,
                },
                Name::new(format!("vip.description.{}", entry.entry)),
                source::text(
                    entry.description.as_deref().unwrap_or(""),
                    source::font(skins.font.clone(), false),
                ),
                Node {
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(Color::srgb_u8(32, 32, 32)),
                GlobalZIndex(280),
            ));
            let name = commands
                .spawn((
                    VipNameLabel {
                        generation,
                        entry: entry.entry,
                    },
                    source::text(&entry.name, source::font(skins.font.clone(), false)),
                    Node {
                        flex_grow: 1.,
                        min_width: px(0.),
                        overflow: Overflow::clip_x(),
                        ..default()
                    },
                ))
                .id();
            let icon = commands
                .spawn((
                    presentation::Icon(entry.entry),
                    ImageNode::default(),
                    Name::new("vip.icon"),
                    Node {
                        width: px(12.),
                        height: px(12.),
                        flex_shrink: 0.,
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .id();
            let context = button(
                &mut commands,
                &skins,
                Control {
                    generation,
                    revision: panel.revision,
                    command: Action::Context(entry.entry),
                },
                "…",
                18.,
                "vip.context",
            );
            commands
                .entity(row)
                .add_children(&[icon, glyph, name, context]);
            commands.entity(list).add_child(row);
        }
    }
    for (entity, role, text) in &metadata {
        let entry = entries
            .iter()
            .find(|entry| entry.entry == role.entry && role.generation == generation);
        let Some(entry) = entry else {
            if role.description {
                commands.entity(entity).despawn();
            }
            continue;
        };
        let value = if role.description {
            entry.description.as_deref().unwrap_or("")
        } else {
            vip_icon_text(entry.icon)
        };
        if text.0 != value {
            commands.entity(entity).insert(Text::new(value));
        }
        if role.description {
            let at = rows
                .iter()
                .find(|(entity, row, interaction, _, _)| {
                    row.entry == role.entry
                        && interaction.get()
                        && guard.allows(*entity)
                        && !owner.modal_active()
                })
                .map(|(_, _, _, node, transform)| {
                    (transform.translation + node.size() * Vec2::new(-0.5, 0.5))
                        * node.inverse_scale_factor
                });
            commands.entity(entity).insert(Node {
                display: if at.is_some() && !value.is_empty() {
                    Display::Flex
                } else {
                    Display::None
                },
                position_type: PositionType::Absolute,
                left: px(at.unwrap_or_default().x),
                top: px(at.unwrap_or_default().y + 3.),
                max_width: px(240.),
                padding: UiRect::all(px(5.)),
                ..default()
            });
        }
    }
    for (label, mut text, mut color) in &mut names {
        if label.generation != generation {
            continue;
        }
        let Some(entry) = entries.iter().find(|entry| entry.entry == label.entry) else {
            continue;
        };
        if text.0 != entry.name {
            text.0.clone_from(&entry.name);
        }
        let status = presence
            .presence
            .entries
            .iter()
            .find(|value| value.entry == entry.entry)
            .map(|value| value.status);
        // TibiaStyle VipOnline/VipOffline/VipPending; no inferred online status.
        color.0 = match status {
            Some(VipPresenceStatus::Online) => Color::srgb_u8(96, 248, 96),
            Some(VipPresenceStatus::Offline) => Color::srgb_u8(248, 96, 96),
            _ => Color::srgb_u8(255, 165, 0),
        };
    }
    for (entity, mut control) in &mut controls {
        // Retiring rows are already queued for despawn; never queue more component writes.
        if matches!(control.command,Action::Edit(entry) | Action::Context(entry) if control.generation!=generation || !entries.iter().any(|row| row.entry==entry))
        {
            continue;
        }
        if matches!(
            control.command,
            Action::Add
                | Action::Refresh
                | Action::Next(_)
                | Action::Close
                | Action::Sort
                | Action::HideOffline
        ) {
            control.generation = generation;
        } else if control.generation != generation {
            continue;
        }
        if matches!(
            control.command,
            Action::Add
                | Action::Refresh
                | Action::Next(_)
                | Action::Close
                | Action::Sort
                | Action::HideOffline
                | Action::Edit(_)
                | Action::Context(_)
        ) {
            control.revision = panel.revision;
            if matches!(control.command, Action::Next(_)) {
                let cursor = snapshot
                    .as_ref()
                    .ok()
                    .and_then(|snapshot| snapshot.next_entry)
                    .filter(|cursor| *cursor != 0);
                control.command = Action::Next(cursor.unwrap_or(0));
                commands.entity(entity).insert(if cursor.is_some() {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
            let blocked = !available(&owner)
                || (!matches!(control.command, Action::Close)
                    && (!panel.selected
                        || panel.pending
                        || (!matches!(control.command, Action::Refresh) && snapshot.is_err())));
            if blocked {
                commands.entity(entity).insert(InteractionDisabled);
            } else {
                commands.entity(entity).remove::<InteractionDisabled>();
            }
        }
    }
}
#[allow(clippy::too_many_arguments)] // Modal lifecycle and editor projection stay independent of retained contact rows.
pub(super) fn dialog(
    mut commands: Commands,
    skins: Res<Skins>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    mut previous: Local<Option<(u64, Entity)>>,
    fields: Query<(Entity, &Field)>,
    mut labels: Query<(&DraftLabel, &mut Text)>,
    toggles: Query<(Entity, &Toggle, Has<bevy::ui::Checked>)>,
    mut indicators: Query<(&ToggleIndicator, &mut ImageNode)>,
    mut focus: ResMut<InputFocus>,
) {
    if state
        .draft
        .as_ref()
        .is_some_and(|draft| !current(&owner, draft))
    {
        state.draft = None;
        state.pending = None;
    }
    let serial = state.draft.as_ref().map(|draft| draft.serial);
    if previous.as_ref().map(|(serial, _)| *serial) != serial {
        if let Some((_, root)) = previous.take() {
            if focus.get().is_some_and(|entity| fields.contains(entity)) {
                focus.clear();
            }
            commands.entity(root).despawn();
        }
        if let Some(draft) = &state.draft {
            let (root, field) = super::dialog::build(&mut commands, &skins, draft);
            *previous = Some((draft.serial, root));
            focus.set(field, FocusCause::Navigated);
        }
    }
    let Some(draft) = &state.draft else { return };
    let checked = |label: Label| match label {
        Label::Notify => draft.notify == Some(true),
        Label::Group(group) => draft
            .groups
            .as_ref()
            .is_some_and(|groups| groups.contains(&group)),
        _ => false,
    };
    for (entity, toggle, installed) in &toggles {
        let value = checked(toggle.0);
        if value && !installed {
            commands.entity(entity).insert(bevy::ui::Checked);
        } else if !value && installed {
            commands.entity(entity).remove::<bevy::ui::Checked>();
        }
    }
    for (indicator, mut image) in &mut indicators {
        image.image = if checked(indicator.0) {
            skins.checked.clone()
        } else {
            skins.unchecked.clone()
        };
    }
    for (label, mut text) in &mut labels {
        let value = match label.0 {
            Label::Icon => format!(
                "Icon: {}",
                draft
                    .icon
                    .map_or_else(|| "Unavailable".into(), |value| value.to_string())
            ),
            Label::Notify => "Notify on login".to_owned(),
            Label::Group(group) => draft
                .catalog
                .iter()
                .find(|row| row.group == group)
                .map(|row| row.name.clone())
                .unwrap_or_default(),
            Label::Feedback => draft.feedback.clone(),
        };
        if text.0 != value {
            text.0 = value;
        }
    }
}

#[derive(Component, Clone, Copy)]
struct PrivateMessage {
    generation: u64,
    revision: u64,
    entry: u64,
}
fn message(owner: &HostOwner, target: PrivateMessage) {
    if !available(owner) || target.generation != owner.generation() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.vip_panel();
    if !panel.selected || panel.pending || panel.revision != target.revision {
        return;
    }
    if let Ok(snapshot) = Snapshot::read(&panel)
        && let Some(contact) = snapshot
            .entries
            .iter()
            .find(|contact| contact.entry == target.entry)
    {
        play.link.chat_open_conversation(&contact.name);
    }
}
fn row_click(
    event: On<PointerClick>,
    mut commands: Commands,
    rows: Query<&Row>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
) {
    use bevy::picking::pointer::PointerButton;
    let Ok(row) = rows.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity) || !available(&owner) || row.generation != owner.generation() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.vip_panel();
    if !panel.selected || panel.pending {
        return;
    }
    let target = PrivateMessage {
        generation: row.generation,
        revision: panel.revision,
        entry: row.entry,
    };
    let swapped = (owner.settings.preferences.controls.mouse
        == crate::play::MousePreset::LeftHanded)
        ^ owner.settings.preferences.swap_mouse_buttons;
    let primary = if swapped {
        PointerButton::Secondary
    } else {
        PointerButton::Primary
    };
    // Secondary clicks open the shared presentation context menu (`presentation::click`).
    if event.button == primary && event.count >= 2 {
        message(&owner, target);
        // The first click may have queued an editor before the double click arrived.
        commands.queue(move |world: &mut World| {
            let mut state = world.resource_mut::<State>();
            if state.draft.as_ref().is_some_and(|draft| {
                draft.generation == target.generation
                    && draft.revision == target.revision
                    && draft
                        .contact
                        .as_ref()
                        .is_some_and(|contact| contact.entry == target.entry)
            }) {
                state.draft = None;
                state.pending = None;
            }
        });
    }
}
