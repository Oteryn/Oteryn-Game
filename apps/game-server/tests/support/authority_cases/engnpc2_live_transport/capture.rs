//! A capture consumer retains the exact actor validated by the live proof owner.
use super::{TestResult, debug};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef};
use oteryn_gameplay_dispatch::gameplay_transport::composed_test_owner::UseCommand;
use oteryn_gameplay_durability::durability::{
    character_progression::CurrentCharacterGameplayFence,
    reward_claim_mint::quest_creature::QuestCreatureUseProof,
};
use oteryn_protocol_oteryn::world_object::UseIntent;
use tokio::sync::Mutex;

pub(super) async fn assert_target(
    runtime: &Mutex<ChannelRuntimeV1>,
    actor: ExactActorRef,
    target: Option<ExactActorRef>,
    fence: CurrentCharacterGameplayFence,
    command: UseCommand,
    intent: &UseIntent,
) -> TestResult {
    let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()
        .map_err(debug)?
        .into_iter()
        .find(|c| c.carried_item.as_deref() == Some("oteryn:item.tibia.i17673"))
        .ok_or("live sampler callback")?;
    let command = CommandRef::new(
        command.game_session_id,
        CommandId::new(command.command_id).map_err(debug)?,
    );
    let runtime = runtime.lock().await;
    let proof =
        QuestCreatureUseProof::read_current(&runtime, actor, fence, command, &callback, intent);
    assert_eq!(proof.as_ref().and_then(|p| p.live_target()), target);
    if target.is_some() {
        assert_eq!(
            target.map(|t| t.placement_identity()),
            intent.use_with.map(|t| t.identity)
        );
    } else {
        assert!(
            proof.is_none(),
            "a retired creature cannot issue a capture proof"
        );
    }
    Ok(())
}
