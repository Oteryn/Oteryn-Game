//! Test-only borrow of the actual retained transaction for SQL-abort fault injection.
use super::*;
impl<'db> CurrentMonsterMap<'_, 'db> {
    /// Counts requested prefetch positions, tile reads, and zone reads, in that order.
    pub fn field_read_counts_for_test(&self) -> [usize; 3] {
        std::array::from_fn(|index| {
            self.field_read_counts[index].load(std::sync::atomic::Ordering::Relaxed)
        })
    }

    pub fn transaction_for_test(&mut self) -> &mut sqlx::Transaction<'db, sqlx::Postgres> {
        self.tx
    }
}
