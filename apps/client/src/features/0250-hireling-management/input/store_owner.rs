//! Reuse the canonical purchase/UUID implementation without widening another feature's cfg.
#[cfg(any(windows, test))]
pub(in crate::play) use crate::play::feature_0154_store::view::native::{customization, purchase};

// This source contains the pure purchase helpers plus a native-only text presenter.
// On this route the Store's own module is not compiled, so there is still only one
// purchase implementation and request-ID sequence in the executable.
#[cfg(not(any(windows, test)))]
#[expect(
    dead_code,
    reason = "the reused Store source also defines an unused native text presenter"
)]
// The referenced source belongs to another lane and is read-only here.
#[rustfmt::skip]
#[path = "../../0154-store/native.rs"]
mod canonical;
#[cfg(not(any(windows, test)))]
pub(in crate::play) use canonical::{customization, purchase};
