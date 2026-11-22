//! Frame-rate limits and the FPS/lag indicator from the retained performance preferences.
use super::{game_ui::WorldRegion, owner::HostOwner, ui::Skins};
use crate::play::PerformancePacer;
use bevy::{prelude::*, window::PrimaryWindow};
use oteryn_client_ui_bevy as source;
use std::time::Instant;

#[derive(Component)]
pub(super) struct Indicator;

pub(super) fn install(app: &mut App) {
    app.init_non_send::<PerformancePacer>()
        .add_systems(Startup, setup.after(super::ui::setup))
        .add_systems(Update, anchor)
        .add_systems(Last, pace);
}

fn setup(mut commands: Commands, skins: Res<Skins>) {
    commands.spawn((
        Indicator,
        bevy::ui::RelativeCursorPosition::default(),
        Name::new("performance.indicator"),
        source::text("", source::font(skins.font.clone(), true)),
        Node {
            position_type: PositionType::Absolute,
            left: px(4.),
            top: px(4.),
            display: Display::None,
            ..default()
        },
        ZIndex(30),
    ));
}

/// Keeps the indicator in the top-left corner of the map viewport, as the reference client
/// draws it, rather than the corner of the OS window.
fn anchor(
    mut commands: Commands,
    indicators: Query<(Entity, Option<&ChildOf>), With<Indicator>>,
    regions: Query<Entity, With<WorldRegion>>,
) {
    let Some(region) = regions.iter().next() else {
        return;
    };
    for (indicator, parent) in &indicators {
        if parent.map(ChildOf::parent) != Some(region) {
            commands.entity(region).add_child(indicator);
        }
    }
}

/// Waits out the rest of the current frame interval, then counts the frame and projects the
/// indicator text. A zero foreground limit never waits.
pub(super) fn pace(
    owner: NonSend<HostOwner>,
    mut pacer: NonSendMut<PerformancePacer>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut previous: Local<Option<(u64, bool)>>,
    mut indicators: Query<
        (
            &mut Text,
            &mut Node,
            Option<&bevy::ui::RelativeCursorPosition>,
        ),
        With<Indicator>,
    >,
) {
    let settings = owner.settings.preferences.render_performance();
    let session = (owner.generation(), owner.play.is_some());
    if previous.is_some_and(|value| value != session) || owner.play.is_none() {
        pacer.reset_range();
    }
    *previous = Some(session);
    pacer.focus(windows.iter().next().is_none_or(|window| window.focused));
    let now = Instant::now();
    let deadline = pacer.deadline(now, settings);
    if deadline > now {
        std::thread::sleep(deadline - now);
    }
    let now = Instant::now();
    pacer.begin_redraw(now, settings);
    pacer.presented(now);
    let latency = owner
        .play
        .as_ref()
        .and_then(|play| play.link.performance_command_latency());
    let label = pacer.indicator(now, settings, latency);
    for (mut text, mut node, cursor) in &mut indicators {
        let display = if label.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        let mut next = label.clone().unwrap_or_default();
        if owner.play.is_some()
            && settings.show_fps
            && owner.settings.preferences.tooltips_enabled
            && cursor.is_some_and(|cursor| cursor.cursor_over())
            && let Some((min, max)) = pacer.fps_range()
        {
            next.push_str(&format!(
                "\nMin: {min}\nMid: {}\nMax: {max}",
                min.midpoint(max)
            ));
        }
        if text.0 != next {
            text.0 = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_spawns_one_hidden_unpickable_indicator_with_the_skin_font() {
        let mut app = App::new();
        app.insert_resource(Skins {
            buttons: source::ButtonImages {
                up: default(),
                down: default(),
            },
            checked: default(),
            unchecked: default(),
            frame: default(),
            inset: default(),
            font: default(),
        });
        app.add_systems(Startup, setup);
        app.update();
        let mut indicators = app
            .world_mut()
            .query_filtered::<(&Node, &Pickable), With<Indicator>>();
        let found: Vec<_> = indicators
            .iter(app.world())
            .map(|(node, pickable)| (node.display, *pickable == Pickable::IGNORE))
            .collect();
        assert_eq!(found, vec![(Display::None, true)]);
    }

    #[test]
    fn the_indicator_sits_in_the_map_viewport_corner() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new();
        app.insert_resource(Skins {
            buttons: source::ButtonImages {
                up: default(),
                down: default(),
            },
            checked: default(),
            unchecked: default(),
            frame: default(),
            inset: default(),
            font: default(),
        });
        app.add_systems(Startup, setup).add_systems(Update, anchor);
        let region = app.world_mut().spawn((WorldRegion, Node::default())).id();
        app.update();
        app.update();
        let mut indicators = app
            .world_mut()
            .query_filtered::<(&ChildOf, &Node), With<Indicator>>();
        let (parent, node) = indicators.single(app.world())?;
        assert_eq!(parent.parent(), region);
        assert_eq!(node.position_type, PositionType::Absolute);
        assert_eq!((node.left, node.top), (px(4.), px(4.)));
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn pace_displays_lag_class_at_reference_thresholds() -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PlayView, play_channel};
    use std::time::Duration;
    let (link, _, outbox) = play_channel();
    let mut host = HostOwner::new(None)?;
    host.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    host.settings.preferences.frames.show_latency = true;
    let mut app = App::new();
    app.insert_non_send(host)
        .init_non_send::<PerformancePacer>()
        .add_systems(Update, pace);
    let indicator = app
        .world_mut()
        .spawn((Indicator, Text::default(), Node::default()))
        .id();
    for (ms, class) in [(249, "Low lag"), (250, "Medium lag"), (500, "High lag")] {
        outbox.performance_command_result(Duration::from_millis(ms));
        app.update();
        assert!(
            app.world()
                .get::<Text>(indicator)
                .ok_or("indicator")?
                .0
                .contains(class)
        );
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn hovering_online_indicator_shows_measured_extrema_and_midpoint()
-> Result<(), Box<dyn std::error::Error>> {
    use std::time::Duration;
    let mut host = HostOwner::new(None)?;
    host.settings.preferences.frames.show_fps = true;
    let (link, _, _) = crate::play::play_channel();
    host.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    let mut pacer = PerformancePacer::default();
    let start = Instant::now() - Duration::from_secs(2);
    for ms in [0, 500, 1000, 1250, 1500, 1750, 2000] {
        pacer.presented(start + Duration::from_millis(ms));
    }
    let mut app = App::new();
    app.insert_non_send(host)
        .insert_non_send(pacer)
        .add_systems(Last, pace);
    let indicator = app
        .world_mut()
        .spawn((
            Indicator,
            Text::default(),
            Node::default(),
            bevy::ui::RelativeCursorPosition {
                cursor_over: true,
                ..default()
            },
        ))
        .id();
    app.update();
    let text = &app.world().get::<Text>(indicator).ok_or("indicator")?.0;
    for value in ["Min: 2", "Mid: 3", "Max: 4"] {
        assert!(text.contains(value), "{text}");
    }
    app.world_mut()
        .get_mut::<bevy::ui::RelativeCursorPosition>(indicator)
        .ok_or("cursor")?
        .cursor_over = false;
    app.update();
    assert!(
        !app.world()
            .get::<Text>(indicator)
            .ok_or("indicator")?
            .0
            .contains("Min:")
    );
    Ok(())
}
