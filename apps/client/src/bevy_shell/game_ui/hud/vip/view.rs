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
            flex_wrap: FlexWrap::Wrap,
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
        ("Name", Action::Sort(0), "vip.sort.name"),
        ("Type", Action::Sort(1), "vip.sort.type"),
        ("Status", Action::Sort(2), "vip.sort.status"),
        ("Hide offline", Action::HideOffline, "vip.hide_offline"),
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
            } else if matches!(command, Action::HideOffline) {
                90.
            } else if matches!(command, Action::Sort(_)) {
                45.
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
    rows: Query<(Entity, &Row)>,
    mut names: Query<(&NameLabel, &mut Text, &mut TextColor), Without<Message>>,
    mut messages: Query<&mut Text, With<Message>>,
    mut controls: Query<(Entity, &mut Control, Has<bevy::ui::Checked>)>,
    mut entered: Local<Option<u64>>,
    mut sections: Local<std::collections::HashMap<Option<u8>, (Entity, Entity)>>,
    mut shown: Local<Option<(u64, Vec<Contact>, Vec<model::Group>)>>,
) {
    let generation = owner.generation();
    if state.generation != Some(generation) || owner.play.is_none() {
        state.generation = Some(generation);
        state.draft = None;
        state.pending = None;
        state.contact_menu = None;
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
    let groups = snapshot
        .as_ref()
        .map(|snapshot| snapshot.groups.as_slice())
        .unwrap_or(&[]);
    let current = (generation, entries.to_vec(), groups.to_vec());
    let rebuild = shown.as_ref() != Some(&current);
    if rebuild {
        for (_, (section, _)) in sections.drain() {
            commands.entity(section).despawn();
        }
        if let Ok(list) = lists.single() {
            for (id, title) in std::iter::once((None, "Ungrouped")).chain(
                groups
                    .iter()
                    .map(|group| (Some(group.group), group.name.as_str())),
            ) {
                let section = commands
                    .spawn((
                        Name::new(format!("vip.contacts.group.{}", id.unwrap_or(0))),
                        Node {
                            width: percent(100.),
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                    ))
                    .id();
                let heading = commands
                    .spawn(source::text(title, source::font(skins.font.clone(), false)))
                    .id();
                commands.entity(section).add_child(heading);
                commands.entity(list).add_child(section);
                sections.insert(id, (section, heading));
            }
        }
        *shown = Some(current);
    }
    let mut desired = Vec::new();
    for entry in entries {
        if owner.settings.preferences.vip_hide_offline
            && presence.presence.entries.iter().any(|value| {
                value.entry == entry.entry && value.status == VipPresenceStatus::Offline
            })
        {
            continue;
        }
        let mut membership = Vec::new();
        for group in groups {
            if entry
                .groups
                .as_ref()
                .is_some_and(|ids| ids.contains(&group.group))
            {
                membership.push(Some(group.group));
            }
        }
        if membership.is_empty() {
            membership.push(None);
        }
        desired.extend(membership.into_iter().map(|group| (entry, group)));
    }
    desired.sort_by_key(|(entry, _)| {
        contact_sort_key(
            entry,
            owner.settings.preferences.vip_sort,
            &presence.presence.entries,
        )
    });
    let mut entities = std::collections::HashMap::new();
    let mut retained = std::collections::HashSet::new();
    for (entity, row) in &rows {
        if rebuild
            || row.generation != generation
            || !desired
                .iter()
                .any(|(entry, group)| entry.entry == row.entry && *group == row.group)
        {
            if !rebuild {
                commands.entity(entity).despawn();
            }
        } else {
            retained.insert((row.entry, row.group));
            entities.insert((row.entry, row.group), entity);
        }
    }
    {
        for &(entry, group) in desired
            .iter()
            .filter(|(entry, group)| !retained.contains(&(entry.entry, *group)))
        {
            let Some(&(list, _)) = sections.get(&group) else {
                continue;
            };
            let row = commands
                .spawn((
                    Row {
                        generation,
                        entry: entry.entry,
                        group,
                    },
                    bevy::ui_widgets::Button,
                    bevy::picking::hover::Hovered::default(),
                    Control {
                        generation,
                        revision: panel.revision,
                        command: Action::Contact(entry.entry),
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
                .observe(input::contact_click)
                .id();
            if let Some(icon) = entry.icon {
                let icon = commands
                    .spawn((
                        Name::new(format!("vip.icon.{icon}")),
                        source::text(icon_glyph(icon), source::font(skins.font.clone(), false)),
                    ))
                    .id();
                commands.entity(row).add_child(icon);
            }
            if let Some(description) = entry.description.as_deref().filter(|text| !text.is_empty())
            {
                let tooltip = commands
                    .spawn((
                        Description(row),
                        source::text(description, source::font(skins.font.clone(), false)),
                        Node {
                            display: Display::None,
                            position_type: PositionType::Absolute,
                            top: px(18.),
                            ..default()
                        },
                        GlobalZIndex(200),
                        BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                    ))
                    .id();
                commands.entity(row).add_child(tooltip);
            }
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
            entities.insert((entry.entry, group), row);
        }
    }
    for (group, &(section, heading)) in sections.iter() {
        let mut children = vec![heading];
        children.extend(
            desired
                .iter()
                .filter(|(_, membership)| membership == group)
                .filter_map(|(entry, group)| entities.get(&(entry.entry, *group)).copied()),
        );
        commands.entity(section).replace_children(&children);
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
    for (entity, mut control, checked) in &mut controls {
        let selected = match control.command {
            Action::Sort(sort) => sort == owner.settings.preferences.vip_sort,
            Action::HideOffline => owner.settings.preferences.vip_hide_offline,
            _ => false,
        };
        if selected && !checked {
            commands.entity(entity).insert(bevy::ui::Checked);
        }
        if !selected && checked {
            commands.entity(entity).remove::<bevy::ui::Checked>();
        }
        // Retiring rows are already queued for despawn; never queue more component writes.
        if matches!(control.command,Action::Edit(entry) | Action::Contact(entry) if control.generation!=generation || !entries.iter().any(|row| row.entry==entry))
        {
            continue;
        }
        if matches!(
            control.command,
            Action::Add
                | Action::Refresh
                | Action::Next(_)
                | Action::Close
                | Action::Sort(_)
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
                | Action::Sort(_)
                | Action::HideOffline
                | Action::Edit(_)
                | Action::Contact(_)
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

#[derive(Component)]
pub(super) struct ContactMenu;
pub(super) fn contact_menu(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    state: Res<State>,
    lists: Query<Entity, With<List>>,
    menus: Query<Entity, With<ContactMenu>>,
    mut shown: Local<Option<(u64, u64, u64)>>,
) {
    let key = state.contact_menu.and_then(|entry| {
        owner
            .play
            .as_ref()
            .map(|play| (owner.generation(), play.link.vip_panel().revision, entry))
    });
    if *shown == key {
        return;
    }
    *shown = key;
    for entity in &menus {
        commands.entity(entity).despawn();
    }
    let Some(entry) = state.contact_menu else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.vip_panel();
    if !Snapshot::read(&panel)
        .is_ok_and(|snapshot| snapshot.entries.iter().any(|row| row.entry == entry))
    {
        return;
    }
    let Ok(list) = lists.single() else {
        return;
    };
    let menu = commands
        .spawn((
            ContactMenu,
            Node {
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    commands.entity(list).add_child(menu);
    for (label, command, name) in [
        ("Message", Action::MessageContact(entry), "vip.message"),
        ("Edit", Action::Edit(entry), "vip.edit"),
    ] {
        let action = button(
            &mut commands,
            &skins,
            Control {
                generation: owner.generation(),
                revision: panel.revision,
                command,
            },
            label,
            100.,
            name,
        );
        commands.entity(menu).add_child(action);
    }
}

fn icon_glyph(icon: u8) -> &'static str {
    const ICONS: [&str; 20] = [
        "•", "★", "♥", "◆", "♠", "♣", "♦", "●", "■", "▲", "✓", "!", "?", "+", "×", "☀", "☾", "⚑",
        "⚔", "♜",
    ];
    ICONS.get(usize::from(icon)).copied().unwrap_or("•")
}
#[derive(Component)]
pub(super) struct Description(Entity);
pub(super) fn descriptions(
    rows: Query<&bevy::picking::hover::Hovered, With<Row>>,
    mut descriptions: Query<(&Description, &mut Node)>,
) {
    for (description, mut node) in &mut descriptions {
        node.display = if rows
            .get(description.0)
            .is_ok_and(|interaction| interaction.0)
        {
            Display::Flex
        } else {
            Display::None
        };
    }
}

pub(super) fn contact_sort_key(
    entry: &Contact,
    sort: u8,
    presence: &[oteryn_session::VipPresenceEntry],
) -> (u8, String, u64) {
    let rank = match sort {
        1 => entry.icon.unwrap_or(0),
        2 => presence
            .iter()
            .find(|value| value.entry == entry.entry)
            .map_or(1, |value| match value.status {
                VipPresenceStatus::Online => 0,
                VipPresenceStatus::Offline => 2,
                _ => 1,
            }),
        _ => 0,
    };
    (rank, entry.name.to_lowercase(), entry.entry)
}
