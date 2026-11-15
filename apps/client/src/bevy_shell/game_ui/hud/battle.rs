//! Live battle rows retain both the admitted session and target generation.
use super::*;
use oteryn_session::{EntityDetail, WorldSpatialEntity};
#[derive(Default)]
pub(super) struct Snapshot {
    generation: Option<u64>,
    rows: Vec<WorldSpatialEntity>,
    active: usize,
    names: Vec<String>,
    pvp: String,
}
#[derive(Component)]
pub(super) struct HealthFill(EntityRef);
#[derive(Component)]
pub(super) struct TargetFrame(EntityRef);
#[derive(Component)]
pub(super) struct Feedback;
fn keyboard(
    mut event: On<bevy::input_focus::FocusedInput<bevy::input::keyboard::KeyboardInput>>,
    disabled: Query<Has<InteractionDisabled>>,
    mut commands: Commands,
) {
    if disabled
        .get(event.focused_entity)
        .is_ok_and(|disabled| !disabled)
        && !event.input.repeat
        && event.input.state == bevy::input::ButtonState::Pressed
        && matches!(event.input.key_code, KeyCode::Enter | KeyCode::Space)
    {
        event.propagate(false);
        commands.trigger(Activate {
            entity: event.focused_entity,
        });
    }
}
#[derive(Component, Default)]
struct Chord {
    held: [bool; 2],
    chord: bool,
    suppress: bool,
}
fn press(
    event: On<bevy::picking::events::PointerPress>,
    mut rows: Query<&mut Chord>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
) {
    use bevy::picking::pointer::PointerButton;
    let index = match event.button {
        PointerButton::Primary => 0,
        PointerButton::Secondary => 1,
        _ => return,
    };
    if let Ok(mut state) = rows.get_mut(event.entity) {
        if mouse.as_ref().is_some_and(|mouse| {
            let other = if index == 0 {
                MouseButton::Right
            } else {
                MouseButton::Left
            };
            !mouse.pressed(other) && !mouse.just_pressed(other)
        }) {
            *state = Chord::default();
        }
        state.held[index] = true;
        state.chord |= state.held.iter().all(|held| *held);
    }
}
fn click(
    event: On<PointerClick>,
    guard: ModalGuard,
    actions: Query<(&HudAction, Has<InteractionDisabled>)>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut chords: Query<&mut Chord>,
    mut owner: NonSendMut<HostOwner>,
    mut commands: Commands,
) {
    use bevy::picking::pointer::PointerButton;
    let Ok((action, disabled)) = actions.get(event.entity) else {
        return;
    };
    if disabled
        || !guard.allows(event.entity)
        || owner.modal_active()
        || action.generation != owner.generation()
    {
        return;
    }
    let Command::Attack(target) = action.command else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let preferences = &owner.settings.preferences;
    let swapped = (preferences.controls.mouse == crate::play::MousePreset::LeftHanded)
        ^ preferences.swap_mouse_buttons;
    let primary = if swapped {
        PointerButton::Secondary
    } else {
        PointerButton::Primary
    };
    let shift = keys
        .as_ref()
        .is_some_and(|keys| keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]));
    let index = match event.button {
        PointerButton::Primary => 0,
        PointerButton::Secondary => 1,
        _ => return,
    };
    let mut chord = false;
    if let Ok(mut state) = chords.get_mut(event.entity) {
        state.held[index] = false;
        if state.suppress {
            *state = Chord::default();
            return;
        }
        chord |= state.chord;
        if chord {
            state.suppress = true;
        }
    }
    if event.button == primary && !shift && !chord {
        commands.trigger(Activate {
            entity: event.entity,
        });
        return;
    }
    if !matches!(
        event.button,
        PointerButton::Primary | PointerButton::Secondary
    ) {
        return;
    }
    let Some(menu) = play.view.creature_context(target, &play.link) else {
        return;
    };
    if event.button == primary || chord {
        play.view.world_context_choose(
            &play.link,
            &menu,
            oteryn_client_ui::world_interaction::menu::Action::Look,
        );
    } else {
        owner.request_creature_context(target);
    }
}
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
    let pvp = play.link.pvp_expert_panel();
    let pvp_state = format!(
        "{:?}:{}:{}:{}",
        pvp.confirmed, pvp.pending, pvp.feedback, pvp.selected
    );
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
    let disabled =
        panel.pending || pvp.pending || panel.combat.is_none() || panel.shutdown || modal;
    for (entity, action, was_disabled) in &actions {
        if matches!(
            action.command,
            Command::Attack(_)
                | Command::Stop
                | Command::Fight(_)
                | Command::Chase(_)
                | Command::Secure(_)
                | Command::Expert(_)
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
            let next =
                crate::bevy_shell::health_circle::health_colour(u32::from(percent_health), 100);
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
        && shown.pvp == pvp_state
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
    if let Some(confirmed) = pvp.confirmed.filter(|_| pvp.selected) {
        for (command, label, selected) in pvp_controls(confirmed) {
            let label = if selected {
                format!("• {label}")
            } else {
                label.into()
            };
            let control = button(&mut commands, &skins, generation, command, &label, 174.);
            if disabled || pvp.pending {
                commands.entity(control).insert(InteractionDisabled);
            }
            commands.entity(regions.battle).add_child(control);
        }
        let status = commands
            .spawn(source::text(
                &pvp.feedback,
                source::font(skins.font.clone(), false),
            ))
            .id();
        commands.entity(regions.battle).add_child(status);
    }
    // BattleListEntry.qml: 22px rows, 20px outfit slot, 2px gap, 5px slim bar.
    for row in &rows {
        let entry = commands
            .spawn((
                Pickable::default(),
                Chord::default(),
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
            .observe(click)
            .observe(press)
            .observe(keyboard)
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
                BackgroundColor(crate::bevy_shell::health_circle::health_colour(
                    u32::from(health),
                    100,
                )),
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
    shown.pvp = pvp_state;
}

fn pvp_controls(
    state: oteryn_client_runtime::pvp_expert::ExpertFightModes,
) -> [(Command, &'static str, bool); 5] {
    use oteryn_client_runtime::pvp_expert::ExpertMode;
    [
        (
            Command::Secure(!state.modes.secure),
            "Secure attack",
            state.modes.secure,
        ),
        (Command::Expert(0), "Dove", state.expert == ExpertMode::Dove),
        (
            Command::Expert(1),
            "White Hand",
            state.expert == ExpertMode::WhiteHand,
        ),
        (
            Command::Expert(2),
            "Yellow Hand",
            state.expert == ExpertMode::YellowHand,
        ),
        (
            Command::Expert(3),
            "Red Fist",
            state.expert == ExpertMode::RedFist,
        ),
    ]
}

pub(super) fn request_pvp(link: &crate::play::PlayLink, secure: Option<bool>, expert: Option<u8>) {
    let panel = link.pvp_expert_panel();
    let Some(state) = panel.confirmed.filter(|_| panel.selected && !panel.pending) else {
        return;
    };
    for (field, value) in [
        (0, (state.modes.fight_mode as u32).to_string()),
        (1, (state.modes.chase as u32).to_string()),
        (
            2,
            u8::from(secure.unwrap_or(state.modes.secure)).to_string(),
        ),
        (
            3,
            expert
                .map(u32::from)
                .unwrap_or(state.expert as u32)
                .to_string(),
        ),
    ] {
        link.pvp_expert_edit(field, value);
    }
    link.pvp_expert_request(oteryn_client_ui::pvp_expert::Intent::Apply);
}

#[cfg(test)]
mod combat_tests {
    use super::*;
    #[test]
    fn combat_hud_rejects_pvp_changes_without_negotiated_state() {
        let (link, mut incoming, _outbox) = crate::play::play_channel();
        request_pvp(&link, Some(false), None);
        request_pvp(&link, None, Some(3));
        assert!(incoming.try_recv().is_err());
        assert!(!link.pvp_expert_panel().pending);
    }
    #[test]
    fn secure_toggle_and_all_expert_modes_follow_confirmed_state() {
        use oteryn_client_runtime::pvp_expert::{ExpertFightModes, ExpertMode, attack::FightModes};
        for expert in [
            ExpertMode::Dove,
            ExpertMode::WhiteHand,
            ExpertMode::YellowHand,
            ExpertMode::RedFist,
        ] {
            for secure in [false, true] {
                let controls = pvp_controls(ExpertFightModes {
                    modes: FightModes {
                        fight_mode: FightMode::Balanced,
                        chase: ChaseMode::Stand,
                        secure,
                    },
                    expert,
                });
                assert!(matches!(controls[0].0, Command::Secure(value) if value != secure));
                assert_eq!(controls[0].2, secure);
                assert_eq!(controls[1..].iter().filter(|control| control.2).count(), 1);
                assert!(
                    matches!(controls[expert as usize + 1].0, Command::Expert(value) if value == expert as u8)
                );
            }
        }
    }
}
