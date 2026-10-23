//! Composition of current Premium with the existing durable death settlement.
use super::*;
use crate::durability::character_death::promotion::{DeathPremiumOwner, DeathPremiumState};

struct PremiumDeathGate<'a, 'r, 'c, 'n> {
    admission: &'a ComposedFreshAdmission<'r, 'c, 'n>,
    session: GameSessionId,
}
impl DeathPremiumOwner for PremiumDeathGate<'_, '_, '_, '_> {
    fn current(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = DeathPremiumState> + Send + '_>> {
        Box::pin(async move {
            let account = self
                .admission
                .states
                .premium_sessions
                .lock()
                .ok()
                .and_then(|sessions| sessions.account_of(self.session));
            let (Some(account), Some(coordinator)) = (account, self.admission.premium_coordinator)
            else {
                return DeathPremiumState::default();
            };
            coordinator
                .with_premium_gate(account, |status, _| {
                    Box::pin(async move {
                        DeathPremiumState {
                            current: status.gate().premium_current(),
                            before_delivery: matches!(
                                status,
                                crate::domain::premium::PremiumStatus::NotActivated(_)
                            ),
                        }
                    })
                })
                .await
                .unwrap_or_default()
        })
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The durable half of [`Self::settle_death`]. The death request carries the binding's
    /// policy, the whole `CHARACTER_EXPERIENCE_TABLE_LEVELS` threshold table, by value across
    /// the commit, so `settle_death` keeps this future on the heap: inline, it would grow every
    /// step future that settles a death by several copies of that table.
    pub(super) async fn commit_death(
        &self,
        session: GameSessionId,
        progression: &SessionProgressionBinding,
        death: actor_spell::PlayerDeath,
    ) -> (
        DeathCommit,
        Option<crate::durability::character_build::DurableBuildState>,
    ) {
        let (map_revision, respawn) = {
            let runtime = self.runtime.lock().await;
            (runtime.map_revision_digest(), runtime.respawn_position())
        };
        let request = player_death_request(
            progression,
            (self.world_id, self.channel_id),
            map_revision,
            death,
            respawn,
        );
        let Ok(Some(fence)) = self.current_quest_fence(session).await else {
            return (DeathCommit::Unavailable, None);
        };
        // CP D1425: the active generation's training formula prices the death's build loss.
        let formula = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .and_then(|native| native.training_formula())
            .map(|formula| formula as &dyn crate::durability::character_build::BuildFormula);
        match self
            .root
            .settle_runtime_player_death(
                &self.revision_sequencer,
                self.character,
                self.holder,
                fence,
                request,
                formula,
                Some(&PremiumDeathGate {
                    admission: self,
                    session,
                }),
            )
            .await
        {
            Ok(committed) => (
                DeathCommit::Settled,
                committed.build_loss.map(|loss| loss.after),
            ),
            Err(error) => {
                operator_event(&format!("player_death_settle_failed reason={error}"));
                (death_commit_refusal(&error), None)
            }
        }
    }
}
