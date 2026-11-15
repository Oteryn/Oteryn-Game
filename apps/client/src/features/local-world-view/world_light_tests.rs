use super::*;
use oteryn_session::{ActorPosition, world_map::*};
#[test]
fn server_item_appearance_emits_light_in_real_scene() -> Result<(), Box<dyn std::error::Error>> {
    let position = ActorPosition {
        x: 100,
        y: 200,
        floor: -7,
    };
    let snapshot = WorldMapViewSnapshot {
        header: MapViewHeader {
            origin: position,
            first_visible_floor: -7,
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 1,
        },
        tiles: vec![oteryn_session::world_map::MapTile {
            position,
            ground_speed: 150,
            more: false,
            items: vec![MapItem {
                definition: MapDefinition::Item(std::num::NonZeroU32::MIN),
                count: 1,
                sub_type: 0,
                appearance_id: 100,
                origin: MapOrigin::BaseOrdinal {
                    ordinal: 0,
                    object_revision: 0,
                },
            }],
        }],
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let world = World::resolve_live_map(&root, &snapshot)?;
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 200), -7, &[])?;
    view.set_world(std::sync::Arc::new(world))?;
    assert_eq!(view.scene().sprites().lighting.sources.len(), 1);
    let light = &view.scene().sprites().lighting.sources[0];
    assert_eq!(light.radius, 3. * view.scene().view().tile_px() as f32);
    assert_eq!(light.color, oteryn_renderer::lighting::light_color(156));
    Ok(())
}
