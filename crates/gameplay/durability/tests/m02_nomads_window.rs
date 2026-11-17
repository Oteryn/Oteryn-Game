//! Focused M02 qualification runs the existing production window observer case.
#![cfg(feature = "test-support")]
use durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
};
use durability::quest_world_callback::QuestWorldContext;
use durability::spell_owner_commit::SpellLanePermit;
use foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};
use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
use oteryn_gameplay_domain::domain;
use oteryn_gameplay_durability::durability;
use oteryn_gameplay_entities as foundation;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[allow(
    dead_code,
    reason = "shared native fixture; only world observation is exercised"
)]
#[path = "../../../../apps/game-server/tests/support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
use bestiary_postgres_harness as h;
#[path = "../src/durability/quest_world_callback/ground_stack/tests/fixture.rs"]
mod fixture;
use fixture::{context, mint_at};
#[path = "../src/durability/quest_world_callback/ground_stack/tests/stateless.rs"]
mod world_observer;

fn spatial(position: Position) -> Vec<u8> {
    [
        i32::from(position.x).to_be_bytes().as_slice(),
        i32::from(position.y).to_be_bytes().as_slice(),
        i16::from(position.floor).to_be_bytes().as_slice(),
    ]
    .concat()
}
