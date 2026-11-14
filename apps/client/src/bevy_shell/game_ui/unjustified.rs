//! Unjustified points sidebar over the existing capability-134 PvP observation.
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    modal::ModalGuard,
};
use bevy::{prelude::*, ui::InteractionDisabled, ui_widgets::Activate};
use oteryn_client_ui::party_skull::{Intent, Panel};
use oteryn_client_ui_bevy as source;

#[derive(Component)]
pub(super) struct UnjustifiedDock;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Fill {
    Day,
    Week,
    Month,
}
#[derive(Component)]
struct SkullIcon;
#[derive(Component)]
struct Refresh(u64);
#[derive(Resource, Default)]
struct State {
    generation: Option<u64>,
    requested: bool,
    snapshot: Option<(Panel, Option<u64>)>,
    root: Option<Entity>,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>().add_systems(Update, project);
}

fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    assets: Res<AssetServer>,
    docks: Query<Entity, With<UnjustifiedDock>>,
    mut state: ResMut<State>,
) {
    let Ok(dock) = docks.single() else {
        return;
    };
    let generation = owner.play.as_ref().map(|_| owner.generation());
    if state.generation != generation {
        state.generation = generation;
        state.requested = false;
        state.snapshot = None;
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
    }
    let panel = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing)
        .map(|play| {
            let mut panel = play.link.party_skull_panel();
            if panel.selected && !panel.pending && !state.requested {
                play.link.party_skull_request(Intent::PvpStatus);
                state.requested = true;
                panel = play.link.party_skull_panel();
            }
            panel
        })
        .filter(|panel| panel.selected);
    let elapsed_ms = panel
        .as_ref()
        .and_then(|panel| panel.status_received_at)
        .map_or(0, |at| {
            u64::try_from(at.elapsed().as_millis()).unwrap_or(u64::MAX)
        });
    let snapshot = panel.map(|panel| {
        let seconds = panel
            .skull_timer
            .and_then(|timer| timer.remaining_seconds(elapsed_ms));
        (panel, seconds)
    });
    if state.snapshot == snapshot {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    if let Some((panel, _)) = &snapshot {
        state.root = Some(build(
            &mut commands,
            &skins,
            &assets,
            dock,
            panel,
            elapsed_ms,
            owner.generation(),
        ));
    }
    state.snapshot = snapshot;
}

fn refresh(
    event: On<Activate>,
    controls: Query<&Refresh>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity) || owner.modal_active() || owner.phase != Phase::Playing {
        return;
    }
    if let Ok(control) = controls.get(event.entity)
        && control.0 == owner.generation()
        && let Some(play) = &owner.play
        && !play.view.gameplay_input_blocked(&play.link)
    {
        play.link.party_skull_request(Intent::PvpStatus);
    }
}

