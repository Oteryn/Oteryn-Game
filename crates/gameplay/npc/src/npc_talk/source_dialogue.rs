//! Focus messages and literal replies from Crystal 201189f8.
//! Keys name the script in data-global/npc. Native owners still evaluate
//! conditional greetings, quest topics, shop actions and travel confirmations.

use crate::content::ProjectV2DialogueKeyword;
use serde::Deserialize;
use std::{collections::BTreeMap, sync::OnceLock};

pub(super) const SOURCE: &str = include_str!("source_dialogue.json");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Dialogue {
    pub(super) greet: Option<Vec<String>>,
    pub(super) farewell: Option<String>,
    pub(super) walkaway: Option<String>,
    pub(super) greetings: Option<Vec<String>>,
    pub(super) farewells: Option<Vec<String>>,
    pub(super) focus_greet: Option<FocusReply>,
    pub(super) focus_farewell: Option<FocusReply>,
    #[serde(default)]
    pub(super) keywords: Vec<ProjectV2DialogueKeyword>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct FocusReply {
    pub(super) triggers: Vec<String>,
    pub(super) reply: String,
    #[serde(default)]
    pub(super) female_only: bool,
}

pub(super) fn catalogue() -> &'static BTreeMap<String, Dialogue> {
    static CATALOGUE: OnceLock<BTreeMap<String, Dialogue>> = OnceLock::new();
    CATALOGUE.get_or_init(|| serde_json::from_str(SOURCE).unwrap_or_default())
}
