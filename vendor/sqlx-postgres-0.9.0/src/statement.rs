use super::{PgColumn, PgTypeInfo};
use crate::column::ColumnIndex;
use crate::error::Error;
use crate::ext::ustr::UStr;
use crate::{PgArguments, Postgres};
use std::sync::Arc;

use sqlx_core::sql_str::SqlStr;
pub(crate) use sqlx_core::statement::Statement;
use sqlx_core::Either;

#[derive(Debug, Clone)]
pub struct PgStatement {
    pub(crate) sql: SqlStr,
    pub(crate) metadata: Metadata,
}

#[derive(Debug, Default)]
pub(crate) struct PgStatementMetadata {
    pub(crate) columns: Vec<PgColumn>,
    // This `Arc` is not redundant; it's used to avoid deep-copying this map for the `Any` backend.
    // See `sqlx-postgres/src/any.rs`
    pub(crate) column_names: Vec<(UStr, usize)>,
    pub(crate) parameters: Vec<PgTypeInfo>,
}

impl Statement for PgStatement {
    type Database = Postgres;

    fn into_sql(self) -> SqlStr {
        self.sql
    }

    fn sql(&self) -> &SqlStr {
        &self.sql
    }

    fn parameters(&self) -> Option<Either<&[PgTypeInfo], usize>> {
        Some(Either::Left(&self.metadata.parameters))
    }

    fn columns(&self) -> &[PgColumn] {
        &self.metadata.columns
    }

    impl_statement_query!(PgArguments);
}

impl ColumnIndex<PgStatement> for &'_ str {
    fn index(&self, statement: &PgStatement) -> Result<usize, Error> {
        statement
            .metadata
            .column_names
            .iter()
            .rev()
            .find(|(name, _)| &**name == *self)
            .map(|(_, index)| *index)
            .ok_or_else(|| Error::ColumnNotFound((*self).into()))
    }
}

// #[cfg(feature = "any")]
// impl<'q> From<PgStatement<'q>> for crate::any::AnyStatement<'q> {
//     #[inline]
//     fn from(statement: PgStatement<'q>) -> Self {
//         crate::any::AnyStatement::<'q> {
//             columns: statement
//                 .metadata
//                 .columns
//                 .iter()
//                 .map(|col| col.clone().into())
//                 .collect(),
//             column_names: statement.metadata.column_names.clone(),
//             parameters: Some(Either::Left(
//                 statement
//                     .metadata
//                     .parameters
//                     .iter()
//                     .map(|ty| ty.clone().into())
//                     .collect(),
//             )),
//             sql: statement.sql,
//         }
//     }
// }

/// Shared allocation debit; every clone follows the private finalization path.
#[derive(Debug, Default)]
pub(crate) struct AllocationLease(
    Option<Arc<sqlx_core::net::resource_budget::ResourceReservation>>,
);
impl Clone for AllocationLease {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl Drop for AllocationLease {
    fn drop(&mut self) {
        if let Some(inner) = self.0.take() {
            drop(Arc::into_inner(inner));
        }
    }
}
impl AllocationLease {
    pub(crate) fn reserve(
        budget: Option<Arc<dyn sqlx_core::net::resource_budget::ResourceBudget>>,
        bytes: usize,
    ) -> Result<Self, Error> {
        let Some(budget) = budget else {
            return Ok(Self::default());
        };
        let bytes = bytes
            .checked_add(arc_size::<
                sqlx_core::net::resource_budget::ResourceReservation,
            >()?)
            .ok_or_else(allocation_denied)?;
        let charge = sqlx_core::net::resource_budget::ResourceReservation::try_new(budget, bytes)
            .map_err(|_| allocation_denied())?;
        Ok(Self(Some(Arc::new(charge))))
    }
}

pub(crate) fn allocation_denied() -> Error {
    Error::Io(std::io::ErrorKind::OutOfMemory.into())
}
pub(crate) fn arc_size<T>() -> Result<usize, Error> {
    std::alloc::Layout::new::<[std::sync::atomic::AtomicUsize; 2]>()
        .extend(std::alloc::Layout::new::<T>())
        .map(|(layout, _)| layout.pad_to_align().size())
        .map_err(|_| allocation_denied())
}

#[derive(Clone, Debug)]
pub(crate) struct Metadata {
    data: Arc<PgStatementMetadata>,
    _lease: AllocationLease,
}
impl Metadata {
    pub(crate) fn new(data: PgStatementMetadata, lease: AllocationLease) -> Self {
        Self {
            data: Arc::new(data),
            _lease: lease,
        }
    }
    pub(crate) fn empty(
        budget: Option<Arc<dyn sqlx_core::net::resource_budget::ResourceBudget>>,
    ) -> Result<Self, Error> {
        let lease = AllocationLease::reserve(budget, arc_size::<PgStatementMetadata>()?)?;
        Ok(Self::new(PgStatementMetadata::default(), lease))
    }
}
impl std::ops::Deref for Metadata {
    type Target = PgStatementMetadata;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[cfg(test)]
pub(crate) mod custody_test_support {
    use sqlx_core::net::resource_budget::{BudgetError, ResourceBudget};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    pub(crate) struct Ledger {
        held: AtomicUsize,
        limit: AtomicUsize,
    }
    impl Ledger {
        pub(crate) fn new(limit: usize) -> Arc<Self> {
            Arc::new(Self {
                held: AtomicUsize::new(0),
                limit: AtomicUsize::new(limit),
            })
        }
        pub(crate) fn held(&self) -> usize {
            self.held.load(Ordering::SeqCst)
        }
        pub(crate) fn limit(&self, limit: usize) {
            self.limit.store(limit, Ordering::SeqCst);
        }
    }
    impl ResourceBudget for Ledger {
        fn try_reserve(&self, bytes: usize) -> Result<(), BudgetError> {
            self.held
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |held| {
                    held.checked_add(bytes)
                        .filter(|next| *next <= self.limit.load(Ordering::SeqCst))
                })
                .map(|_| ())
                .map_err(|_| BudgetError::Unavailable)
        }
        fn release(&self, bytes: usize) {
            assert!(self.held.fetch_sub(bytes, Ordering::SeqCst) >= bytes);
        }
    }
}

#[cfg(test)]
mod custody_tests {
    use super::custody_test_support::Ledger;
    use super::*;
    #[test]
    fn metadata_clones_hold_allocation_after_origin_drops() {
        let budget = Ledger::new(usize::MAX);
        let metadata = Metadata::empty(Some(budget.clone())).unwrap();
        let held = budget.held();
        let clone = metadata.clone();
        drop(metadata);
        assert_eq!(budget.held(), held);
        drop(clone);
        assert_eq!(budget.held(), 0);
        budget.limit(held - 1);
        assert!(Metadata::empty(Some(budget.clone())).is_err());
        assert_eq!(budget.held(), 0);
    }
}
