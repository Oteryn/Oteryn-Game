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
    mut indicators: Query<(&mut Text, &mut Node), With<Indicator>>,
) {
    let settings = owner.settings.preferences.render_performance();
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
    for (mut text, mut node) in &mut indicators {
        let display = if label.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
        let next = label.as_deref().unwrap_or("");
        if text.0 != next {
            text.0 = next.into();
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
