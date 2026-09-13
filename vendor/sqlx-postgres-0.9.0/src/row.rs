use crate::column::ColumnIndex;
use crate::error::Error;
use crate::message::DataRow;
use crate::statement::Metadata;
use crate::value::PgValueFormat;
use crate::{PgColumn, PgValueRef, Postgres};
use sqlx_core::row::debug_row;
pub(crate) use sqlx_core::row::Row;

/// Implementation of [`Row`] for PostgreSQL.
pub struct PgRow {
    pub(crate) data: DataRow,
    pub(crate) format: PgValueFormat,
    pub(crate) metadata: Metadata,
}

impl Row for PgRow {
    type Database = Postgres;

    fn columns(&self) -> &[PgColumn] {
        &self.metadata.columns
    }

    fn try_get_raw<I>(&self, index: I) -> Result<PgValueRef<'_>, Error>
    where
        I: ColumnIndex<Self>,
    {
        let index = index.index(self)?;
        let column = &self.metadata.columns[index];
        let value = self.data.get(index);

        Ok(PgValueRef {
            format: self.format,
            row: Some(&self.data.storage),
            type_info: column.type_info.clone(),
            value,
        })
    }
}

impl ColumnIndex<PgRow> for &'_ str {
    fn index(&self, row: &PgRow) -> Result<usize, Error> {
        row.metadata
            .column_names
            .iter()
            .rev()
            .find(|(name, _)| &**name == *self)
            .map(|(_, index)| *index)
            .ok_or_else(|| Error::ColumnNotFound((*self).into()))
    }
}

impl std::fmt::Debug for PgRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        debug_row(self, f)
    }
}
