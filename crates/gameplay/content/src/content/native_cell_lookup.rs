//! Read-only candidate collision port. Each implementation retains its own
//! admission/encoding limits; this trait cannot construct cells or activate pins.
use super::static_cell_engine::{
    EngineeringStaticCellIndex, EngineeringStaticCellScope, StaticCellEngineError,
};
use super::{CollisionClass, LogicalCell};
mod sealed {
    pub trait Sealed {}
    impl Sealed for super::EngineeringStaticCellIndex {}
    impl Sealed for crate::content::project::native_spell_world::QualifiedNativeSpellWorld {}
    impl Sealed for super::BundleCollisionIndex {}
}
pub trait NativeStaticCellLookup: sealed::Sealed {
    fn lookup(
        &self,
        active_scope: &EngineeringStaticCellScope,
        cell: LogicalCell,
    ) -> Result<CollisionClass, StaticCellEngineError>;
}
impl NativeStaticCellLookup for EngineeringStaticCellIndex {
    fn lookup(
        &self,
        scope: &EngineeringStaticCellScope,
        cell: LogicalCell,
    ) -> Result<CollisionClass, StaticCellEngineError> {
        self.lookup(scope, cell)
    }
}
impl NativeStaticCellLookup for super::project::native_spell_world::QualifiedNativeSpellWorld {
    fn lookup(
        &self,
        scope: &EngineeringStaticCellScope,
        cell: LogicalCell,
    ) -> Result<CollisionClass, StaticCellEngineError> {
        self.lookup(scope, cell)
    }
}
impl NativeStaticCellLookup for BundleCollisionIndex {
    fn lookup(
        &self,
        scope: &EngineeringStaticCellScope,
        cell: LogicalCell,
    ) -> Result<CollisionClass, StaticCellEngineError> {
        self.lookup(scope, cell)
    }
}

/// Keeps original engineering serialization/admission separate from the
/// bounded source-map owner and the booted world bundle (MAP-CUTOVER-1a). Each variant is
/// constructed only by qualification or by a bundle boot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeMovementCollisionIndex {
    Entry(EngineeringStaticCellIndex),
    Source(super::project::native_spell_world::QualifiedNativeSpellWorld),
    Bundle(BundleCollisionIndex),
}
impl sealed::Sealed for NativeMovementCollisionIndex {}
impl NativeMovementCollisionIndex {
    pub fn lookup(
        &self,
        scope: &EngineeringStaticCellScope,
        cell: LogicalCell,
    ) -> Result<CollisionClass, StaticCellEngineError> {
        match self {
            Self::Entry(index) => index.lookup(scope, cell),
            Self::Source(index) => index.lookup(scope, cell),
            Self::Bundle(index) => index.lookup(scope, cell),
        }
    }
    pub fn source_world(
        &self,
    ) -> Option<&super::project::native_spell_world::QualifiedNativeSpellWorld> {
        match self {
            Self::Source(world) => Some(world),
            Self::Entry(_) | Self::Bundle(_) => None,
        }
    }
}
impl NativeStaticCellLookup for NativeMovementCollisionIndex {
    fn lookup(
        &self,
        scope: &EngineeringStaticCellScope,
        cell: LogicalCell,
    ) -> Result<CollisionClass, StaticCellEngineError> {
        self.lookup(scope, cell)
    }
}

/// Read-only bundle collision supplied by the World owner.
pub trait BundleCollisionLookup: std::fmt::Debug + std::any::Any + Send + Sync {
    /// A bounded activity keeps its already qualified map owner as its parent.
    /// Consumers needing the concrete map follow this link, while collision
    /// continues to use the child's clipped lookup.
    fn parent_index(&self) -> Option<&BundleCollisionIndex> {
        None
    }
    fn lookup(
        &self,
        scope: &EngineeringStaticCellScope,
        cell: LogicalCell,
    ) -> Result<CollisionClass, StaticCellEngineError>;
    /// Complete House-marker evidence from this exact loaded bundle scope.
    /// Older or unqualified ports provide no absence proof.
    fn house_tiles_present(
        &self,
        _scope: &EngineeringStaticCellScope,
    ) -> Result<bool, StaticCellEngineError> {
        Err(StaticCellEngineError::Unqualified)
    }
    fn same_index(&self, other: &dyn BundleCollisionLookup) -> bool;
    /// A bounded read-only template, or fresh map state for an already allocated Instance.
    fn activity_region(
        &self,
        _bounds: [i32; 6],
        _instance: Option<[u8; 16]>,
        _start: [i32; 3],
    ) -> Option<BundleCollisionIndex> {
        None
    }
    fn for_activity_instance(&self, _instance: [u8; 16]) -> Option<BundleCollisionIndex> {
        None
    }
}

/// A shared bundle collision port. The World owner retains its concrete map view.
#[derive(Debug, Clone)]
pub struct BundleCollisionIndex(pub std::sync::Arc<dyn BundleCollisionLookup>);
impl PartialEq for BundleCollisionIndex {
    fn eq(&self, other: &Self) -> bool {
        self.0.same_index(other.0.as_ref())
    }
}
impl Eq for BundleCollisionIndex {}
impl BundleCollisionIndex {
    pub fn activity_region(
        &self,
        bounds: [i32; 6],
        instance: Option<[u8; 16]>,
        start: [i32; 3],
    ) -> Option<Self> {
        self.0.activity_region(bounds, instance, start)
    }
    pub fn for_activity_instance(&self, instance: [u8; 16]) -> Option<Self> {
        self.0.for_activity_instance(instance)
    }
    pub fn lookup(
        &self,
        scope: &EngineeringStaticCellScope,
        cell: LogicalCell,
    ) -> Result<CollisionClass, StaticCellEngineError> {
        self.0.lookup(scope, cell)
    }

    pub fn house_tiles_present(
        &self,
        scope: &EngineeringStaticCellScope,
    ) -> Result<bool, StaticCellEngineError> {
        self.0.house_tiles_present(scope)
    }
}
