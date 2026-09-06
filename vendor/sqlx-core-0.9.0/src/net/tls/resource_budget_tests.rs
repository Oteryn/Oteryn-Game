use crate::net::resource_budget::{BudgetError, ResourceBudget, ResourceReservation};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug)]
struct Ledger {
    limit: usize,
    used: AtomicUsize,
}
impl ResourceBudget for Ledger {
    fn try_reserve(&self, bytes: usize) -> Result<(), BudgetError> {
        self.used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(bytes).filter(|next| *next <= self.limit)
            })
            .map(|_| ())
            .map_err(|_| BudgetError::Unavailable)
    }
    fn release(&self, bytes: usize) {
        let prior = self.used.fetch_sub(bytes, Ordering::AcqRel);
        assert!(prior >= bytes, "reservation released twice");
    }
}
fn ledger(limit: usize) -> Arc<Ledger> {
    Arc::new(Ledger {
        limit,
        used: AtomicUsize::new(0),
    })
}

#[test]
fn denial_precedes_input_controlled_allocation() {
    let budget = ledger(8);
    let held = ResourceReservation::try_new(budget.clone(), 8).unwrap();
    let calls = AtomicUsize::new(0);
    let result = ResourceReservation::try_new(budget.clone(), 1).map(|reservation| {
        calls.fetch_add(1, Ordering::Relaxed);
        reservation.bind(vec![0u8; 1])
    });
    assert!(matches!(result, Err(BudgetError::Unavailable)));
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    assert_eq!(budget.used.load(Ordering::Acquire), 8);
    drop(held);
    assert_eq!(budget.used.load(Ordering::Acquire), 0);
}

#[test]
fn reservation_uses_same_owner_balance_without_extra_driver_capacity() {
    let budget = ledger(4_194_304);
    let request = ResourceReservation::try_new(budget.clone(), 524_288).unwrap();
    let driver = ResourceReservation::try_new(budget.clone(), 3_670_016).unwrap();
    assert!(ResourceReservation::try_new(budget.clone(), 1).is_err());
    drop(driver);
    assert_eq!(budget.used.load(Ordering::Acquire), 524_288);
    drop(request);
    assert_eq!(budget.used.load(Ordering::Acquire), 0);
}

#[test]
fn growth_denial_and_overflow_leave_original_custody_unchanged() {
    let budget = ledger(64);
    let mut charge = ResourceReservation::try_new(budget.clone(), 48).unwrap();
    assert_eq!(charge.try_grow(17), Err(BudgetError::Unavailable));
    assert_eq!(charge.try_grow(usize::MAX), Err(BudgetError::Overflow));
    assert_eq!(charge.bytes(), 48);
    assert_eq!(budget.used.load(Ordering::Acquire), 48);
    charge.try_grow(16).unwrap();
    assert_eq!(charge.bytes(), 64);
}

#[test]
fn splitting_transfers_without_minting_capacity_and_failed_split_is_inert() {
    let budget = ledger(100);
    let mut charge = ResourceReservation::try_new(budget.clone(), 100).unwrap();
    assert!(charge.split_off(101).is_err());
    let retained = charge.split_off(70).unwrap();
    assert_eq!(charge.bytes(), 30);
    assert_eq!(retained.bytes(), 70);
    assert_eq!(budget.used.load(Ordering::Acquire), 100);
    drop(charge);
    assert_eq!(budget.used.load(Ordering::Acquire), 70);
    drop(retained);
    assert_eq!(budget.used.load(Ordering::Acquire), 0);
}

#[test]
fn backing_is_dropped_before_its_reservation_is_released() {
    struct Backing(Arc<Ledger>);
    impl Drop for Backing {
        fn drop(&mut self) {
            assert_eq!(self.0.used.load(Ordering::Acquire), 48);
        }
    }
    let budget = ledger(48);
    let retained = ResourceReservation::try_new(budget.clone(), 48)
        .unwrap()
        .bind(Backing(budget.clone()));
    assert_eq!(retained.charged_bytes(), 48);
    let _ = retained.get();
    drop(retained);
    assert_eq!(budget.used.load(Ordering::Acquire), 0);
}

#[test]
fn connection_style_shared_backing_remains_charged_until_last_clone_drops() {
    let budget = ledger(786_432);
    let held = ResourceReservation::try_new(budget.clone(), 786_432)
        .unwrap()
        .bind([0u8; 0]);
    // The Arc itself is intentionally fixture-owned; a production Arc allocation needs its own charge.
    let first = Arc::new(held);
    let second = first.clone();
    drop(first);
    assert_eq!(budget.used.load(Ordering::Acquire), 786_432);
    drop(second);
    assert_eq!(budget.used.load(Ordering::Acquire), 0);
}

#[test]
fn concurrent_callers_cannot_exceed_shared_owner_balance() {
    use std::sync::Barrier;
    let budget = ledger(64);
    let barrier = Arc::new(Barrier::new(9));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let budget = budget.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let reservation = ResourceReservation::try_new(budget, 16).ok();
                barrier.wait();
                barrier.wait();
                reservation.is_some()
            })
        })
        .collect();
    barrier.wait();
    assert_eq!(budget.used.load(Ordering::Acquire), 64);
    barrier.wait();
    assert_eq!(
        handles
            .into_iter()
            .map(|h| usize::from(h.join().unwrap()))
            .sum::<usize>(),
        4
    );
    assert_eq!(budget.used.load(Ordering::Acquire), 0);
}

#[test]
fn cancelled_pending_work_keeps_charge_until_captured_backing_drops() {
    use std::future::{pending, Future};
    use std::task::{Context, Poll, Waker};
    let budget = ledger(48);
    let retained = ResourceReservation::try_new(budget.clone(), 48)
        .unwrap()
        .bind(vec![0u8; 48]);
    let mut work = Box::pin(async move {
        pending::<()>().await;
        retained
    });
    let mut context = Context::from_waker(Waker::noop());
    assert!(matches!(work.as_mut().poll(&mut context), Poll::Pending));
    assert_eq!(budget.used.load(Ordering::Acquire), 48);
    drop(work);
    assert_eq!(budget.used.load(Ordering::Acquire), 0);
}

#[test]
fn debugging_custody_never_formats_the_retained_input() {
    struct Sensitive;
    impl std::fmt::Debug for Sensitive {
        fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            panic!("must not format protected backing")
        }
    }
    let retained = ResourceReservation::try_new(ledger(8), 8)
        .unwrap()
        .bind(Sensitive);
    assert_eq!(format!("{retained:?}"), "Charged { charged_bytes: 8 }");
}
