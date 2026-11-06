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
    fn lookup(
        &self,
        scope: &EngineeringStaticCellScope,
        cell: LogicalCell,
    ) -> Result<CollisionClass, StaticCellEngineError>;
    fn same_index(&self, other: &dyn BundleCollisionLookup) -> bool;
    /// Immutable parent retained by a content-owned bounded room view.
    fn parent_index(&self) -> Option<&BundleCollisionIndex> {
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
    /// The map owner beneath content-owned immutable room bounds.
    pub fn map_source(&self) -> &dyn BundleCollisionLookup {
        let mut source = self.0.as_ref();
        // Qualification creates at most one wrapper per template. The defensive bound
        // prevents a foreign implementation from introducing an unbounded parent cycle.
        for _ in 0..16 {
            let Some(parent) = source.parent_index() else {
                break;
            };
            source = parent.0.as_ref();
        }
        source
    }
    pub(crate) fn activity_region(
        &self,
        scope: EngineeringStaticCellScope,
        bounds: [i32; 6],
    ) -> Self {
        Self(std::sync::Arc::new(ActivityBundleCollision {
            source: self.clone(),
            scope,
            bounds,
        }))
    }

    pub fn lookup(
        &self,
        scope: &EngineeringStaticCellScope,
        cell: LogicalCell,
    ) -> Result<CollisionClass, StaticCellEngineError> {
        self.0.lookup(scope, cell)
    }
}

#[derive(Debug)]
struct ActivityBundleCollision {
    source: BundleCollisionIndex,
    scope: EngineeringStaticCellScope,
    bounds: [i32; 6],
}
impl BundleCollisionLookup for ActivityBundleCollision {
    fn lookup(
        &self,
        scope: &EngineeringStaticCellScope,
        cell: LogicalCell,
    ) -> Result<CollisionClass, StaticCellEngineError> {
        if scope != &self.scope {
            return Err(StaticCellEngineError::ScopeMismatch);
        }
        let [x0, y0, x1, y1, z0, z1] = self.bounds;
        if !(x0..=x1).contains(&cell.x)
            || !(y0..=y1).contains(&cell.y)
            || !(z0..=z1).contains(&cell.z)
        {
            return Err(StaticCellEngineError::Absent);
        }
        self.source.lookup(scope, cell)
    }
    fn same_index(&self, other: &dyn BundleCollisionLookup) -> bool {
        (other as &dyn std::any::Any)
            .downcast_ref::<Self>()
            .is_some_and(|other| {
                self.bounds == other.bounds
                    && self.scope == other.scope
                    && self.source == other.source
            })
    }
    fn parent_index(&self) -> Option<&BundleCollisionIndex> {
        Some(&self.source)
    }
}
