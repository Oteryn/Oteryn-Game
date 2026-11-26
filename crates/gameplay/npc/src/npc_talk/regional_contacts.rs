//! Crystal focus text and unconditional keyword replies for the seven regional placement sets.
use crate::content::ProjectV2DialogueKeyword;
use serde::Deserialize;
use std::{collections::BTreeMap, sync::OnceLock};

pub(super) const DATA: &str = include_str!("../../../../../content/npcs/regional-dialogue.json");

// Crystal's pinned default messages in data/npclib/npc_system/npc_handler.lua:81–90.
pub(super) const GREET: &str = "Greetings, |PLAYERNAME|.";
pub(super) const FAREWELL: &str = "Good bye, |PLAYERNAME|.";
pub(super) const WALKAWAY: &str = "Good bye.";

#[derive(Deserialize)]
struct Catalogue {
    contacts: BTreeMap<String, Contact>,
}

#[derive(Deserialize)]
pub(super) struct Contact {
    pub greet: Option<String>,
    pub farewell: Option<String>,
    pub walkaway: Option<String>,
    #[serde(default)]
    pub greet_aliases: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<ProjectV2DialogueKeyword>,
}

pub(super) fn get(npc: &str) -> Option<&'static Contact> {
    static REGIONAL: OnceLock<Option<Catalogue>> = OnceLock::new();
    REGIONAL
        .get_or_init(|| serde_json::from_str(DATA).ok())
        .as_ref()?
        .contacts
        .get(npc)
}
