use crate::play::{PlayView, Pushed};
use oteryn_renderer::TileCoord;
use oteryn_session::world_clock::WorldClock;
#[test]
fn clock_and_light_survive_scene_rebuilds() -> Result<(), oteryn_renderer::BatchError> {
    let mut view = PlayView::new(TileCoord::new(0, 0), 7, &[])?;
    let original = view.scene().atlas().rgba().to_vec();
    view.apply_pushed(&Pushed { world_clock: Some(WorldClock { tibian_minute: 1439, light_level: 128, light_color: 180 }), ..Pushed::default() })?;
    assert_eq!(view.world_clock_text().as_deref(), Some("23:59"));
    assert_ne!(view.scene().atlas().rgba(), original);
    let lit = view.scene().atlas().clone();
    view.apply_pushed(&Pushed::default())?;
    assert_eq!(view.scene().atlas(), &lit);
    Ok(())
}
