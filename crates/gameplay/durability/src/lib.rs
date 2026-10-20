//! ADR-0019 gameplay crate `oteryn-gameplay-durability`: modules moved verbatim from `oteryn-game-server` (re-exported there at the old path).

pub mod durability;
pub mod character_bootstrap_intent;

#[allow(unused_imports)] pub(crate) use oteryn_gameplay_foundation::foundation;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_content::content;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_domain::domain;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_admission_evidence as admission_evidence;
#[allow(unused_imports)] pub(crate) use oteryn_gameplay_native_admission_source as native_admission_source;
