//! Detached, finite Map DATA reads; no cached occupancy or spawn approval.
use super::*;
use crate::native_world_read::{PreparedCreatureTileRead, ReadCreatureTile};

pub struct PreparedMonsterMapRead {
    cells: Vec<PreparedCreatureTileRead>,
}
pub struct ReadPreparedMonsterMap {
    cells: Vec<ReadCreatureTile>,
}
impl PreparedMonsterMapRead {
    /// The closed source owner supplies its original candidate order. Unknown
    /// cells refuse; this does not open cells outside the qualified source map.
    pub fn prepare(
        runtime: &ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        objects: &LocalObjectRuntime,
        positions: &[MovementLocalPosition],
    ) -> Option<Self> {
        let mut cells = Vec::new();
        cells.try_reserve_exact(positions.len()).ok()?;
        for &position in positions {
            if cells
                .iter()
                .any(|cell: &PreparedCreatureTileRead| cell.position() == position)
            {
                continue;
            }
            cells.push(
                crate::native_world_read::prepare_creature_tile_read(
                    room, runtime, objects, position,
                )
                .ok()?,
            );
        }
        (!cells.is_empty()).then_some(Self { cells })
    }
    /// Caller holds only the SQL transaction/independent scope owner here.
    pub async fn read(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        authority: &SpellItemScopeAuthority,
    ) -> Option<ReadPreparedMonsterMap> {
        let mut cells = Vec::new();
        cells.try_reserve_exact(self.cells.len()).ok()?;
        for cell in &self.cells {
            cells.push(
                crate::native_world_read::read_prepared_creature_tile_in_transaction(
                    tx, authority, cell,
                )
                .await
                .ok()?,
            );
        }
        Some(ReadPreparedMonsterMap { cells })
    }
}
impl ReadPreparedMonsterMap {
    /// Keep the read transaction and its row locks alive until the synchronous
    /// consumer finishes. Child-specific occupancy MUST be reread at admission;
    /// this view contains only current Map/Item DATA, never an occupancy grant.
    pub fn into_current_view(
        self,
        runtime: &ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        objects: &LocalObjectRuntime,
        content: &NativeGameplayState,
    ) -> Option<MonsterMap> {
        if content.source_digest() != runtime.content_pin().server_artifact_digest() {
            return None;
        }
        let binding = runtime.binding();
        let mut view = MonsterMap {
            valid: true,
            scope: binding.runtime_scope(),
            generation: binding.scope_generation(),
            context: runtime.pinned_movement_context(),
            activation: runtime.content_pin().activation_sequence(),
            server: runtime.content_pin().server_artifact_digest(),
            client: runtime.content_pin().client_artifact_digest(),
            frame: runtime.content_pin().frame_binding_digest(),
            map: runtime.content_pin().map_revision_digest(),
            tiles: BTreeMap::new(),
            source_steps: Default::default(),
        };
        for cell in self.cells {
            if !cell.prepared().current_for(room, runtime, objects) {
                return None;
            }
            let at = cell.prepared().position();
            let tile = match cell.qualified_tile() {
                Some(tile) => merge_current_tile(room, content, at, tile, cell.items())?,
                None => Tile {
                    ground_clear: false,
                    admits: false,
                    solid: true,
                    source_spawn_admits: false,
                    projectile: true,
                    speed: 0,
                    field: None,
                    source_placement_proven: true,
                    source_placement: SourcePlacementTerrain {
                        exists: false,
                        ground: false,
                        immovable_solid: false,
                        protection_zone: false,
                        floor_change: false,
                        teleport: false,
                    },
                },
            };
            view.tiles.insert((at.x, at.y, at.floor), Some(tile));
        }
        Some(view)
    }
}
