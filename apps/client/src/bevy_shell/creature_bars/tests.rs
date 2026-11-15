use super::*;
use crate::bevy_shell::owner::{ActivePlay, HostOwner};
use crate::play::{PlayView, play_channel};
use oteryn_renderer::TileCoord;
use oteryn_session::{
    ActorPosition, EntityDetail, EntityKind, EntityRef, StepDirection, WorldSpatialEntity,
};

#[test]
fn otc_textmsg_bevy_health_bars_follow_visible_actors_and_settings()
-> Result<(), Box<dyn std::error::Error>> {
    let mut host = HostOwner::new(None)?;
    let (link, _commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    let name = oteryn_session::EntityDisplayName::new("Visible actor")
        .map_err(|error| format!("{error:?}"))?;
    let entity = |id, kind, floor, x| WorldSpatialEntity {
        entity: EntityRef {
            identity: [id; 16],
            generation: 1,
        },
        kind,
        position: ActorPosition { x, y: 100, floor },
        display_name: Some(name),
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 60,
        },
    };
    let entities = vec![
        entity(1, EntityKind::Creature, 7, 100),
        entity(2, EntityKind::Player, 7, 101),
        entity(3, EntityKind::Npc, 7, 102),
        entity(4, EntityKind::Creature, 6, 100),
        entity(5, EntityKind::Creature, 7, 1000),
    ];
    view = view.with_entities(entities.clone())?;
    view.set_presentations(
        entities
            .into_iter()
            .map(|entity| oteryn_session::VisibleAppearance {
                entity,
                look: None,
                light: None,
                forge_stacks: None,
            })
            .collect(),
    )?;
    assert_eq!(view.scene().name_count(), 3);
    host.play = Some(ActivePlay { view, link });
    let mut app = App::new();
    app.insert_non_send(host);
    install(&mut app);
    app.update();
    let bars = app
        .world_mut()
        .query::<(&CreatureBar, &Sprite, &Transform)>()
        .iter(app.world())
        .map(|(_, sprite, transform)| (sprite.color, transform.translation.z))
        .collect::<Vec<_>>();
    assert_eq!(
        bars.len(),
        6,
        "three actors each have a black track and coloured fill"
    );
    assert_eq!(
        bars.iter()
            .filter(|(colour, _)| *colour == Color::srgb_u8(161, 161, 0))
            .count(),
        3
    );
    outbox.bevy_hud_fixture(
        &[],
        Some(oteryn_session::ActorVitals {
            health: 25,
            max_health: 100,
            ..Default::default()
        }),
    );
    {
        let mut host = app.world_mut().non_send_mut::<HostOwner>();
        let play = host.play.as_mut().ok_or("play")?;
        play.view.tick(&play.link).map_err(|_| "tick")?;
    }
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&CreatureBar>()
            .iter(app.world())
            .count(),
        8,
        "own vitals also produce a bar"
    );
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .creature_bars = false;
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&CreatureBar>()
            .iter(app.world())
            .count(),
        0
    );
    Ok(())
}

#[test]
fn otc_textmsg_health_colour_thresholds() {
    for (percent, expected) in [
        (100, [0, 188, 0]),
        (93, [0, 188, 0]),
        (92, [80, 161, 80]),
        (61, [80, 161, 80]),
        (60, [161, 161, 0]),
        (31, [161, 161, 0]),
        (30, [191, 10, 10]),
        (9, [191, 10, 10]),
        (8, [145, 15, 15]),
        (4, [145, 15, 15]),
        (3, [133, 12, 12]),
        (0, [133, 12, 12]),
    ] {
        assert_eq!(super::health_rgb(percent), expected, "{percent}%");
    }
}
