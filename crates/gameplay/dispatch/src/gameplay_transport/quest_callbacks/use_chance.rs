//! Crystal 201189f8 data-global/scripts/quests/the_ancient_tombs/actions_oasis_lever_door.lua:26-45.
use super::*;
use oteryn_gameplay_quests::quest::world_callbacks::UseChance;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

pub(in crate::gameplay_transport) fn wins(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    command: CommandRef,
    callback: &str,
    placement: u64,
    chance: &UseChance,
) -> Option<bool> {
    let occurrence = Sha256::new()
        .chain_update(actor.placement_identity())
        .chain_update(command.game_session_id().as_bytes())
        .chain_update(command.command_id().get().to_be_bytes())
        .chain_update(placement.to_be_bytes())
        .chain_update(callback.as_bytes())
        .finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&occurrence[..16]);
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    let draw = deterministic_decision_u64(
        &root,
        DecisionOccurrenceId::from_bytes(bytes),
        "quest.callback.use.chance",
        0,
    )
    .ok()?;
    let face = crate::spell::uniform_draw(draw, 1, i64::from(chance.faces));
    Some(face <= i64::from(chance.winning_faces))
}
