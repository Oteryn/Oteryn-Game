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
    impl Sealed for crate::map::boot::BundleCollisionIndex {}
}
pub(crate) trait NativeStaticCellLookup: sealed::Sealed {
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
impl NativeStaticCellLookup for crate::map::boot::BundleCollisionIndex {
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
pub(crate) enum NativeMovementCollisionIndex {
    Entry(EngineeringStaticCellIndex),
    Source(super::project::native_spell_world::QualifiedNativeSpellWorld),
    Bundle(crate::map::boot::BundleCollisionIndex),
}
impl sealed::Sealed for NativeMovementCollisionIndex {}
impl NativeMovementCollisionIndex {
    pub(crate) fn lookup(
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
    pub(crate) fn source_world(
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
