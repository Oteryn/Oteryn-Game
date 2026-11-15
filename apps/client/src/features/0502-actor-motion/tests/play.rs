use super::*;
use oteryn_protocol_oteryn::{actor_motion::ActorMotion, world_spatial_entities as wire};

fn motion_view(forecast: u32) -> Result<PlayView, Box<dyn std::error::Error>> {
    let mut view = view()?;
    let entity = wire::WorldSpatialEntity {
        motion: Some(ActorMotion {
            prewalk_ms: [forecast; 8],
            ..ActorMotion::default()
        }),
        display_name: None,
        kind: wire::EntityKind::Player,
        entity: EntityRef {
            identity: [1; 16],
            generation: 1,
        },
        position: ActorPosition {
            x: 100,
            y: 200,
            floor: 0,
        },
        detail: wire::EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let snapshot = wire::WorldSpatialEntitiesSnapshot {
        content_generation: [0; 32],
        actor_position: entity.position,
        own_identity: entity.entity.identity,
        entities: vec![entity],
    };
    let fields = wire::EntityFields::selected(&[6, 13, 25, 502]);
    let payload = wire::encode_world_spatial_entities_snapshot_with_fields(
        &snapshot,
        fields,
        &wire::EntityLights::new(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let (decoded, _) = wire::decode_world_spatial_entities_snapshot_with_fields(&payload, fields)
        .map_err(|e| format!("{e:?}"))?;
    view.set_own_identity(decoded.own_identity);
    view.set_presentations(vec![oteryn_session::VisibleAppearance {
        entity: decoded.entities[0],
        look: None,
        light: None,
        forge_stacks: None,
    }])?;
    Ok(view)
}

#[test]
fn actor_motion_prewalk_starts_before_reply_and_ack_does_not_restart()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = motion_view(300)?;
    view.arrow(StepDir::East);
    assert_eq!(view.send_step()?, Some(StepDir::East));
    assert_eq!(view.own(), TileCoord::new(101, 200));
    let started = view
        .walk_started()
        .ok_or("prewalk must begin before a reply")?;
    assert_eq!(view.walk_duration(), Some(Duration::from_millis(300)));
    let mut reply = moved_to(101, 200, 0);
    reply.step_wait_ms = 450;
    view.apply(&reply)?;
    assert_eq!(view.walk_started(), Some(started));
    assert_eq!(view.walk_duration(), Some(Duration::from_millis(450)));
    Ok(())
}

#[test]
fn actor_motion_prewalk_refusal_restores_origin_and_unknown_tile_waits()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = motion_view(300)?;
    view.arrow(StepDir::West);
    view.send_step()?;
    assert_eq!(view.own(), TileCoord::new(99, 200));
    let mut reply = moved_to(100, 200, 0);
    reply.disposition = StepDisposition::Blocked;
    reply.world_spatial_delta = None;
    view.apply(&reply)?;
    assert_eq!(view.own(), TileCoord::new(100, 200));
    assert_eq!(view.walk_started(), None);
    let mut unknown = motion_view(0)?;
    unknown.arrow(StepDir::East);
    unknown.send_step()?;
    assert_eq!(unknown.own(), TileCoord::new(100, 200));
    assert_eq!(unknown.walk_started(), None);
    Ok(())
}

#[test]
fn actor_motion_pushed_origin_does_not_cancel_prediction_and_relocation_does()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = motion_view(300)?;
    view.arrow(StepDir::East);
    view.send_step()?;
    view.apply_pushed(&Pushed {
        own: Some(ActorPosition {
            x: 100,
            y: 200,
            floor: 0,
        }),
        ..Pushed::default()
    })?;
    assert_eq!(view.own(), TileCoord::new(101, 200));
    assert!(view.walk_started().is_some());
    view.apply_pushed(&Pushed {
        own: Some(ActorPosition {
            x: 99,
            y: 200,
            floor: 0,
        }),
        ..Pushed::default()
    })?;
    assert_eq!(view.own(), TileCoord::new(99, 200));
    assert_eq!(view.walk_started(), None);
    let mut replacement = motion_view(300)?;
    replacement.arrow(StepDir::East);
    replacement.send_step()?;
    replacement.apply_pushed(&Pushed {
        motion_reset: true,
        own: Some(ActorPosition {
            x: 100,
            y: 200,
            floor: 0,
        }),
        ..Pushed::default()
    })?;
    assert_eq!(replacement.own(), TileCoord::new(100, 200));
    assert_eq!(replacement.walk_started(), None);
    Ok(())
}
