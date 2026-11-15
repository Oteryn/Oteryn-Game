//! Unjustified points: server-observed counters and skull risk, without inferred kill limits.
use super::super::{
    owner::{HostOwner, Phase},
    ui::Skins,
};
use super::modal::ModalGuard;
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::party_skull::{Intent, Panel};
use oteryn_client_ui_bevy as source;
#[cfg(test)]
mod tests;
#[derive(Resource, Default)]
struct State {
    generation: u64,
    panel: Option<Panel>,
    root: Option<Entity>,
    content: Option<Entity>,
    refresh: Option<Entity>,
}
#[derive(Component)]
struct Control {
    generation: u64,
    close: bool,
}
#[derive(Component)]
struct RiskFill;
#[derive(Component)]
struct TimerLabel;
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>().add_systems(Update, project);
}
pub(in crate::bevy_shell) fn open(owner: &HostOwner) {
    if owner.modal_active() {
        return;
    }
    if let Some(play) = &owner.play {
        play.link.party_skull_open();
        play.link.party_skull_request(Intent::PvpStatus);
    }
}
fn timer_text(panel: &Panel, elapsed_ms: u64) -> String {
    let Some(timer) = panel.skull_timer else {
        return "Skull duration unavailable".into();
    };
    // Keep the authoritative skull until a new observation; reaching zero is not a removal event.
    let name = match timer.skull {
        0 => "No",
        1 => "Yellow",
        2 => "Green",
        3 => "White",
        4 => "Red",
        5 => "Black",
        6 => "Orange",
        _ => "Unknown",
    };
    match timer.remaining_seconds(elapsed_ms) {
        Some(seconds) => format!("{name} skull: {seconds}s remaining"),
        None => format!("{name} skull: expiry unavailable"),
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut timers: Query<&mut Text, With<TimerLabel>>,
) {
    let panel = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing)
        .map(|play| play.link.party_skull_panel())
        .filter(|p| p.open);
    let timer = panel
        .as_ref()
        .map(|p| {
            timer_text(
                p,
                p.status_received_at.map_or(0, |at| {
                    u64::try_from(at.elapsed().as_millis()).unwrap_or(u64::MAX)
                }),
            )
        })
        .unwrap_or_default();
    for mut text in &mut timers {
        if text.0 != timer {
            text.0.clone_from(&timer);
        }
    }
    if state.generation == owner.generation() && state.panel == panel {
        return;
    }
    if (state.generation != owner.generation() || panel.is_none())
        && let Some(root) = state.root.take()
    {
        commands.entity(root).despawn();
        state.content = None;
        state.refresh = None;
    }
    state.generation = owner.generation();
    state.panel = panel;
    let Some(panel) = state.panel.clone() else {
        return;
    };
    let root = if let Some(root) = state.root {
        root
    } else {
        let root = commands
            .spawn((
                Name::new("unjustified_points.window"),
                GlobalZIndex(90),
                bevy::ui_widgets::ScrollArea,
                bevy::input_focus::tab_navigation::TabGroup::new(5),
                source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
                Node {
                    position_type: PositionType::Absolute,
                    right: px(8.),
                    top: px(70.),
                    width: px(300.),
                    max_width: percent(95.),
                    max_height: percent(85.),
                    overflow: Overflow::scroll_y(),
                    padding: UiRect {
                        top: px(22.),
                        bottom: px(6.),
                        left: px(6.),
                        right: px(6.),
                    },
                    flex_direction: FlexDirection::Column,
                    row_gap: px(4.),
                    ..default()
                },
            ))
            .id();
        source::dialog_caption(
            &mut commands,
            root,
            "Unjustified Points",
            skins.font.clone(),
        );
        for (caption, close) in [("Refresh", false), ("Close", true)] {
            let button = commands
                .spawn((
                    source::button(skins.buttons.clone(), 90.),
                    Control {
                        generation: owner.generation(),
                        close,
                    },
                ))
                .observe(activate)
                .id();
            if !close {
                state.refresh = Some(button);
            }
            let text = commands
                .spawn(source::button_caption(caption, skins.font.clone()))
                .id();
            commands.entity(button).add_child(text);
            commands.entity(root).add_child(button);
        }
        state.root = Some(root);
        root
    };
    if let Some(refresh) = state.refresh {
        if !panel.selected || panel.pending {
            commands
                .entity(refresh)
                .insert(bevy::ui::InteractionDisabled);
        } else {
            commands
                .entity(refresh)
                .remove::<bevy::ui::InteractionDisabled>();
        }
    }
    if let Some(content) = state.content.take() {
        commands.entity(content).despawn();
    }
    let content = commands
        .spawn(Node {
            width: percent(100.),
            flex_direction: FlexDirection::Column,
            row_gap: px(4.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(root).add_child(content);
    state.content = Some(content);
    let root = content;
    label(
        &mut commands,
        &skins,
        root,
        if panel.pending {
            "Waiting for the server"
        } else {
            &panel.feedback
        },
    );
    if !panel.selected {
        label(
            &mut commands,
            &skins,
            root,
            "Unjustified points unavailable in this session",
        );
    } else if let Some(snapshot) = &panel.unjustified {
        label(&mut commands, &skins, root, &snapshot.text());
        if let Some(situations) = &snapshot.open_situations {
            label(
                &mut commands,
                &skins,
                root,
                &format!("Open PvP situations: {} (at observation)", situations.len()),
            );
        }
        let timer = label(&mut commands, &skins, root, &timer);
        commands.entity(timer).insert(TimerLabel);
        for (index, window) in snapshot.windows.iter().enumerate() {
            for (name, limit, color) in [
                ("Red", Some(window.limit_milli), Color::srgb_u8(200, 55, 55)),
                (
                    "Black",
                    panel.black_limits_milli.map(|limits| limits[index]),
                    Color::srgb_u8(100, 100, 100),
                ),
            ] {
                let period = ["Daily", "Weekly", "Monthly"][index];
                let Some(limit) = limit.filter(|limit| *limit > 0) else {
                    label(
                        &mut commands,
                        &skins,
                        root,
                        &format!("{period} {name} threshold unavailable"),
                    );
                    continue;
                };
                let remaining = limit.saturating_sub(window.points_milli);
                label(
                    &mut commands,
                    &skins,
                    root,
                    &format!(
                        "{period} {name}: {}.{:03} points left / {}.{:03}",
                        remaining / 1000,
                        remaining % 1000,
                        limit / 1000,
                        limit % 1000
                    ),
                );
                let track = commands
                    .spawn((
                        Node {
                            width: percent(100.),
                            height: px(6.),
                            flex_shrink: 0.,
                            ..default()
                        },
                        BackgroundColor(Color::BLACK),
                    ))
                    .id();
                let ratio = window.points_milli.min(limit) as f64 / limit as f64;
                let fill = commands
                    .spawn((
                        RiskFill,
                        Node {
                            width: percent((ratio * 100.) as f32),
                            height: percent(100.),
                            ..default()
                        },
                        BackgroundColor(color),
                    ))
                    .id();
                commands.entity(track).add_child(fill);
                commands.entity(root).add_child(track);
            }
        }
    } else {
        label(
            &mut commands,
            &skins,
            root,
            "No PvP observation received. Refresh to request status.",
        );
    }
}
fn label(commands: &mut Commands, skins: &Skins, root: Entity, text: &str) -> Entity {
    let child = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(root).add_child(child);
    child
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity) || owner.phase != Phase::Playing || owner.modal_active() {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if control.generation != owner.generation() {
        return;
    }
    if let Some(play) = &owner.play {
        if !play.link.party_skull_panel().open {
            return;
        }
        if control.close {
            play.link.party_skull_close();
        } else if !play.view.gameplay_input_blocked(&play.link) {
            play.link.party_skull_request(Intent::PvpStatus);
        }
    }
}
