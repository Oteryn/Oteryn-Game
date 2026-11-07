//! Test-only borrow of the actual retained transaction for SQL-abort fault injection.
use super::*;
impl<'db> CurrentMonsterMap<'_, 'db> {
    pub fn transaction_for_test(&mut self) -> &mut sqlx::Transaction<'db, sqlx::Postgres> {
        self.tx
    }
}
