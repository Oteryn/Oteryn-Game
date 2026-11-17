use oteryn_gameplay_dispatch::gameplay_transport::world_spatial::{
    ChannelEntities, ChannelEntity, SessionVisibility, VisibilityUpdate, VisibleKind,
};
use oteryn_protocol_oteryn::world_spatial::{
    ActorPosition, StepDirection, WorldSpatialObservation,
};

#[test]
fn qualification_party_icons_preserve_header_until_own_push_is_projected()
-> Result<(), Box<dyn std::error::Error>> {
    let debug = |error| format!("{error:?}");
    let before = ActorPosition {
        x: 32369,
        y: 32238,
        floor: 7,
    };
    let after = ActorPosition {
        x: before.x + 1,
        ..before
    };
    let mut channel = ChannelEntities {
        content_generation: [7; 32],
        observer: [1; 16],
        observed_at_ms: 0,
        lights: Default::default(),
        entities: vec![ChannelEntity {
            display_name: None,
            health_percent: 100,
            direction: StepDirection::East,
            kind: VisibleKind::Player,
            identity: [1; 16],
            generation: 1,
            position: before,
            revision: 1,
        }],
    };
    let mut view = SessionVisibility::with_objects(false);
    let shown = view.snapshot(&channel, &mut |_| Ok(())).map_err(debug)?;
    // Another actor pushes us between entity projection and the icon observation.
    channel.entities[0].position = after;
    channel.entities[0].revision += 1;
    let icons = view
        .stationary_delta(WorldSpatialObservation {
            content_generation: channel.content_generation,
            actor_position: after,
        })
        .map_err(debug)?;
    assert!(icons.enter.is_empty() && icons.update.is_empty() && icons.leave.is_empty());
    assert_eq!(icons.actor_position, shown.entities[0].position);
    let VisibilityUpdate::Delta(pushed) = view.refresh(&channel, &mut |_| Ok(())).map_err(debug)?
    else {
        return Err("own push was not projected as a delta".into());
    };
    assert_eq!(pushed.actor_position, after);
    assert_eq!(pushed.update.len(), 1);
    assert_eq!(pushed.update[0].entity.identity, shown.own_identity);
    assert_eq!(pushed.update[0].position, after);
    let icons = view
        .stationary_delta(WorldSpatialObservation {
            content_generation: channel.content_generation,
            actor_position: after,
        })
        .map_err(debug)?;
    assert_eq!(icons.actor_position, after);
    Ok(())
}
