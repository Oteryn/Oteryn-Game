//! Advisory selection cannot bypass the writer's absence predicate.
use super::*;

pub(super) async fn reject_stale_advisory_loss(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    muriel: EntityRef,
) -> TestResult {
    let mut advisory = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let candidate = advisory
        .iter_mut()
        .find(|c| c.key.ends_with("/bone-pit-lost-sample"))
        .ok_or("loss callback")?;
    // Simulate a stale selector, while leaving canonical durable terms intact.
    candidate.inventory.clear();
    candidate.keywords = vec!["lostproof".into()];
    let candidate = candidate.clone();
    owner
        .states
        .npc_conversations
        .fixture_reward_callbacks(advisory)?;
    let before = xp(h).await?;
    let result =
        super::super::qengine_rewards::wire_reward(owner, admitted, muriel, &candidate, false)
            .await?;
    assert_eq!(result.0, NpcIntentDisposition::Rejected);
    assert_eq!(stage(h, BONE).await?, Some(2));
    assert_eq!(carried(h, 131).await?, 1);
    assert_eq!(carried(h, 6299).await?, 0);
    assert_eq!(xp(h).await?, before);
    Ok(())
}
