//! Exact real source geometry only; this does not admit an encounter or its death.
use super::*;
use crate::content::{CollisionClass, NativeEntryMovementCells, NativeEntryStart};
impl ContentActivationController {
    /// Read-only current B3 geometry, projected into the existing SourceWorld
    /// child convention. Unknown cells remain unknown. No gameplay actor,
    /// membership, cooldown, quest, MINT or scope assignment is issued here.
    pub fn qualify_rakesh_source_geometry(
        &self,
        room: &QualifiedNativeEntryRoom,
        origin: &NativeEntryMovementCells,
        origin_map_digest: [u8; 32],
    ) -> Result<QualifiedNativeEntryRoom, ActivityTemplateRefusal> {
        use ActivityTemplateRefusal as R;
        let active = self
            .active()
            .filter(|_| self.is_ready())
            .ok_or(R::NoActiveGeneration)?;
        if origin.scope() != room.movement_cells().scope()
            || origin.scope().generation_digest != active.identity().server_artifact_digest()
            || room.compiled().server_digest() != active.identity().server_artifact_digest()
            || room.compiled().client_digest() != active.identity().client_artifact_digest()
            || origin_map_digest == [0; 32]
            || room.map_revision_digest() != origin_map_digest
        {
            return Err(R::InvalidSourceCapture);
        }
        for (x, y) in [(33039, 32964), (33040, 32962)] {
            if origin
                .index()
                .lookup(origin.scope(), LogicalCell { x, y, z: 8 })
                .ok()
                != Some(CollisionClass::Walkable)
            {
                return Err(R::UnknownPlacement);
            }
        }
        let start = NativeEntryStart {
            x: 33045,
            y: 32988,
            floor: -8,
        };
        let source = crate::content::project::native_spell_world::QualifiedNativeSpellWorld::activity_from_bundle(
            origin,[33034,32983,33051,33000,8,8],start,
        ).ok_or(R::SourceMapUnavailable)?;
        room.projected_activity_view(source, start, origin_map_digest)
            .ok_or(R::SourceMapUnavailable)
    }
}
