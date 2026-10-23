//! APP-3 appearance selection and connection-local appearance dictionary.
mod catalogue;
mod mappings;
mod selection;
mod wire;

pub use crate::charm_wire::CyclopediaWireError as AppearanceError;
pub use crate::world_spatial_entities::EntityRef;

pub const CAPABILITY_APPEARANCE_V1: u32 = 25;
pub const COMMAND_TYPE_APPEARANCE_CHANGE: u32 = 25;
pub const STATE_DOMAIN_APPEARANCE: u32 = 21;
pub const SNAPSHOT_TYPE_APPEARANCE: u32 = 1;
pub const DELTA_TYPE_APPEARANCE_MAPPINGS: u32 = 1;
pub const DELTA_TYPE_APPEARANCE_CATALOGUE: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Context {
    pub content_generation: [u8; 32],
    pub cosmetics_digest: [u8; 32],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub outfit_key: String,
    pub colours: [u16; 4],
    pub addons: u8,
    pub mount_key: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selected {
    pub selection: Selection,
    pub revision: u64,
    pub provenance: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub context: Context,
    pub expected_root_revision: u64,
    pub prior: Option<Selected>,
    pub selection: Selection,
    pub content_revision: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeResult {
    Committed(u64),
    Rejected,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutfitOption {
    pub key: String,
    pub look_type: u32,
    pub permitted_addons: u8,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalogue {
    pub context: Context,
    pub root_revision: u64,
    pub selected: Option<Selected>,
    pub palette: Vec<[u8; 3]>,
    pub outfits: Vec<OutfitOption>,
    pub mounts: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Look {
    Outfit {
        look_type: u32,
        colours: [u16; 4],
        addons: u8,
    },
    Item {
        definition_ref: u32,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub context: Context,
    pub appearance_ref: u32,
    pub entity: EntityRef,
    pub root_revision: u64,
    pub condition_revision: u64,
    pub owner_time_us: u64,
    pub look: Look,
    pub selection_revision: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mappings {
    pub context: Context,
    pub upserts: Vec<Binding>,
    pub forgets: Vec<EntityRef>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub catalogue: Catalogue,
    pub bindings: Vec<Binding>,
    pub high_water: u32,
}

#[cfg(test)]
#[path = "appearance_tests.rs"]
mod tests;