fn build(
    commands: &mut Commands,
    skins: &Skins,
    assets: &AssetServer,
    dock: Entity,
    panel: &Panel,
    elapsed_ms: u64,
    generation: u64,
) -> Entity {
    let root = commands
        .spawn((
            Name::new("unjustified.sidebar"),
            source::frame(skins.frame.clone(), [2., 17., 2., 2.]),
            Node {
                width: px(176.),
                flex_shrink: 0.,
                padding: UiRect {
                    top: px(20.),
                    bottom: px(4.),
                    left: px(4.),
                    right: px(4.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(3.),
                ..default()
            },
        ))
        .id();
    commands.entity(dock).add_child(root);
    source::dialog_caption(commands, root, "Unjustified Points", skins.font.clone());
    if let Some(snapshot) = &panel.unjustified {
        // OTClient's period-specific green/yellow/red bands; limits remain server-owned.
        for (index, (period, name, green, yellow, denominator)) in [
            (Fill::Day, "Day", 1u128, 2u128, 3u128),
            (Fill::Week, "Week", 2, 3, 5),
            (Fill::Month, "Month", 4, 8, 10),
        ]
        .into_iter()
        .enumerate()
        {
            let window = snapshot.windows[index];
            let limit = if panel
                .skull_timer
                .is_some_and(|timer| matches!(timer.skull, 4 | 5))
            {
                panel.black_limits_milli.map_or(0, |limits| limits[index])
            } else {
                window.limit_milli
            };
            let points = window.points_milli;
            let maximum = if limit == 0 {
                "unavailable".into()
            } else {
                format!("{}.{:03}", limit / 1000, limit % 1000)
            };
            label(
                commands,
                skins,
                root,
                format!("{name}: {}.{:03} / {maximum}", points / 1000, points % 1000),
            );
            let track = commands
                .spawn((
                    Node {
                        width: percent(100.),
                        height: px(8.),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(Color::srgb_u8(30, 30, 30)),
                ))
                .id();
            let color = if u128::from(points) * denominator <= u128::from(limit) * green {
                Color::srgb_u8(50, 180, 70)
            } else if u128::from(points) * denominator <= u128::from(limit) * yellow {
                Color::srgb_u8(220, 180, 40)
            } else {
                Color::srgb_u8(210, 50, 40)
            };
            let width = if limit == 0 {
                0.
            } else {
                (points.min(limit) as f64 / limit as f64 * 100.) as f32
            };
            let fill = commands
                .spawn((
                    period,
                    Node {
                        width: percent(width),
                        height: percent(100.),
                        display: if points == 0 || limit == 0 {
                            Display::None
                        } else {
                            Display::Flex
                        },
                        ..default()
                    },
                    BackgroundColor(color),
                ))
                .id();
            commands.entity(track).add_child(fill);
            commands.entity(root).add_child(track);
        }
    } else {
        label(commands, skins, root, "Unjustified points unavailable");
    }
    if let Some(timer) = panel.skull_timer {
        if let Some(path) = skull_path(timer.skull) {
            let icon = commands
                .spawn((
                    SkullIcon,
                    ImageNode::new(assets.load(path)),
                    Node {
                        width: px(13.),
                        height: px(13.),
                        ..default()
                    },
                ))
                .id();
            commands.entity(root).add_child(icon);
        } else {
            label(
                commands,
                skins,
                root,
                if timer.skull == 0 {
                    "No skull"
                } else {
                    "Skull tier unavailable"
                },
            );
        }
    }
    let time = panel
        .skull_timer
        .and_then(|timer| timer.remaining_seconds(elapsed_ms));
    label(
        commands,
        skins,
        root,
        time.map_or_else(
            || "Skull time unavailable".into(),
            |seconds| {
                format!(
                    "Skull time: {}d {}h {}m {}s",
                    seconds / 86400,
                    seconds / 3600 % 24,
                    seconds / 60 % 60,
                    seconds % 60
                )
            },
        ),
    );
    if !panel.feedback.is_empty() {
        label(commands, skins, root, &panel.feedback);
    }
    let button = commands
        .spawn((
            source::button(skins.buttons.clone(), 80.),
            Refresh(generation),
        ))
        .observe(refresh)
        .id();
    if panel.pending {
        commands.entity(button).insert(InteractionDisabled);
    }
    let caption = commands
        .spawn(source::button_caption("Refresh", skins.font.clone()))
        .id();
    commands.entity(button).add_child(caption);
    commands.entity(root).add_child(button);
    root
}

fn skull_path(skull: u8) -> Option<&'static str> {
    match skull {
        1 => Some("images/game/skulls/skull_yellow.png"),
        2 => Some("images/game/skulls/skull_green.png"),
        3 => Some("images/game/skulls/skull_white.png"),
        4 => Some("images/game/skulls/skull_red.png"),
        5 => Some("images/game/skulls/skull_black.png"),
        6 => Some("images/game/skulls/skull_orange.png"),
        _ => None,
    }
}

fn label(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let text = commands
        .spawn((
            source::text(value, source::font(skins.font.clone(), false)),
            Node {
                width: percent(100.),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(text);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bevy_shell::ui::Skins;
    use bevy::{asset::AssetPlugin, ecs::system::RunSystemOnce};
    use oteryn_client_ui::party_skull::{FragWindow, Panel, SkullTimer, UnjustifiedSnapshot};
    use oteryn_client_ui_bevy as source;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn render(panel: Panel, elapsed_ms: u64) -> Result<App, Box<dyn std::error::Error>> {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>();
        app.world_mut()
            .run_system_once(move |mut commands: Commands, assets: Res<AssetServer>| {
                let skins = Skins {
                    buttons: source::ButtonImages {
                        up: default(),
                        down: default(),
                    },
                    checked: default(),
                    unchecked: default(),
                    frame: default(),
                    inset: default(),
                    font: default(),
                };
                let dock = commands.spawn(Node::default()).id();
                build(&mut commands, &skins, &assets, dock, &panel, elapsed_ms, 1);
            })
            .map_err(|error| format!("{error:?}"))?;
        Ok(app)
    }

    fn panel(skull: u8) -> Panel {
        Panel {
            selected: true,
            unjustified: Some(UnjustifiedSnapshot {
                windows: [
                    FragWindow {
                        points_milli: 1500,
                        limit_milli: 3000,
                    },
                    FragWindow {
                        points_milli: 2000,
                        limit_milli: 5000,
                    },
                    FragWindow {
                        points_milli: 9000,
                        limit_milli: 10000,
                    },
                ],
                open_situations: Some(Vec::new()),
            }),
            skull_timer: Some(SkullTimer {
                skull,
                remaining_ms: Some(2500),
            }),
            black_limits_milli: Some([6000, 10000, 20000]),
            ..default()
        }
    }

    #[test]
    fn unjustified_bars() -> TestResult {
        let mut app = render(panel(0), 0)?;
        let mut query = app.world_mut().query::<(&Fill, &Node, &BackgroundColor)>();
        let bars: Vec<_> = query.iter(app.world()).collect();
        assert_eq!(bars.len(), 3);
        for (index, width, color) in [
            (Fill::Day, 50., Color::srgb_u8(220, 180, 40)),
            (Fill::Week, 40., Color::srgb_u8(50, 180, 70)),
            (Fill::Month, 90., Color::srgb_u8(210, 50, 40)),
        ] {
            let (_, node, actual) = bars
                .iter()
                .find(|(fill, _, _)| **fill == index)
                .ok_or("bar")?;
            assert_eq!(node.width, percent(width));
            assert_eq!(actual.0, color);
        }
        let mut edge = panel(0);
        if let Some(snapshot) = &mut edge.unjustified {
            snapshot.windows[0].points_milli = 0;
            snapshot.windows[1].points_milli = u64::MAX;
            snapshot.windows[2].limit_milli = 0;
        }
        let mut app = render(edge, 0)?;
        let mut query = app.world_mut().query::<(&Fill, &Node)>();
        for (fill, node) in query.iter(app.world()) {
            match fill {
                Fill::Day | Fill::Month => assert_eq!(node.display, Display::None),
                Fill::Week => assert_eq!(node.width, percent(100.)),
            }
        }
        let mut app = render(
            Panel {
                selected: true,
                ..default()
            },
            0,
        )?;
        let mut fills = app.world_mut().query::<&Fill>();
        assert_eq!(fills.iter(app.world()).count(), 0);
        let mut labels = app.world_mut().query::<&Text>();
        assert!(
            labels
                .iter(app.world())
                .any(|text| text.0.contains("unavailable"))
        );
        Ok(())
    }

    #[test]
    fn unjustified_skull_tier() -> TestResult {
        for (skull, expected_path, daily_width) in [
            (0, None, 50.),
            (3, Some("images/game/skulls/skull_white.png"), 50.),
            (4, Some("images/game/skulls/skull_red.png"), 25.),
            (5, Some("images/game/skulls/skull_black.png"), 25.),
        ] {
            let mut app = render(panel(skull), 1000)?;
            let mut icons = app
                .world_mut()
                .query_filtered::<&ImageNode, With<SkullIcon>>();
            let paths: Vec<_> = icons
                .iter(app.world())
                .map(|icon| {
                    icon.image
                        .path()
                        .map(|path| path.path().to_string_lossy().into_owned())
                })
                .collect();
            assert_eq!(
                paths,
                expected_path
                    .into_iter()
                    .map(|path| Some(path.to_owned()))
                    .collect::<Vec<_>>()
            );
            let mut fills = app.world_mut().query::<(&Fill, &Node)>();
            let (_, daily) = fills
                .iter(app.world())
                .find(|(fill, _)| **fill == Fill::Day)
                .ok_or("daily")?;
            assert_eq!(daily.width, percent(daily_width));
            let mut labels = app.world_mut().query::<&Text>();
            assert!(labels.iter(app.world()).any(|text| text.0.contains("2s")));
        }
        Ok(())
    }
}
