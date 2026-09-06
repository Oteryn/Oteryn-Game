//! Allocation custody supplied by the operation owner, never a driver-local budget.
//!
//! This primitive does not calculate TLS or PostgreSQL allocation bounds. The caller
//! must prove a capacity/overlap bound and acquire it *before* allocating. In
//! particular, an operation's normal timeout is not permission to release a
//! reservation while a connection, shared backing, or completion still owns bytes.

use std::fmt;
use std::sync::Arc;

/// Bounded accounting failures; rejected input is never copied into an error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BudgetError {
    /// The owning operation cannot reserve the requested additional capacity.
    Unavailable,
    /// Byte accounting overflowed its representation.
    Overflow,
    /// A transfer requested more bytes than the current reservation owns.
    InvalidTransfer,
}

impl fmt::Display for BudgetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Unavailable => "operation resource budget unavailable",
            Self::Overflow => "operation resource accounting overflow",
            Self::InvalidTransfer => "invalid operation resource transfer",
        })
    }
}

impl std::error::Error for BudgetError {}

/// The existing operation owner's shared ledger.
///
/// Implementations must atomically reserve against that same owner's remaining
/// allowance, without creating another allowance for a socket or driver. A failed
/// call must leave the balance unchanged. These methods must not allocate from
/// peer-sized input. The owner accounts this ledger's own storage before creating
/// it; cloning its `Arc` here creates no second allocation or resource ceiling.
///
/// This trait deliberately supplies no maximum, fallback, or default ledger.
pub trait ResourceBudget: Send + Sync {
    /// Atomically debit `bytes`, or return a bounded error without a partial debit.
    fn try_reserve(&self, bytes: usize) -> Result<(), BudgetError>;

    /// Return previously reserved bytes after their owning allocation is gone.
    fn release(&self, bytes: usize);
}

/// Exclusive custody of charged bytes in the supplied owner ledger.
///
/// This value is intentionally not `Clone`: a cloned backing allocation needs a
/// fresh reservation. Sharing a backing is safe only when this reservation stays
/// with that backing until its final owner drops it. The reservation's inline
/// metadata is part of the containing owner allocation, not an accounting exemption.
pub struct ResourceReservation {
    budget: Arc<dyn ResourceBudget>,
    bytes: usize,
}

impl ResourceReservation {
    /// Reserve before performing the allocation that will consume these bytes.
    pub fn try_new(budget: Arc<dyn ResourceBudget>, bytes: usize) -> Result<Self, BudgetError> {
        budget.try_reserve(bytes)?;
        Ok(Self { budget, bytes })
    }

    /// Bytes still owned by this reservation.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    /// Reserve additional capacity before growth, including old/new overlap.
    ///
    /// On overflow or exhaustion the original reservation remains unchanged.
    pub fn try_grow(&mut self, additional: usize) -> Result<(), BudgetError> {
        let next = self
            .bytes
            .checked_add(additional)
            .ok_or(BudgetError::Overflow)?;
        self.budget.try_reserve(additional)?;
        self.bytes = next;
        Ok(())
    }

    /// Transfer custody without acquiring or releasing any owner capacity.
    ///
    /// The caller retains both charges during overlapping allocation lifetimes.
    /// Dropping the returned reservation is lawful only when its corresponding
    /// allocation has ended; splitting alone does not prove a buffer was freed.
    pub fn split_off(&mut self, bytes: usize) -> Result<Self, BudgetError> {
        let remaining = self
            .bytes
            .checked_sub(bytes)
            .ok_or(BudgetError::InvalidTransfer)?;
        let transferred = Self {
            budget: Arc::clone(&self.budget),
            bytes,
        };
        self.bytes = remaining;
        Ok(transferred)
    }

    /// Attach an already reserved allocation to its custody.
    ///
    /// Acquire this reservation *before* constructing `value`. Its bound covers
    /// allocations destroyed by `value`. The containing owner separately charges
    /// the inline `Charged`/reservation representation and any enclosing Arc/Box:
    /// an inner destructor cannot prove its enclosing allocation has been freed.
    /// This helper cannot infer bounds or validate an opaque dependency
    /// allocation. It guarantees destruction of `value` precedes release.
    pub fn bind<T>(self, value: T) -> Charged<T> {
        Charged {
            value,
            reservation: self,
        }
    }
}

impl Drop for ResourceReservation {
    fn drop(&mut self) {
        self.budget.release(self.bytes);
    }
}

/// An allocation and its custody; field order makes backing destruction happen first.
pub struct Charged<T> {
    value: T,
    reservation: ResourceReservation,
}

impl<T> Charged<T> {
    /// Borrow the retained object without detaching it from its charge.
    pub fn get(&self) -> &T {
        &self.value
    }

    /// The retained charge, including any proved spare capacity.
    pub fn charged_bytes(&self) -> usize {
        self.reservation.bytes()
    }
}

impl fmt::Debug for ResourceReservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceReservation")
            .field("bytes", &self.bytes)
            .finish()
    }
}

impl<T> fmt::Debug for Charged<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Charged")
            .field("charged_bytes", &self.charged_bytes())
            .finish()
    }
}
