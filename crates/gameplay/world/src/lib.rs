//! ADR-0019 gameplay crate `oteryn-gameplay-world`: modules moved verbatim from `oteryn-game-server` (re-exported there at the old path).

pub mod house_scope;
pub mod map;
pub mod movement;
pub mod world_clock;
pub mod world_object_revert;
pub mod world_runtime;

#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use crate::durability::native_world_items;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use crate::durability::native_world_read;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_content::content;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_domain::domain;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_durability::character_bootstrap_intent;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_durability::durability;
#[allow(unused_imports, reason = "preserve the former server module path")]
pub(crate) use oteryn_gameplay_foundation::foundation;
