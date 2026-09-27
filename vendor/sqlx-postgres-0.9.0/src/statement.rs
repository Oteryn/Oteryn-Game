use super::{PgColumn, PgTypeInfo};
use crate::column::ColumnIndex;
use crate::error::Error;
use crate::ext::ustr::UStr;
use crate::{PgArguments, Postgres};
use std::sync::Arc;

use sqlx_core::net::{ResourceBudget, ResourceReservation};
use sqlx_core::sql_str::SqlStr;
pub(crate) use sqlx_core::statement::Statement;
use sqlx_core::{Either, HashMap};

#[derive(Debug, Clone)]
pub struct PgStatement {
    pub(crate) sql: SqlStr,
    pub(crate) metadata: Arc<PgStatementMetadata>,
}

#[derive(Debug, Default)]
pub(crate) struct PgStatementMetadata {
    pub(crate) columns: Vec<PgColumn>,
    pub(crate) column_names: Option<Arc<HashMap<UStr, usize>>>,
    pub(crate) first_slice_column_names: Option<Vec<(UStr, usize)>>,
    pub(crate) parameters: Vec<PgTypeInfo>,
    pub(crate) _allocation: Option<Arc<ResourceReservation>>,
}

impl PgStatementMetadata {
    pub(crate) fn column_index(&self, name: &str) -> Option<usize> {
        if let Some(names) = &self.first_slice_column_names {
            return names
                .iter()
                .rev()
                .find(|(column_name, _)| &**column_name == name)
                .map(|(_, index)| *index);
        }
        self.column_names
            .as_ref()
            .and_then(|names| names.get(name).copied())
    }

    #[cfg(feature = "any")]
    pub(crate) fn any_column_names(&self) -> Result<Arc<HashMap<UStr, usize>>, Error> {
        self.column_names.clone().ok_or_else(|| {
            Error::AnyDriverError(
                "Any driver is outside the WP3 PostgreSQL first-slice profile".into(),
            )
        })
    }

    pub(crate) fn empty_first_slice(
        budget: Arc<dyn ResourceBudget>,
    ) -> Result<Arc<Self>, Error> {
        use std::alloc::Layout;
        let counters = Layout::array::<std::sync::atomic::AtomicUsize>(2)
            .map_err(|_| Error::Io(std::io::ErrorKind::OutOfMemory.into()))?;
        let (layout, _) = counters
            .extend(Layout::new::<Self>())
            .map_err(|_| Error::Io(std::io::ErrorKind::OutOfMemory.into()))?;
        let allocation = ResourceReservation::try_new_shared(
            budget,
            layout.pad_to_align().size(),
        )
        .map_err(|_| Error::Io(std::io::ErrorKind::OutOfMemory.into()))?;
        Ok(Arc::new(Self {
            columns: Vec::new(),
            column_names: None,
            first_slice_column_names: Some(Vec::new()),
            parameters: Vec::new(),
            _allocation: Some(allocation),
        }))
    }
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
            .column_index(self)
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


#[cfg(test)]
mod wp3_metadata_charge_tests {
    use super::*;
    use sqlx_core::net::BudgetError;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Ledger(AtomicUsize);

    impl ResourceBudget for Ledger {
        fn try_reserve(&self, bytes: usize) -> Result<(), BudgetError> {
            self.0.fetch_add(bytes, Ordering::AcqRel);
            Ok(())
        }

        fn release(&self, bytes: usize) {
            self.0.fetch_sub(bytes, Ordering::AcqRel);
        }
    }

    #[test]
    fn wp3_metadata_charge_follows_final_arc_owner() {
        let ledger = Arc::new(Ledger(AtomicUsize::new(0)));
        let owner: Arc<dyn ResourceBudget> = ledger.clone();
        let metadata = PgStatementMetadata::empty_first_slice(owner).unwrap();
        let held = ledger.0.load(Ordering::Acquire);
        assert!(held > 0);
        let clone = metadata.clone();
        drop(metadata);
        assert_eq!(ledger.0.load(Ordering::Acquire), held);
        drop(clone);
        assert_eq!(ledger.0.load(Ordering::Acquire), 0);
    }
}
