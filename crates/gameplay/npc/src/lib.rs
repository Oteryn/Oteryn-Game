//! L4 runtime: NPC walking, voice, dialogues, travel, bank/trade conversations and vocation choice.

pub mod bank_npc;
pub mod greeting;
pub mod npc_actor;
pub mod npc_command;
pub mod npc_talk;
pub mod npc_travel;
pub mod npc_voice;
pub mod vocation_choice;

pub(crate) use oteryn_gameplay_content::content;
pub(crate) use oteryn_gameplay_domain::domain;
pub(crate) use oteryn_gameplay_foundation::foundation;
pub(crate) use oteryn_gameplay_world::{map, movement};
