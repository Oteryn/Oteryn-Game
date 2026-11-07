//! Local player chat rules (CHAT-0 §3, §4 and §6, `CHAT0-LOCAL-PRIVATE-WORLD-ROOMS-V1`).
//!
//! Pure rules of CHAT-1: the text bounds, who hears a `say`, `whisper` or `yell`, the yell gates
//! and cooldown, the per-character spam bucket with its in-memory mute, and the NPC greeting match.
//! The wire (capability `CHAT_V1`, `CHAT_INTENT`, domain `CHAT`) and the runtime that feeds these
//! rules live in `gameplay_transport::chat_intent` (CHAT-WIRE-1); the World relay, private
//! messages, rooms and the durable mute row are CHAT-2. Chat is Channel-local here and never
//! durable.

pub mod egress;
pub mod greeting;
pub mod guild_room;
pub mod line;
pub mod relay;
pub mod spam;
#[cfg(test)]
mod tests;

#[allow(unused_imports)] // Preserve the existing text grammar paths.
pub use crate::durability::chat_text::{ChatText, TEXT_MAX_BYTES, TEXT_MAX_CHARS, TextError};
pub use egress::{ChatEgress, EGRESS_MAX_LINES};
pub use greeting::{GreetingNpc, greeted_npc};
pub use line::{local_line, local_listeners, private_line, room_line};
pub use spam::{ChatLimiter, ChatRefusal, YellGate};

pub use crate::foundation::speech::{ChatPosition, Heard, SpeechMode, WHISPER_OBSCURED, hears};
