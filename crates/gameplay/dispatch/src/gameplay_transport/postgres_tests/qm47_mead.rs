//! M47 drives the fenced world writer, preserving the command receipt on replay.
use super::TestResult;
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::scenario;
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::quest_world_callback::QuestWorldContext;
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
