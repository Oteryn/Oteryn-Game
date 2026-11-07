//! Closed, read-only Rakesh source room projected from the current Bundle owner.
use super::*;

pub(super) fn project(
    origin: &super::super::NativeEntryMovementCells,
    bounds: [i32; 6],
    start: super::super::NativeEntryStart,
) -> Option<QualifiedNativeSpellWorld> {
    if bounds != [33034, 32983, 33051, 33000, 8, 8]
        || !matches!(
            origin.index(),
            crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)
        )
        || origin.scope().generation_digest == [0; 32]
        || (start.x, start.y, start.floor) != (33045, 32988, -8)
    {
        return None;
    }
    let mut scope = origin.scope().clone();
    // The SourceWorld/view convention is negative native floors. A distinct
    // profile retains only the horizontal physics proven here; it never claims
    // the Thalom-only source height/floor-change qualification.
    scope.coordinate_frame =
        crate::content::CoordinateFrameRef::new("oteryn:frame/srv7-rakesh-source-portal-r1")
            .ok()?;
    let mut map_basis = b"oteryn:rakesh-source-geometry-view:v1\0".to_vec();
    map_basis.extend_from_slice(origin.scope().map_revision.as_str().as_bytes());
    for value in bounds {
        map_basis.extend_from_slice(&value.to_be_bytes());
    }
    let map_digest = crate::content::digest::sha256(&map_basis);
    let map_text: String = map_digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    scope.map_revision = crate::content::MapRevisionRef::new(&format!("sha256:{map_text}")).ok()?;
    let mut cells = BTreeMap::new();
    let mut immutable_tiles = BTreeMap::new();
    for y in bounds[1]..=bounds[3] {
        for x in bounds[0]..=bounds[2] {
            let source = LogicalCell { x, y, z: 8 };
            let child = LogicalCell { x, y, z: -8 };
            let tile = origin
                .spell_tiles()
                .lookup(origin.scope(), source)
                .ok()
                .cloned();
            let tile = tile.filter(|tile| {
                let expected = if tile.ground_present() && !tile.flags().block_solid {
                    CollisionClass::Walkable
                } else {
                    CollisionClass::Blocked
                };
                origin.index().lookup(origin.scope(), source).ok() == Some(expected)
            });
            if let Some(tile) = &tile {
                immutable_tiles.insert(child, tile.clone());
            }
            cells.insert(child, tile);
        }
    }
    let view = QualifiedNativeSpellWorld {
        activity_start: Some(start),
        wheel_removal_allowed: false,
        scope,
        temple: LogicalCell {
            x: start.x,
            y: start.y,
            z: i32::from(start.floor),
        },
        cells,
        immutable_tiles,
        pending_placements: Vec::new(),
        native_item_bindings: BTreeMap::new(),
        bounds: [bounds[0], bounds[1], bounds[2], bounds[3], -8, -8],
        bindings: Vec::new(),
    };
    for (x, y) in [(33045, 32988), (33042, 32993), (33043, 32986)] {
        if view.lookup(view.scope(), LogicalCell { x, y, z: -8 }).ok()
            != Some(CollisionClass::Walkable)
        {
            return None;
        }
    }
    Some(view)
}
