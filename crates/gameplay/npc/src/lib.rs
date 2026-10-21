//! L4 runtime: NPC walking, voice, dialogues, travel, bank/trade conversations and vocation choice.

pub mod bank_npc;
pub mod greeting;
pub mod npc_actor;
pub mod npc_command;
pub mod npc_talk;
pub mod npc_travel;
pub mod npc_voice;
pub mod vocation_choice;

use oteryn_gameplay_content::content;
use oteryn_gameplay_domain::domain;
use oteryn_gameplay_entities as foundation;
use oteryn_gameplay_world::{map, movement};
