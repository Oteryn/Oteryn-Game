//! First-species points go through the existing Charm balance and Character revision lane.
use super::super::{ComposedFreshAdmission, operator_event};
use crate::combat::CapturedRewardPrincipal;
use crate::durability::bestiary_progress::BestiaryKillOccurrence;
use crate::durability::character_progression::CharacterProgressionError;
use crate::foundation::{ChannelRuntimeV1, RewardDamageShare};

#[derive(Debug, Clone)]
pub(in crate::gameplay_transport) struct EchoDeathReward {
    pub stars: u8,
    pub recipients: Vec<CapturedRewardPrincipal>,
}

pub(in crate::gameplay_transport) fn capture_recipients(
    runtime: &ChannelRuntimeV1,
    stars: u8,
    damage: &[RewardDamageShare],
) -> EchoDeathReward {
    // DamageLedger already attributes player summons to their Character master. Only
    // participants still bound at death are included; there is no damage-percentage gate.
    let recipients = runtime
        .positioned_actor_census()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(actor, _, session)| {
            let session = session?;
            let lease = runtime.bound_attacker_lease(actor, session).ok()?;
            damage
                .iter()
                .any(|share| {
                    share.character == lease.character_id()
                        && share.total > 0
                        && share.session == Some(session)
                        && share.lease_generation == Some(lease.generation())
                })
                .then_some(CapturedRewardPrincipal {
                    character: lease.character_id(),
                    lease_generation: lease.generation(),
                    session,
                    actor,
                })
        })
        .collect();
    EchoDeathReward { stars, recipients }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Returns false only for an unavailable owner, so the original kill is retried with
    /// its unchanged occurrence. Every successful recipient is independently replay-safe.
    pub(in crate::gameplay_transport) async fn credit_echo_warden_death(
        &self,
        entry: &super::super::kill_reward::PendingKillSettlement,
    ) -> bool {
        let Some(reward) = &entry.echo else {
            return true;
        };
        let Ok(occurrence) = BestiaryKillOccurrence::from_bytes(entry.facts.occurrence) else {
            operator_event("echo_warden_reward_invalid_occurrence");
            return true;
        };
        for recipient in &reward.recipients {
            let mut fence = match self.current_quest_fence(recipient.session).await {
                Ok(Some(fence)) => fence,
                Ok(None) => continue,
                Err(()) => return false,
            };
            if fence.character_id.as_bytes() != recipient.character.as_bytes()
                || fence.character_lease_generation != recipient.lease_generation
            {
                continue;
            }
            let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
            let Ok(revision) = slot.cursor(self.root, self.character).await else {
                return false;
            };
            fence.expected_character_revision = revision;
            match self
                .root
                .credit_echo_warden_kill(
                    self.character,
                    self.holder,
                    fence,
                    occurrence,
                    &entry.creature,
                    reward.stars,
                )
                .await
            {
                Ok(_) => {}
                Err(CharacterProgressionError::Unavailable(_)) => return false,
                Err(error) => operator_event(&format!(
                    "echo_warden_reward_refused creature={} reason={error:?}",
                    entry.creature,
                )),
            }
        }
        true
    }
}
