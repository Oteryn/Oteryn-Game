use super::*;
use oteryn_session::VipPresenceStatus;
#[derive(Component)]
pub(super) struct NameLabel {
    generation: u64,
    entry: u64,
}
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
    rows: Query<(Entity, &Row)>,
    mut names: Query<(&NameLabel, &mut Text, &mut TextColor), Without<Message>>,
    mut messages: Query<&mut Text, With<Message>>,
    mut controls: Query<(Entity, &mut Control)>,
    mut entered: Local<Option<u64>>,
) {
    let generation = owner.generation();
    if state.generation != Some(generation) || owner.play.is_none() {
        state.generation = Some(generation);
        state.draft = None;
        state.pending = None;
    }
    let Some(play) = &owner.play else {
        for (entity, _) in &rows {
            commands.entity(entity).despawn();
        }
        for mut message in &mut messages {
            message.0 = "VIP unavailable in this session".into();
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
    for (entity, row) in &rows {
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
                    bevy::ui_widgets::Button,
                    Control {
                        generation,
                        revision: panel.revision,
                        command: Action::Edit(entry.entry),
                    },
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
                .id();
            let name = commands
                .spawn((
                    NameLabel {
                        generation,
                        entry: entry.entry,
                    },
                    source::text(&entry.name, source::font(skins.font.clone(), false)),
                    Node {
                        width: percent(100.),
                        min_width: px(0.),
                        overflow: Overflow::clip_x(),
                        ..default()
                    },
                ))
                .id();
            commands.entity(row).add_child(name);
            commands.entity(list).add_child(row);
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
        if matches!(control.command,Action::Edit(entry) if control.generation!=generation || !entries.iter().any(|row| row.entry==entry))
        {
            continue;
        }
        if matches!(
            control.command,
            Action::Add | Action::Refresh | Action::Next(_) | Action::Close
        ) {
            control.generation = generation;
        } else if control.generation != generation {
            continue;
        }
        if matches!(
            control.command,
            Action::Add | Action::Refresh | Action::Next(_) | Action::Close | Action::Edit(_)
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
