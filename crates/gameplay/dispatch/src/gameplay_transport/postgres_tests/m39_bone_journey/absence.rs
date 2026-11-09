//! The canonical writer must refuse loss while a real sample is carried.
use super::*;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use oteryn_gameplay_quests::quest::QuestRefusal;

pub(super) async fn reject_stale_advisory_loss(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    assert_eq!(stage(h, BONE).await?, Some(2));
    assert_eq!(carried(h, 131).await?, 1);
    let before = xp(h).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
        .with_wheel(owner.premium_coordinator);
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    // Deliberately bypass advisory selection, using the canonical loss callback.
    let outcome = slot
        .commit_quest_reward(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: callback("bone-pit-lost-sample")?.key,
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(1_000_000).map_err(debug)?,
                )),
            },
            owner.quest_catalogue.clone().ok_or("catalogue")?,
            crate::durability::reward_claim_mint::QuestRewardAdmission {
                creature_use: None,
                native,
                weights: &weights,
                fee_payment: None,
                actor_conditions: None,
                carried_use: None,
                peer_use: None,
            },
            &owner.premium_coordinator,
        )
        .await
        .map_err(debug)?;
    drop(slot);
    assert!(
        matches!(
            outcome,
            QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
        ),
        "canonical absence guard must reject the stale branch: {outcome:?}"
    );
    assert_eq!(stage(h, BONE).await?, Some(2));
    assert_eq!(carried(h, 131).await?, 1);
    assert_eq!(carried(h, 6299).await?, 0);
    assert_eq!(xp(h).await?, before);
    Ok(())
}
