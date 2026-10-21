//! ADR-0019 gameplay crate `oteryn-gameplay-content`: modules moved verbatim from `oteryn-game-server` (re-exported there at the old path).

pub mod content;

#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_domain::domain;
#[allow(unused_imports)]
pub(crate) use oteryn_gameplay_foundation::foundation;
