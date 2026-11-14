//! Live battle rows retain both the admitted session and target generation.
use super::*;
use oteryn_session::{EntityDetail, WorldSpatialEntity};
#[derive(Default)]
pub(super) struct Snapshot {
    generation: Option<u64>,
    rows: Vec<WorldSpatialEntity>,
    active: usize,
    names: Vec<String>,
}
#[derive(Component)]
pub(super) struct HealthFill(EntityRef);
#[derive(Component)]
pub(super) struct TargetFrame(EntityRef);
#[derive(Component)]
pub(super) struct Feedback;
fn name(row: &WorldSpatialEntity) -> String {
    row.display_name
        .as_ref()
        .and_then(|name| name.as_str().ok())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{:?}", row.kind))
}
// Separate ECS queries update retained controls without rebuilding them on health packets.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    regions: Res<super::super::GameRegions>,
    skins: Res<Skins>,
    mut shown: Local<Snapshot>,
    actions: Query<(Entity, &HudAction, Has<InteractionDisabled>)>,
    mut health: Query<(&HealthFill, &mut Node, &mut BackgroundColor)>,
    mut feedback: Query<&mut Text, With<Feedback>>,
    mut frames: Query<(&TargetFrame, &mut BorderColor)>,
) {
    let Some(play) = &owner.play else {
        if shown.generation.take().is_some() {
            commands.entity(regions.battle).despawn_children();
        }
        return;
    };
    let generation = owner.generation();
    let panel = play.link.battle_panel();
    let active = play.link.active_battle_list();
    let rows = play.link.filtered_battle_rows(active);
    let modal = owner.modal_active() || play.view.gameplay_input_blocked(&play.link);
    let names = owner
        .settings
        .preferences
        .battle_lists
        .iter()
        .map(|list| list.name.clone())
        .collect::<Vec<_>>();
    let disabled = panel.pending || panel.combat.is_none() || panel.shutdown || modal;
    for (entity, action, was_disabled) in &actions {
        if matches!(
            action.command,
            Command::Attack(_) | Command::Stop | Command::Fight(_) | Command::Chase(_)
        ) {
            if disabled && !was_disabled {
                commands.entity(entity).insert(InteractionDisabled);
            }
            if !disabled && was_disabled {
                commands.entity(entity).remove::<InteractionDisabled>();
            }
        }
    }
    let by_entity = rows
        .iter()
        .map(|row| (row.entity, row))
        .collect::<std::collections::BTreeMap<_, _>>();
    for (fill, mut node, mut color) in &mut health {
        if let Some(row) = by_entity.get(&fill.0) {
            let percent_health = match row.detail {
                EntityDetail::Actor { health_percent, .. } => health_percent,
                _ => 0,
            };
            let width = percent(f32::from(percent_health.min(100)));
            if node.width != width {
                node.width = width;
            }
            let next = Color::srgb_u8(0, 128, 0);
            if color.0 != next {
                color.0 = next;
            }
        }
    }
    for (frame, mut border) in &mut frames {
        let selected = panel
            .combat
            .is_some_and(|combat| combat.target == Some(frame.0));
        let next = BorderColor::all(if selected {
            Color::srgb_u8(220, 40, 40)
        } else {
            Color::NONE
        });
        if *border != next {
            *border = next;
        }
    }
    for mut text in &mut feedback {
        let next = panel.feedback.unwrap_or("");
        if text.0 != next {
            text.0 = next.into();
        }
    }
    let same_rows = shown.rows.len() == rows.len()
        && shown.rows.iter().zip(&rows).all(|(old, new)| {
            old.entity == new.entity && old.display_name == new.display_name && old.kind == new.kind
        });
    if shown.generation == Some(generation)
        && same_rows
        && shown.active == active
        && shown.names == names
    {
        return;
    }
    commands.entity(regions.battle).despawn_children();
    let title = commands
        .spawn(source::text(
            "Battle",
            source::font(skins.font.clone(), false),
        ))
        .id();
    commands.entity(regions.battle).add_child(title);
    let config = super::super::battle_config::entry(&mut commands, &skins, generation);
    commands.entity(regions.battle).add_child(config);
    for (index, name) in names.iter().enumerate() {
        let label = if index == active {
            format!("• {name}")
        } else {
            name.clone()
        };
        let selector = button(
            &mut commands,
            &skins,
            generation,
            Command::BattleList(index),
            &label,
            174.,
        );
        commands.entity(regions.battle).add_child(selector);
    }
    let modes = commands
        .spawn(Node {
            width: percent(100.),
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(2.),
            row_gap: px(2.),
            ..default()
        })
        .id();
    for (command, label) in [
        (Command::Stop, "Stop"),
        (Command::Fight(FightMode::Offensive), "Attack"),
        (Command::Fight(FightMode::Balanced), "Balance"),
        (Command::Fight(FightMode::Defensive), "Defend"),
        (Command::Chase(ChaseMode::Stand), "Stand"),
        (Command::Chase(ChaseMode::Chase), "Chase"),
    ] {
        let button = button(&mut commands, &skins, generation, command, label, 56.);
        if disabled {
            commands.entity(button).insert(InteractionDisabled);
        }
        commands.entity(modes).add_child(button);
    }
    commands.entity(regions.battle).add_child(modes);
    // BattleListEntry.qml: 22px rows, 20px outfit slot, 2px gap, 5px slim bar.
    for row in &rows {
        let entry = commands
            .spawn((
                bevy::ui_widgets::Button,
                bevy::input_focus::tab_navigation::TabIndex(0),
                HudAction {
                    generation,
                    command: Command::Attack(row.entity),
                },
                Node {
                    width: percent(100.),
                    height: px(22.),
                    flex_shrink: 0.,
                    overflow: Overflow::clip(),
                    ..default()
                },
            ))
            .id();
        if disabled {
            commands.entity(entry).insert(InteractionDisabled);
        }
        let target = panel
            .combat
            .is_some_and(|combat| combat.target == Some(row.entity));
        let icon = commands
            .spawn((
                TargetFrame(row.entity),
                Node {
                    width: px(20.),
                    height: px(20.),
                    border: UiRect::all(px(1.)),
                    ..default()
                },
                BorderColor::all(if target {
                    Color::srgb_u8(220, 40, 40)
                } else {
                    Color::NONE
                }),
                Pickable::IGNORE,
            ))
            .id();
        let caption = commands
            .spawn((
                source::text(name(row), source::font(skins.font.clone(), false)),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(22.),
                    right: px(0.),
                    top: px(0.),
                    ..default()
                },
            ))
            .id();
        let track = commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(22.),
                    right: px(0.),
                    height: px(5.),
                    bottom: px(2.),
                    border: UiRect::all(px(1.)),
                    ..default()
                },
                BorderColor::all(Color::BLACK),
                Pickable::IGNORE,
            ))
            .id();
        let health = match row.detail {
            EntityDetail::Actor { health_percent, .. } => health_percent,
            _ => 0,
        };
        let fill = commands
            .spawn((
                HealthFill(row.entity),
                Node {
                    width: percent(f32::from(health.min(100))),
                    height: percent(100.),
                    ..default()
                },
                BackgroundColor(Color::srgb_u8(0, 128, 0)),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(track).add_child(fill);
        commands.entity(entry).add_children(&[icon, caption, track]);
        commands.entity(regions.battle).add_child(entry);
    }
    let label = commands
        .spawn((
            Feedback,
            source::text(
                panel.feedback.unwrap_or(""),
                source::font(skins.font.clone(), false),
            ),
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(regions.battle).add_child(label);
    shown.generation = Some(generation);
    shown.rows = rows;
    shown.active = active;
    shown.names = names;
}
