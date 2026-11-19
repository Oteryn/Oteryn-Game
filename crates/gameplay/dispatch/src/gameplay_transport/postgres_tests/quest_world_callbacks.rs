//! QUESTCB2: literal commands exercise the real callback, map, quest and item owners.
use super::quest_world_callbacks_fixtures::{callback, scenario, world};

use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[path = "quest_world_callbacks/actions.rs"]
mod actions;

#[path = "quest_world_callbacks/assertions.rs"]
mod assertions;

#[path = "quest_world_callbacks/lions_rock.rs"]
mod lions_rock;

#[path = "quest_world_callbacks/literal_uses.rs"]
mod literal_uses;

#[path = "quest_world_callbacks/rotten_blood.rs"]
mod rotten_blood;

#[path = "quest_world_callbacks/unnatural_selection.rs"]
mod unnatural_selection;
