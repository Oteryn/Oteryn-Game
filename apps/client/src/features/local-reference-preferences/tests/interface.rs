use super::*;

#[test]
fn tooltips_use_visible_server_names_delay_and_generation() -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::HoverTooltip;
    use std::time::{Duration, Instant};
    let entity = WorldSpatialEntity {
        entity: EntityRef { identity: [7; 16], generation: 1 },
        kind: oteryn_session::EntityKind::Creature,
        display_name: Some(oteryn_session::EntityDisplayName::new("Rat").map_err(|_| "name")?),
        position: ActorPosition { x: 101, y: 200, floor: 0 },
        detail: oteryn_session::EntityDetail::Actor { direction: StepDirection::South, appearance_ref: 1, health_percent: 100 },
    };
    let mut play = view()?.with_entities(vec![entity])?;
    let [x, y] = play.scene().view().tile_to_screen(TileCoord::new(101, 200));
    let at = [x + 1.0, y + 1.0];
    let target = play.tooltip_target(at).ok_or("missing visible target")?;
    let mut hover = HoverTooltip::default();
    let now = Instant::now();
    let delay = Duration::from_millis(500);
    assert_eq!(hover.update(Some(target.clone()), now, delay), None);
    assert_eq!(hover.update(Some(target.clone()), now + delay, delay), Some("Rat"));
    play.entities[0].entity.generation += 1;
    play.rebuild()?;
    assert_eq!(hover.update(play.tooltip_target(at), now + delay, delay), None);
    let mut preferences = play.preferences().clone();
    preferences.tooltips_enabled = false;
    play.set_preferences(preferences)?;
    assert_eq!(play.tooltip_target(at), None);
    assert_eq!(hover.update(None, now + delay + delay, delay), None);
    Ok(())
}
