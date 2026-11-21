//! Session-local FIFO presentation for achievement banners.
use super::*;
use std::{collections::VecDeque, time::Duration};

// OTClient infobanner.lua: FADE_IN_MS, DEFAULT_HOLD_MS and FADE_OUT_MS.
const FADE_IN: Duration = Duration::from_millis(400);
const HOLD: Duration = Duration::from_millis(3000);
const FADE_OUT: Duration = Duration::from_millis(300);

#[derive(Component)]
struct Banner;
#[derive(Resource, Default)]
struct Queue {
    generation: Option<u64>,
    pending: VecDeque<String>,
    current: Option<(String, Duration)>,
}
impl Queue {
    fn next(&mut self, now: Duration) {
        self.current = self.pending.pop_front().map(|text| (text, now));
    }
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<Queue>()
        .add_systems(Startup, setup.after(super::super::setup))
        .add_systems(Update, (feed, project).chain());
}
fn setup(mut commands: Commands, skins: Res<Skins>) {
    spawn_banner(&mut commands, skins.font.clone());
}
fn spawn_banner(commands: &mut Commands, font: Handle<Font>) {
    commands
        .spawn((
            Banner,
            bevy::ui_widgets::Button,
            oteryn_client_ui_bevy::text("", oteryn_client_ui_bevy::font(font, false)),
            Node {
                position_type: PositionType::Absolute,
                top: px(110.),
                left: percent(20.),
                max_width: percent(55.),
                display: Display::None,
                ..default()
            },
            GlobalZIndex(31),
        ))
        .insert((Pickable::default(), TextLayout::default()))
        .observe(click);
}
fn feed(owner: NonSend<HostOwner>, time: Res<Time>, mut queue: ResMut<Queue>) {
    let generation = owner.play.as_ref().map(|_| owner.generation());
    if queue.generation != generation {
        *queue = Queue {
            generation,
            ..default()
        };
    }
    if let Some(play) = &owner.play {
        queue.pending.extend(play.link.take_achievement_banners());
        if queue.current.is_none() {
            queue.next(time.elapsed());
        }
    }
}
fn click(
    event: On<Activate>,
    banners: Query<(), With<Banner>>,
    owner: NonSend<HostOwner>,
    time: Res<Time>,
    mut queue: ResMut<Queue>,
) {
    if banners.contains(event.entity)
        && owner.play.is_some()
        && queue.generation == Some(owner.generation())
    {
        queue.next(time.elapsed());
    }
}
fn project(
    time: Res<Time>,
    mut queue: ResMut<Queue>,
    mut banners: Query<(&mut Text, &mut TextColor, &mut Node), With<Banner>>,
) {
    if queue.current.as_ref().is_some_and(|(_, start)| {
        time.elapsed().saturating_sub(*start) >= FADE_IN + HOLD + FADE_OUT
    }) {
        queue.next(time.elapsed());
    }
    for (mut text, mut color, mut node) in &mut banners {
        let (value, alpha) = queue.current.as_ref().map_or(("", 0.), |(text, start)| {
            let elapsed = time.elapsed().saturating_sub(*start);
            let alpha = if elapsed < FADE_IN {
                elapsed.as_secs_f32() / FADE_IN.as_secs_f32()
            } else if elapsed <= FADE_IN + HOLD {
                1.
            } else {
                1. - (elapsed - FADE_IN - HOLD).as_secs_f32() / FADE_OUT.as_secs_f32()
            };
            (text.as_str(), alpha)
        });
        if text.0 != value {
            text.0 = value.to_owned();
        }
        color.0.set_alpha(alpha);
        node.display = if queue.current.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::{PlayView, play_channel};
    use oteryn_renderer::TileCoord;
    use oteryn_session::{
        AppliedDelta, SessionEvent,
        achievement_notices::{AchievementEarned, AchievementWatermark},
    };
    type Result = std::result::Result<(), Box<dyn std::error::Error>>;

    fn app() -> std::result::Result<(App, crate::play::Outbox), Box<dyn std::error::Error>> {
        let mut owner = HostOwner::new(None)?;
        let (link, _, outbox) = play_channel();
        outbox.achievement_notices_select_for_test();
        owner.play = Some(crate::bevy_shell::owner::ActivePlay {
            view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
            link,
        });
        let mut app = App::new();
        app.insert_non_send(owner)
            .init_resource::<Time>()
            .init_resource::<Queue>()
            .add_systems(Update, (feed, project).chain());
        spawn_banner(&mut app.world_mut().commands(), Handle::default());
        Ok((app, outbox))
    }
    fn send(outbox: &crate::play::Outbox, count: u32) {
        outbox.push(vec![SessionEvent::AchievementEarned(AppliedDelta {
            server_sequence: count.into(),
            base_revision: (count - 1).into(),
            new_revision: count.into(),
            value: AchievementEarned {
                key: format!("achievement/{count}"),
                name: format!("Earned {count}"),
                watermark: AchievementWatermark {
                    fact_count: count,
                    total_points: count,
                },
            },
        })]);
    }
    fn advance(app: &mut App, millis: u64) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(millis));
        app.update();
    }
    fn text(app: &mut App) -> String {
        app.world_mut()
            .query_filtered::<&Text, With<Banner>>()
            .iter(app.world())
            .map(|t| t.0.clone())
            .collect()
    }
    fn alpha(app: &mut App) -> f32 {
        app.world_mut()
            .query_filtered::<&TextColor, With<Banner>>()
            .iter(app.world())
            .map(|t| t.0.alpha())
            .sum()
    }
    #[test]
    fn banner_queue_fifo() -> Result {
        let (mut app, outbox) = app()?;
        for n in 1..=40 {
            send(&outbox, n);
        }
        app.update();
        for n in 1..=40 {
            assert_eq!(
                text(&mut app),
                oteryn_client_ui::achievement_notices::advancement_text(&format!("Earned {n}"))
            );
            advance(&mut app, 3700);
        }
        assert!(text(&mut app).is_empty());
        Ok(())
    }
    #[test]
    fn banner_fade_timeline() -> Result {
        let (mut app, outbox) = app()?;
        send(&outbox, 1);
        app.update();
        assert_eq!(alpha(&mut app), 0.);
        advance(&mut app, 200);
        assert_eq!(alpha(&mut app), 0.5);
        advance(&mut app, 200);
        assert_eq!(alpha(&mut app), 1.);
        advance(&mut app, 3000);
        assert_eq!(alpha(&mut app), 1.);
        advance(&mut app, 150);
        assert!((alpha(&mut app) - 0.5).abs() < 0.001);
        advance(&mut app, 150);
        assert!(text(&mut app).is_empty());
        Ok(())
    }
    #[test]
    fn banner_click_advances() -> Result {
        let (mut app, outbox) = app()?;
        send(&outbox, 1);
        send(&outbox, 2);
        app.update();
        let entity = app
            .world_mut()
            .query_filtered::<Entity, With<Banner>>()
            .single(app.world())?;
        assert_ne!(app.world().get::<Pickable>(entity), Some(&Pickable::IGNORE));
        app.world_mut().trigger(Activate { entity });
        app.update();
        assert!(text(&mut app).contains("Earned 2"));
        assert_eq!(alpha(&mut app), 0.);
        Ok(())
    }
    #[test]
    fn banner_achievement_feed_once_and_session_cleanup() -> Result {
        let (mut app, outbox) = app()?;
        send(&outbox, 1);
        app.update();
        app.update();
        assert!(text(&mut app).contains("Earned 1"));
        advance(&mut app, 3700);
        assert!(text(&mut app).is_empty());
        send(&outbox, 2);
        app.update();
        app.world_mut().non_send_mut::<HostOwner>().close();
        app.update();
        assert!(text(&mut app).is_empty());
        Ok(())
    }
}
