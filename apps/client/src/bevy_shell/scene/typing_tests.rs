use super::*;
use oteryn_renderer::TileCoord;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn typing_app() -> Result<(App, crate::play::Outbox), Box<dyn std::error::Error>> {
    use crate::play::{PlayView, play_channel};
    use oteryn_session::{
        ActorPosition, EntityDetail, EntityDisplayName, EntityKind, EntityRef, StepDirection,
        VisibleAppearance, WorldSpatialEntity,
    };
    let mut view = PlayView::new(TileCoord::new(100, 100), 7, &[])?;
    view.set_presentations(vec![VisibleAppearance {
        entity: WorldSpatialEntity {
            entity: EntityRef {
                identity: [7; 16],
                generation: 9,
            },
            kind: EntityKind::Player,
            position: ActorPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            display_name: Some(EntityDisplayName::new("Speaker").map_err(|e| format!("{e:?}"))?),
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        },
        look: None,
        light: None,
        forge_stacks: None,
    }])?;
    let (link, _, outbox) = play_channel();
    let mut host = HostOwner::new(None)?;
    host.play = Some(super::super::owner::ActivePlay { view, link });
    let mut app = App::new();
    app.insert_non_send(host)
        .init_resource::<WorldViewport>()
        .add_systems(Update, super::typing::project);
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    Ok((app, outbox))
}

fn typing_update(outbox: &crate::play::Outbox, active: bool, generation: u64) -> TestResult {
    use oteryn_session::{AppliedDelta, ChatSpeaker, SessionEvent, player_typing::TypingUpdate};
    outbox.push(vec![SessionEvent::PlayerTyping(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: TypingUpdate {
            speaker: ChatSpeaker {
                identity: [7; 16],
                generation: std::num::NonZeroU64::new(generation).ok_or("generation")?,
            },
            typing: active,
        },
    })]);
    Ok(())
}

#[test]
fn typing_icon_drawn_for_active_indicator() -> TestResult {
    let (mut app, outbox) = typing_app()?;
    app.update();
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<super::typing::TypingDot>>()
            .iter(app.world())
            .count(),
        0
    );
    typing_update(&outbox, true, 9)?;
    app.update();
    let host = app.world().non_send::<HostOwner>();
    let scene = host.play.as_ref().ok_or("play")?.view.scene();
    let [x, y] = scene.view().tile_to_screen(TileCoord::new(100, 100));
    let half_tile = scene.view().tile_px() as f32 / 2.;
    let scene_size = Vec2::new(
        scene.view().viewport_px().0 as f32,
        scene.view().viewport_px().1 as f32,
    );
    let window = app
        .world_mut()
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .single(app.world())?;
    let size = Vec2::new(
        window.physical_width() as f32,
        window.physical_height() as f32,
    );
    let offset = ((size - scene_size) * 0.5).floor();
    let first = Vec2::new(x + half_tile + 2. + 1., y - 14. - 4. + 1.);
    let expected = Vec2::new(
        first.x + offset.x - size.x / 2.,
        size.y / 2. - first.y - offset.y,
    );
    let mut dots: Vec<_> = app
        .world_mut()
        .query_filtered::<(&Sprite, &Transform, &RenderLayers), With<super::typing::TypingDot>>()
        .iter(app.world())
        .map(|(s, t, l)| (s.custom_size, t.translation, l.clone()))
        .collect();
    dots.sort_by(|a, b| a.1.x.total_cmp(&b.1.x));
    assert_eq!(dots.len(), 3);
    assert_eq!(dots[0].0, Some(Vec2::splat(2.)));
    assert_eq!(dots[0].1.truncate(), expected);
    assert_eq!(dots[0].2, RenderLayers::layer(1));
    assert_eq!(dots[2].1.x - dots[0].1.x, 6.);
    Ok(())
}

#[test]
fn typing_icon_cleared() -> TestResult {
    let (mut app, outbox) = typing_app()?;
    for (active, generation, count) in [(true, 9, 3), (false, 9, 0), (true, 8, 0), (true, 9, 3)] {
        typing_update(&outbox, active, generation)?;
        app.update();
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<super::typing::TypingDot>>()
                .iter(app.world())
                .count(),
            count
        );
    }
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .view
        .set_presentations(Vec::new())?;
    app.update();
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<super::typing::TypingDot>>()
            .iter(app.world())
            .count(),
        0
    );
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    app.update();
    Ok(())
}
