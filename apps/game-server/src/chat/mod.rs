//! Local player chat rules (CHAT-0 §3, §4 and §6, `CHAT0-LOCAL-PRIVATE-WORLD-ROOMS-V1`).
//!
//! Pure rules of CHAT-1: the text bounds, who hears a `say`, `whisper` or `yell`, the yell gates
//! and cooldown, the per-character spam bucket with its in-memory mute, and the NPC greeting match.
//! The wire (capability `CHAT_V1`, `CHAT_INTENT`, domain `CHAT`) and the runtime that feeds these
//! rules live in `gameplay_transport::chat_intent` (CHAT-WIRE-1); the World relay, private
//! messages, rooms and the durable mute row are CHAT-2. Chat is Channel-local here and never
//! durable.

pub(crate) mod egress;
pub(crate) mod greeting;
pub(crate) mod line;
pub(crate) mod relay;
pub(crate) mod spam;
#[cfg(test)]
mod tests;

#[allow(unused_imports)] // Preserve the existing text grammar paths.
pub(crate) use crate::durability::chat_text::{
    ChatText, TEXT_MAX_BYTES, TEXT_MAX_CHARS, TextError,
};
pub(crate) use egress::{ChatEgress, EGRESS_MAX_LINES};
pub(crate) use greeting::{GreetingNpc, greeted_npc};
pub(crate) use line::{local_line, local_listeners, private_line, room_line};
pub(crate) use spam::{ChatLimiter, ChatRefusal, YellGate};

pub(crate) use crate::foundation::speech::{
    ChatPosition, Heard, SpeechMode, WHISPER_OBSCURED, hears,
};
