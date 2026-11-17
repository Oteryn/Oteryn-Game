//! Market catalogue filters and depot selection.
use super::*;

#[derive(Component)]
pub(super) struct DepotCaption;

pub(super) fn catalogue_filters(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    key: Key,
    panel: &MarketPanel,
) {
    let filters = scroll(commands, parent, px(220.), 8);
    let filter = &panel.details.filter;
    for (kind, name, caption, value) in [
        (
            FilterKind::Vocation,
            "vocation",
            "Vocation (1–6; blank = any)",
            filter.vocation.map(u16::from),
        ),
        (
            FilterKind::Minimum,
            "minimum",
            "Minimum level",
            filter.minimum_level,
        ),
        (
            FilterKind::Maximum,
            "maximum",
            "Maximum level",
            filter.maximum_level,
        ),
        (
            FilterKind::Forge,
            "forge",
            "Forge class (0–4; blank = any)",
            filter.forge_class.map(u16::from),
        ),
    ] {
        label(commands, skin, filters, caption);
        let mut editor = EditableText::new(value.map(|v| v.to_string()).unwrap_or_default());
        editor.max_characters = Some(5);
        let field = commands
            .spawn((
                FilterEditor { key, kind },
                Name::new(format!("market.filter.{name}")),
                TextInput,
                editor,
                source::font(skin.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextCursorStyle::default(),
                TabIndex(0),
                source::frame(skin.inset.clone(), [1.; 4]),
                Node {
                    width: percent(100.),
                    min_height: px(25.),
                    flex_shrink: 0.,
                    ..default()
                },
            ))
            .id();
        commands.entity(filters).add_child(field);
    }
    let depot = commands
        .spawn((
            source::checkbox(source::checkbox_row(false)),
            FilterDepot(key),
            Name::new("market.filter.depot"),
        ))
        .observe(change_depot)
        .id();
    if filter.depot_only {
        commands.entity(depot).insert(bevy::ui::Checked);
    }
    commands.entity(filters).add_child(depot);
    let caption = commands
        .spawn((
            source::button_caption(
                if filter.depot_only {
                    "[x] Depot only"
                } else {
                    "[ ] Depot only"
                },
                skin.font.clone(),
            ),
            DepotCaption,
        ))
        .id();
    commands.entity(depot).add_child(caption);
    let apply = commands
        .spawn((
            source::button(skin.buttons.clone(), 140.),
            FilterApply(key),
            Name::new("market.filter.apply"),
        ))
        .observe(apply_filter)
        .id();
    commands.entity(filters).add_child(apply);
    let caption = commands
        .spawn(source::button_caption("Apply filters", skin.font.clone()))
        .id();
    commands.entity(apply).add_child(caption);
}

pub(super) fn change_depot(
    event: On<bevy::ui_widgets::ValueChange<bool>>,
    controls: Query<&FilterDepot>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    guard: ModalGuard,
    mut commands: Commands,
    mut captions: Query<&mut Text, With<DepotCaption>>,
) {
    if guard.allows(event.source)
        && let Ok(control) = controls.get(event.source)
        && offers::current(&owner, &state, control.0).is_some()
    {
        if event.value {
            commands.entity(event.source).insert(bevy::ui::Checked);
        } else {
            commands.entity(event.source).remove::<bevy::ui::Checked>();
        }
        for mut text in &mut captions {
            text.0 = if event.value {
                "[x] Depot only"
            } else {
                "[ ] Depot only"
            }
            .into();
        }
    }
}

pub(super) fn apply_filter(
    event: On<Activate>,
    controls: Query<&FilterApply>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    guard: ModalGuard,
    mut pending: ResMut<PendingFilter>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
        && offers::current(&owner, &state, control.0).is_some()
    {
        pending.0 = Some((control.0, event.entity));
    }
}
