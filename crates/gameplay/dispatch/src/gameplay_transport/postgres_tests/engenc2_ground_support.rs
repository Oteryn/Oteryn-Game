//! One compiled copy of the private production reader per PG test executable.
#[path = "../../../../durability/src/durability/quest_world_callback/ground_read.rs"]
mod reader;
pub(crate) use reader::{Context, matches};
