//! SEC-TRUST-1/SEC-REL-1 arithmetic and role policy only (SEC_CLIENT01 §3).
//!
//! These leaves are not registered in the server. Passing their predicates never
//! authenticates signed bytes, resolves current trust, bootstraps a restored
//! store, grants admission, or authorizes a challenge/outcome.

pub(crate) mod release_policy;
pub(crate) mod trust_policy;
