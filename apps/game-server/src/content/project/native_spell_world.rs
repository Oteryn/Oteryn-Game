//! Explicit source-map candidate. This does not widen FirstProduction/OTSCENG1,
//! activate an artifact, grant mutable item ownership, or manufacture adjacent floors.
use super::native_spell_tiles::{NativeSpellTileFlags, QualifiedSpellTile, SourceFloorChange};
use super::{
    ProjectError, ProjectV2Family, ProjectV2SourceIdentityBinding,
    ProjectV2SourceIdentityDisposition,
};
use crate::content::static_cell_engine::{EngineeringStaticCellScope, StaticCellEngineError};
use crate::content::{CollisionClass, LogicalCell};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub(crate) const SOURCE_WORLD_PROFILE: &str = "native-source-spell-world-qualification-3";
pub(crate) const SOURCE_WORLD_FRAME: &str = "oteryn:frame/canary-thalom-spell-r3";
pub(crate) const SOURCE_WORLD_MAP: &str = "oteryn:map/canary-thalom-spell-r3";
const PIN: &str = "99902524e052f37574194466c2949c576e4ab269";
const DOCUMENT_SHA: &str = "44f860d20712a32e6bc22aa269d3e220d39d164b7738d3c150a406579849b5f5";
const MAX_BYTES: usize = 8 * 1024 * 1024;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceClosure {
    path: String,
    git_blob: String,
    sha256: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceTown {
    town_id: u32,
    name: String,
    temple: [i32; 3],
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeFrame {
    key: String,
    map_revision: String,
    origin_x: i32,
    origin_y: i32,
    floor_transform: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceItemPolicy {
    server_item_id: u32,
    ground: bool,
    ground_speed: u16,
    block_solid: bool,
    block_projectile: bool,
    block_pathfind: bool,
    movable: bool,
    pickupable: bool,
    container: bool,
    floor_change: bool,
    floor_change_kind: Option<SourceFloorChange>,
    has_height: bool,
    dynamic_kind: Option<String>,
    xml_attributes: Vec<BTreeMap<String, String>>,
    magic_field: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceItem {
    server_item_id: u32,
    depth: u16,
    attributes: BTreeMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceSemantics {
    ground: Option<u32>,
    ground_speed: Option<u16>,
    top_ids: Vec<u32>,
    unmaterialized_dynamic_items: Vec<u32>,
    height_count: u16,
    floor_changes: Vec<SourceFloorChange>,
    flags: NativeSpellTileFlags,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceTile {
    source_position: [i32; 3],
    native_position: [i32; 3],
    otbm_flags: u32,
    house_id: Option<u32>,
    zones: Vec<u16>,
    items: Vec<SourceItem>,
    semantics: SourceSemantics,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceWorldDocument {
    schema: String,
    candidate_profile: String,
    source_repository: String,
    source_revision: String,
    source_closure: Vec<SourceClosure>,
    imports: Vec<super::ImportBatch>,
    sources: Vec<super::ProjectV2Source>,
    source_bounds: [i32; 6],
    source_town: SourceTown,
    native_frame: NativeFrame,
    source_identity_bindings: Vec<ProjectV2SourceIdentityBinding>,
    native_item_bindings: Vec<ProjectV2SourceIdentityBinding>,
    item_policies: Vec<SourceItemPolicy>,
    tiles: Vec<SourceTile>,
}
/// Private construction binds the actual provenance-qualified source cells to one
/// caller's independently active World/frame/map/generation scope. Matching this
/// stored scope alone grants no activation or actor/session authority.
/// Exact pending source placement, constructed only by the pinned map qualifier.
/// It is initialization data, not current Item ownership or empty-tile evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QualifiedMutableSourcePlacement {
    cell: LogicalCell,
    ordinal: u16,
    source_position: [i32; 3],
    binding: ProjectV2SourceIdentityBinding,
    attributes: BTreeMap<String, serde_json::Value>,
    source_kind: Option<String>,
    movable: bool,
    pickupable: bool,
    block_solid: bool,
    block_projectile: bool,
    block_pathfind: bool,
    has_height: bool,
}
impl QualifiedMutableSourcePlacement {
    pub(crate) fn cell(&self) -> LogicalCell {
        self.cell
    }
    pub(crate) fn ordinal(&self) -> u16 {
        self.ordinal
    }
    pub(crate) fn source_position(&self) -> [i32; 3] {
        self.source_position
    }
    pub(crate) fn binding(&self) -> &ProjectV2SourceIdentityBinding {
        &self.binding
    }
    pub(crate) fn attributes(&self) -> &BTreeMap<String, serde_json::Value> {
        &self.attributes
    }
    pub(crate) fn source_kind(&self) -> Option<&str> {
        self.source_kind.as_deref()
    }
    pub(crate) fn movable(&self) -> bool {
        self.movable
    }
    pub(crate) fn pickupable(&self) -> bool {
        self.pickupable
    }
    pub(crate) fn block_solid(&self) -> bool {
        self.block_solid
    }
    pub(crate) fn block_projectile(&self) -> bool {
        self.block_projectile
    }
    pub(crate) fn block_pathfind(&self) -> bool {
        self.block_pathfind
    }
    pub(crate) fn has_height(&self) -> bool {
        self.has_height
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QualifiedNativeSpellWorld {
    scope: EngineeringStaticCellScope,
    cells: BTreeMap<LogicalCell, Option<QualifiedSpellTile>>,
    immutable_tiles: BTreeMap<LogicalCell, QualifiedSpellTile>,
    pending_placements: Vec<QualifiedMutableSourcePlacement>,
    native_item_bindings: BTreeMap<u32, ProjectV2SourceIdentityBinding>,
    bounds: [i32; 6],
    bindings: Vec<ProjectV2SourceIdentityBinding>,
}
impl QualifiedNativeSpellWorld {
    pub(crate) fn native_item_binding(&self, id: u32) -> Option<&ProjectV2SourceIdentityBinding> {
        self.native_item_bindings.get(&id)
    }
    pub(crate) fn pending_mutable_placements(&self) -> &[QualifiedMutableSourcePlacement] {
        &self.pending_placements
    }
    /// Ground and immutable source stack facts only. This cannot authorize entry
    /// through pending/current mutable owners and leaves ordinary lookup Unknown.
    pub(crate) fn immutable_tile(
        &self,
        active_scope: &EngineeringStaticCellScope,
        cell: LogicalCell,
    ) -> Result<&QualifiedSpellTile, StaticCellEngineError> {
        if active_scope != &self.scope {
            return Err(StaticCellEngineError::ScopeMismatch);
        }
        self.immutable_tiles
            .get(&cell)
            .ok_or(StaticCellEngineError::Absent)
    }
    pub(crate) fn contains_capture_cell(&self, cell: LogicalCell) -> bool {
        let [min_x, min_y, max_x, max_y, min_z, max_z] = self.bounds;
        (min_x..=max_x).contains(&cell.x)
            && (min_y..=max_y).contains(&cell.y)
            && (min_z..=max_z).contains(&cell.z)
    }
    pub(crate) fn spell_tiles(&self) -> super::QualifiedSpellTiles {
        super::QualifiedSpellTiles::from_source_world(self)
    }
    pub(crate) fn scope(&self) -> &EngineeringStaticCellScope {
        &self.scope
    }
    pub(crate) const fn entry_start(&self) -> super::NativeEntryStart {
        super::NativeEntryStart {
            x: 0,
            y: 0,
            floor: -5,
        }
    }
    pub(crate) fn cell_count(&self) -> usize {
        self.cells.len()
    }
    pub(crate) fn source_identity_bindings(&self) -> &[ProjectV2SourceIdentityBinding] {
        &self.bindings
    }
    pub(crate) fn lookup(
        &self,
        active_scope: &EngineeringStaticCellScope,
        cell: LogicalCell,
    ) -> Result<CollisionClass, StaticCellEngineError> {
        if active_scope != &self.scope {
            return Err(StaticCellEngineError::ScopeMismatch);
        }
        let tile = self
            .cells
            .get(&cell)
            .ok_or(StaticCellEngineError::Absent)?
            .as_ref()
            .ok_or(StaticCellEngineError::Unqualified)?;
        Ok(if tile.ground_present() && !tile.flags().block_solid {
            CollisionClass::Walkable
        } else {
            CollisionClass::Blocked
        })
    }
    pub(super) fn qualified_cells(&self) -> &BTreeMap<LogicalCell, Option<QualifiedSpellTile>> {
        &self.cells
    }
    pub(super) const fn native_bounds(&self) -> [i32; 6] {
        self.bounds
    }
}
fn invalid() -> ProjectError {
    ProjectError::InvalidProject("native source world qualification")
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub(crate) fn qualify(
    bytes: &[u8],
    scope: EngineeringStaticCellScope,
) -> Result<QualifiedNativeSpellWorld, ProjectError> {
    if bytes.len() > MAX_BYTES
        || scope.generation_digest == [0; 32]
        || scope.coordinate_frame.as_str() != SOURCE_WORLD_FRAME
        || scope.map_revision.as_str() != SOURCE_WORLD_MAP
        || hex(&crate::content::digest::sha256(bytes)) != DOCUMENT_SHA
    {
        return Err(invalid());
    }
    let doc: SourceWorldDocument = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if doc.schema != "OTERYN_NATIVE_SPELL_SOURCE_WORLD/v1"
        || doc.candidate_profile != SOURCE_WORLD_PROFILE
        || doc.source_repository != "https://github.com/opentibiabr/canary"
        || doc.source_revision != PIN
        || doc.source_bounds != [5840, 5283, 5871, 5314, 5, 8]
        || doc.source_town
            != (SourceTown {
                town_id: 6,
                name: "Thalom".into(),
                temple: [5854, 5298, 5],
            })
        || doc.native_frame
            != (NativeFrame {
                key: SOURCE_WORLD_FRAME.into(),
                map_revision: SOURCE_WORLD_MAP.into(),
                origin_x: 5854,
                origin_y: 5298,
                floor_transform: "native_floor=-source_z".into(),
            })
        || doc.tiles.len() != 2833
        || doc.tiles.len() > 4096
        || doc.source_closure.len() != 14
    {
        return Err(invalid());
    }
    let mut policies = BTreeMap::new();
    for p in &doc.item_policies {
        if p.server_item_id == 0
            || p.server_item_id > 65535
            || policies.insert(p.server_item_id, p).is_some()
        {
            return Err(invalid());
        }
    }
    let native_item_bindings: BTreeMap<u32, ProjectV2SourceIdentityBinding> = doc
        .native_item_bindings
        .iter()
        .map(|binding| {
            let id = binding.external_id.parse::<u32>().map_err(|_| invalid())?;
            if binding.external_id != id.to_string()
                || binding.target.family != ProjectV2Family::Item
                || binding.identity_namespace != "ots/item_server_id"
                || binding.source_revision != "ff7ede593c69d4c658b382c97443e8155926924a"
                || binding.source_key != "oteryn:source.crystalserver"
            {
                return Err(invalid());
            }
            Ok((id, binding.clone()))
        })
        .collect::<Result<_, ProjectError>>()?;
    if native_item_bindings.len() != 37
        || native_item_bindings.len() != doc.native_item_bindings.len()
    {
        return Err(invalid());
    }
    let mut ids = BTreeSet::new();
    for binding in &doc.source_identity_bindings {
        let id = binding.external_id.parse::<u32>().map_err(|_| invalid())?;
        let policy = policies.get(&id).ok_or_else(invalid)?;
        let family = if policy.ground {
            ProjectV2Family::Terrain
        } else {
            ProjectV2Family::Item
        };
        let prefix = if policy.ground { "terrain" } else { "item" };
        if binding.external_id != id.to_string()
            || binding.source_revision != PIN
            || binding.source_key != "oteryn:source/canary-spell-world-r3"
            || binding.identity_namespace != "ots/item_server_id"
            || binding.disposition != ProjectV2SourceIdentityDisposition::AcceptedAlias
            || binding.target.family != family
            || if let Some(native) = native_item_bindings.get(&id) {
                binding.target != native.target
            } else {
                binding.target.key != format!("oteryn:{prefix}.source.canary.id{id}")
                    || binding.target.revision != "source-map-r3"
            }
            || !ids.insert(id)
        {
            return Err(invalid());
        }
    }
    if ids.len() != policies.len() {
        return Err(invalid());
    }
    let mut cells = BTreeMap::new();
    let mut immutable_tiles = BTreeMap::new();
    let mut pending_placements = Vec::new();
    for row in &doc.tiles {
        let [x, y, z] = row.source_position;
        let native = [x - 5854, y - 5298, -z];
        if !(5840..=5871).contains(&x)
            || !(5283..=5314).contains(&y)
            || !(5..=8).contains(&z)
            || row.native_position != native
            || row.house_id.is_some()
        {
            return Err(invalid());
        }
        let mut flags = NativeSpellTileFlags {
            block_solid: false,
            block_projectile: false,
            immovable_block_solid: false,
            immovable_block_item: false,
            immovable_nonfield_block_item: false,
            floor_change: false,
            protection_zone: row.otbm_flags & 1 != 0,
        };
        let mut ground = None;
        let mut top = Vec::new();
        let mut dynamic = Vec::new();
        let mut height_count = 0_u16;
        let mut floor_changes = BTreeSet::new();
        let mut immutable_flags = flags;
        let mut immutable_top = Vec::new();
        let mut immutable_height = 0_u16;
        let mut immutable_floor_changes = BTreeSet::new();
        for (ordinal, item) in row.items.iter().enumerate() {
            let p = policies.get(&item.server_item_id).ok_or_else(invalid)?;
            if item.depth != 0 {
                continue;
            }
            let movable = p.movable
                && !item.attributes.contains_key("5")
                && item.attributes.get("4").and_then(serde_json::Value::as_u64) != Some(100);
            if p.ground {
                ground = Some(p.server_item_id)
            } else {
                top.push(p.server_item_id);
                if movable
                    || p.pickupable
                    || p.container
                    || p.dynamic_kind.is_some()
                    || item.attributes.contains_key("dest")
                {
                    dynamic.push(p.server_item_id);
                    let binding = doc
                        .source_identity_bindings
                        .iter()
                        .find(|b| b.external_id == p.server_item_id.to_string())
                        .ok_or_else(invalid)?;
                    pending_placements.push(QualifiedMutableSourcePlacement {
                        cell: LogicalCell {
                            x: native[0],
                            y: native[1],
                            z: native[2],
                        },
                        ordinal: u16::try_from(ordinal).map_err(|_| invalid())?,
                        source_position: row.source_position,
                        binding: binding.clone(),
                        attributes: item.attributes.clone(),
                        source_kind: p.dynamic_kind.clone(),
                        movable,
                        pickupable: p.pickupable,
                        block_solid: p.block_solid,
                        block_projectile: p.block_projectile,
                        block_pathfind: p.block_pathfind,
                        has_height: p.has_height,
                    });
                }
            }
            let mutable = !p.ground
                && (movable
                    || p.pickupable
                    || p.container
                    || p.dynamic_kind.is_some()
                    || item.attributes.contains_key("dest"));
            if !mutable {
                if !p.ground {
                    immutable_top.push(p.server_item_id)
                }
                immutable_flags.block_solid |= p.block_solid;
                immutable_flags.block_projectile |= p.block_projectile;
                immutable_flags.immovable_block_solid |= p.block_solid && !movable;
                immutable_flags.immovable_block_item |= p.block_pathfind && !movable;
                immutable_flags.immovable_nonfield_block_item |=
                    p.block_pathfind && !p.magic_field && !movable;
                immutable_flags.floor_change |= p.floor_change;
                if p.has_height {
                    immutable_height = immutable_height.checked_add(1).ok_or_else(invalid)?;
                }
                if let Some(kind) = p.floor_change_kind {
                    immutable_floor_changes.insert(kind);
                }
            }
            flags.block_solid |= p.block_solid;
            flags.block_projectile |= p.block_projectile;
            flags.immovable_block_solid |= p.block_solid && !movable;
            flags.immovable_block_item |= p.block_pathfind && !movable;
            flags.immovable_nonfield_block_item |= p.block_pathfind && !p.magic_field && !movable;
            flags.floor_change |= p.floor_change;
            if p.floor_change != p.floor_change_kind.is_some() {
                return Err(invalid());
            }
            if p.has_height {
                height_count = height_count.checked_add(1).ok_or_else(invalid)?;
            }
            if let Some(kind) = p.floor_change_kind {
                floor_changes.insert(kind);
            }
        }
        let speed = ground.map(|id| policies[&id].ground_speed);
        if row.semantics
            != (SourceSemantics {
                ground,
                ground_speed: speed,
                top_ids: top.clone(),
                unmaterialized_dynamic_items: dynamic.clone(),
                height_count,
                floor_changes: floor_changes.iter().copied().collect(),
                flags,
            })
        {
            return Err(invalid());
        }
        immutable_tiles.insert(
            LogicalCell {
                x: native[0],
                y: native[1],
                z: native[2],
            },
            QualifiedSpellTile::from_source_world(
                ground,
                speed,
                immutable_top,
                immutable_flags,
                immutable_height,
                immutable_floor_changes.into_iter().collect(),
                row.otbm_flags & 1 == 0 && row.otbm_flags & 4 != 0,
                row.otbm_flags & 1 == 0 && row.otbm_flags & 4 == 0 && row.otbm_flags & 16 != 0,
            ),
        );
        let tile = if dynamic.is_empty() {
            Some(QualifiedSpellTile::from_source_world(
                ground,
                speed,
                top,
                flags,
                height_count,
                floor_changes.into_iter().collect(),
                row.otbm_flags & 1 == 0 && row.otbm_flags & 4 != 0,
                row.otbm_flags & 1 == 0 && row.otbm_flags & 4 == 0 && row.otbm_flags & 16 != 0,
            ))
        } else {
            None
        };
        if cells
            .insert(
                LogicalCell {
                    x: native[0],
                    y: native[1],
                    z: native[2],
                },
                tile,
            )
            .is_some()
        {
            return Err(invalid());
        }
    }
    let start = cells
        .get(&LogicalCell { x: 0, y: 0, z: -5 })
        .and_then(Option::as_ref)
        .ok_or_else(invalid)?;
    if !start.ground_present() || start.flags().block_solid {
        return Err(invalid());
    }
    Ok(QualifiedNativeSpellWorld {
        scope,
        cells,
        bounds: [-14, -15, 17, 16, -8, -5],
        bindings: doc.source_identity_bindings,
        immutable_tiles,
        pending_placements,
        native_item_bindings,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceWorldClientCell {
    pub(crate) position: [i32; 3],
    pub(crate) ground: Option<String>,
    pub(crate) immutable_items: Vec<String>,
    pub(crate) owner_pending: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceWorldClientDocument {
    pub(crate) schema: String,
    pub(crate) coordinate_frame: String,
    pub(crate) map_revision: String,
    pub(crate) start: [i32; 3],
    pub(crate) cells: Vec<SourceWorldClientCell>,
}
pub(crate) fn client_projection(bytes: &[u8]) -> Result<Vec<u8>, ProjectError> {
    if bytes.len() > MAX_BYTES || hex(&crate::content::digest::sha256(bytes)) != DOCUMENT_SHA {
        return Err(invalid());
    }
    let doc: SourceWorldDocument = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    let keys: BTreeMap<u32, String> = doc
        .source_identity_bindings
        .iter()
        .map(|binding| {
            Ok((
                binding.external_id.parse::<u32>().map_err(|_| invalid())?,
                binding.target.key.clone(),
            ))
        })
        .collect::<Result<_, ProjectError>>()?;
    let cells = doc
        .tiles
        .iter()
        .map(|tile| {
            let pending = !tile.semantics.unmaterialized_dynamic_items.is_empty();
            let immutable_items = if pending {
                Vec::new()
            } else {
                tile.semantics
                    .top_ids
                    .iter()
                    .map(|id| keys.get(id).cloned().ok_or_else(invalid))
                    .collect::<Result<_, _>>()?
            };
            Ok(SourceWorldClientCell {
                position: tile.native_position,
                ground: tile
                    .semantics
                    .ground
                    .map(|id| keys.get(&id).cloned().ok_or_else(invalid))
                    .transpose()?,
                immutable_items,
                owner_pending: pending,
            })
        })
        .collect::<Result<_, ProjectError>>()?;
    serde_json::to_vec(&SourceWorldClientDocument {
        schema: "OTERYN_NATIVE_SPELL_SOURCE_WORLD_CLIENT/v1".into(),
        coordinate_frame: SOURCE_WORLD_FRAME.into(),
        map_revision: SOURCE_WORLD_MAP.into(),
        start: [0, 0, -5],
        cells,
    })
    .map_err(|_| invalid())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    fn scope() -> EngineeringStaticCellScope {
        let world = crate::foundation::WorldId::decode(&[
            0, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1,
        ])
        .unwrap();
        let room = crate::content::qualify_native_entry_room(world).unwrap();
        let mut scope = room.movement_cells().scope().clone();
        scope.coordinate_frame =
            crate::content::CoordinateFrameRef::new(SOURCE_WORLD_FRAME).unwrap();
        scope.map_revision = crate::content::MapRevisionRef::new(SOURCE_WORLD_MAP).unwrap();
        // Pure source qualification tests; this digest grants no activation/session authority.
        scope.generation_digest = [2; 32];
        scope
    }
    const SOURCE: &[u8] = include_bytes!(
        "../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
    );
    #[test]
    fn mutable_source_capture_stays_unknown_until_actual_item_owner() {
        let scope = scope();
        let map = qualify(SOURCE, scope.clone()).unwrap();
        assert_eq!(map.pending_mutable_placements().len(), 57);
        let cells: BTreeSet<_> = map
            .pending_mutable_placements()
            .iter()
            .map(|p| p.cell())
            .collect();
        assert_eq!(cells.len(), 49);
        for cell in cells {
            assert_eq!(
                map.lookup(&scope, cell),
                Err(StaticCellEngineError::Unqualified)
            );
            let immutable = map.immutable_tile(&scope, cell).unwrap();
            assert_eq!(
                immutable.ground_present(),
                immutable.ground_source_id().is_some()
            );
            assert!(immutable.no_pvp_zone().is_some());
            assert!(immutable.pvp_zone().is_some());
        }
        for p in map.pending_mutable_placements() {
            let native = map
                .native_item_binding(p.binding().external_id.parse().unwrap())
                .unwrap();
            assert_eq!(native.target, p.binding().target);
            assert_eq!(
                native.source_revision,
                "ff7ede593c69d4c658b382c97443e8155926924a"
            );
            assert_eq!(p.binding().source_revision, PIN);
        }
        let mut stale = scope.clone();
        stale.generation_digest = [7; 32];
        let first = map.pending_mutable_placements()[0].cell();
        assert!(map.immutable_tile(&stale, first).is_err());
    }
    #[test]
    fn genuine_source_multifloor_rope_and_origin_are_qualified() {
        let scope = scope();
        let map = qualify(SOURCE, scope.clone()).unwrap();
        assert_eq!(map.cell_count(), 2833);
        assert_eq!(map.entry_start().floor, -5);
        let tiles = map.spell_tiles();
        let rope = LogicalCell { x: 7, y: 14, z: -8 };
        let dest = LogicalCell { x: 7, y: 13, z: -7 };
        assert_eq!(map.lookup(&scope, rope), Ok(CollisionClass::Walkable));
        assert_eq!(map.lookup(&scope, dest), Ok(CollisionClass::Walkable));
        assert_eq!(
            tiles.lookup(&scope, rope).unwrap().ground_source_id(),
            Some(386)
        );
        assert_eq!(
            tiles.lookup(&scope, dest).unwrap().ground_speed(),
            Some(140)
        );
        assert!(
            tiles
                .lookup(&scope, LogicalCell { x: 0, y: 0, z: -5 })
                .unwrap()
                .flags()
                .protection_zone
        );
        let mut stale = scope.clone();
        stale.generation_digest = [3; 32];
        assert_eq!(
            map.lookup(&stale, rope),
            Err(StaticCellEngineError::ScopeMismatch)
        );
    }
    #[test]
    fn source_mutable_tiles_and_outside_capture_remain_unknown() {
        let scope = scope();
        let map = qualify(SOURCE, scope.clone()).unwrap();
        let tiles = map.spell_tiles();
        let unknown = map
            .qualified_cells()
            .iter()
            .find(|(_, v)| v.is_none())
            .unwrap()
            .0;
        assert_eq!(
            map.lookup(&scope, *unknown),
            Err(StaticCellEngineError::Unqualified)
        );
        assert!(matches!(
            tiles.lookup(&scope, *unknown),
            Err(super::super::SpellTileLookupError::Unknown)
        ));
        let outside = LogicalCell { x: 18, y: 0, z: -5 };
        assert!(!map.contains_capture_cell(outside));
        assert!(matches!(
            tiles.lookup(&scope, outside),
            Err(super::super::SpellTileLookupError::Unknown)
        ));
    }
    #[test]
    fn source_flags_offsets_and_bindings_cannot_be_substituted() {
        let mut doc: serde_json::Value = serde_json::from_slice(SOURCE).unwrap();
        doc["tiles"][0]["semantics"]["flags"]["block_solid"] = serde_json::json!(false);
        assert!(qualify(&serde_json::to_vec(&doc).unwrap(), scope()).is_err());
        let mut wrong = scope();
        wrong.coordinate_frame =
            crate::content::CoordinateFrameRef::new("oteryn:entry.frame").unwrap();
        assert!(qualify(SOURCE, wrong).is_err());
        let client = client_projection(SOURCE).unwrap();
        let text = String::from_utf8(client).unwrap();
        for private in [
            "source_revision",
            "source_closure",
            "server_item_id",
            "block_solid",
            "ots/item_server_id",
            "git_blob",
        ] {
            assert!(!text.contains(private));
        }
    }
}
