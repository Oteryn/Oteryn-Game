//! Native Svargrond and Tibiadrome arena controls. Server results are the sole progress authority.
//! Accepted native presentation difference: docs/contracts/protocol-oteryn/v1/arena-v1.md,
//! Native client section (Ctrl+Shift+Z, command controls, server-only progress).
//! The Z binding preserves CL1/CL2 account achievements on Ctrl+Shift+A.
//! Donor hypothesis: Crystal 201189f871d23597b82caf88fd611454a2ddefb8,
//! data-global/lib/quests/svargrond_arena.lua:9,118-173 (timer, ranks and fees),
//! data-global/startup/tables/chest.lua:2307-2419 (independent reward groups).
//! These controls consume existing messages; no game rule is introduced here.
mod input;
mod protocol;
mod state;
mod view;
pub use input::{DROME_REWARDS, drome_intent, open_key};
pub use protocol::{DromeAction, DromeReply, Intent, apply_result, request};
pub use state::ArenaPanel;
pub use view::panel_text;
#[cfg(test)]
#[path = "tests/e2e_w17.rs"]
mod e2e_w17;
#[cfg(test)]
mod tests;
