use oteryn_renderer::TileCoord;
use oteryn_session::EntityRef;
#[derive(Debug, Clone, Default)]
pub(crate) struct CreatureDrag {
    pub pointer: Option<(f64, f64)>,
    pub source: Option<(TileCoord, i16, Option<EntityRef>)>,
}
