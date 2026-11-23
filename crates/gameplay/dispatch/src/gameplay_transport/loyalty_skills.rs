//! P13 read-only account loyalty and trained-skill display.
use super::{AdmissionRetryBudget, connection::AdmissionRefusal};
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::durability::character_build::{CombatLoyaltyProjection, DurableBuildState};
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{ChannelId, ExactActorRef, GameSessionId};
use crate::foundation::{CommandId, CommandRef};
use crate::spell::{cast::PlayerSpellState, mana_training::CompiledTrainingFormula};
use oteryn_protocol_oteryn::loyalty_skills::{
    CAPABILITY_LOYALTY_SKILLS_V1, LoyaltySkills, SkillLevel,
};

/// Preserve bundle membership and withhold the read when the training formula is absent.
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != CAPABILITY_LOYALTY_SKILLS_V1)
        .collect()
}

macro_rules! authority_methods {
    () => {
        fn loyalty_skills_available(&self) -> bool {
            false
        }
        fn loyalty_skills_query(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _command_id: u64,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;

pub(super) struct AdmissionCombatBuild<'a> {
    pub(super) build: DurableBuildState,
    training: Option<(&'a CompiledTrainingFormula, CombatLoyaltyProjection)>,
}

impl AdmissionCombatBuild<'_> {
    pub(super) fn install(&self, state: &mut PlayerSpellState) -> Option<()> {
        state.load_owned_build(&self.build).ok()?;
        if let Some((formula, loyalty)) = self.training {
            state.enable_owned_training(&self.build, formula).ok()?;
            state.set_combat_loyalty(loyalty, formula);
        }
        Some(())
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn load_admission_combat_build(
        &self,
        budget: &AdmissionRetryBudget<'_>,
        fence: CurrentCharacterItemFence,
        house_origin: Option<ChannelId>,
    ) -> Result<AdmissionCombatBuild<'_>, AdmissionRefusal> {
        let Some((formula, digest)) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .and_then(|native| Some((native.training_formula()?, native.source_digest())))
        else {
            let build = self
                .root
                .read_character_build_state(self.character, fence.character_id)
                .await
                .map_err(|_| AdmissionRefusal::Unavailable)?;
            return Ok(AdmissionCombatBuild {
                build,
                training: None,
            });
        };
        let (build, loyalty) = budget
            .retry_unavailable(async || self.read_combat_loyalty(fence, digest, house_origin).await)
            .await?;
        Ok(AdmissionCombatBuild {
            build,
            training: Some((formula, loyalty)),
        })
    }

    async fn read_combat_loyalty(
        &self,
        fence: CurrentCharacterItemFence,
        digest: [u8; 32],
        house_origin: Option<ChannelId>,
    ) -> Result<(DurableBuildState, CombatLoyaltyProjection), AdmissionRefusal> {
        self.root
            .read_combat_loyalty_build(self.character, self.holder, &fence, digest, house_origin)
            .await
            .map_err(|error| match error {
                CharacterProgressionError::Unavailable(_) => AdmissionRefusal::Unavailable,
                _ => AdmissionRefusal::Rejected,
            })
    }

    /// Refresh under the same lane as combat and custody transfers. The owner clock
    /// retries an unavailable read on its next pass, and reconnect has its own budget.
    pub(super) async fn refresh_combat_loyalty(
        &self,
        _permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        fence: Option<CurrentCharacterItemFence>,
    ) -> Result<(), AdmissionRefusal> {
        use AdmissionRefusal::{Rejected, Unavailable};
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return Ok(());
        };
        let Some(formula) = native.training_formula() else {
            return Ok(());
        };
        let digest = native.source_digest();
        let house_origin = {
            let runtime = self.runtime.lock().await;
            if runtime.content_pin().server_artifact_digest() != digest {
                return Err(Rejected);
            }
            if runtime
                .house_player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            {
                Some(self.channel_id)
            } else {
                let facts = runtime
                    .player_control_facts(actor, session)
                    .map_err(|_| Rejected)?;
                if facts.control_loss.is_some() {
                    return Err(Rejected);
                }
                None
            }
        };
        let fence = match fence {
            Some(fence) => fence,
            None => self
                .current_quest_fence(session)
                .await
                .map_err(|()| Unavailable)?
                .ok_or(Rejected)?
                .item_fence(),
        };
        if fence.game_session_id != session {
            return Err(Rejected);
        }
        let (_, loyalty) = self
            .read_combat_loyalty(fence, digest, house_origin)
            .await?;
        let runtime = self.runtime.lock().await;
        if runtime.content_pin().server_artifact_digest() != digest {
            return Err(Rejected);
        }
        let mut states = self.states.spell_states.lock().await;
        if states.has_pending_spell_commit(actor, session) {
            return Err(Rejected);
        }
        let state = if house_origin.is_some() {
            runtime
                .assert_house_actor_spell_unreserved(actor, session)
                .map_err(|_| Rejected)?;
            if runtime
                .house_player_writes_fenced(actor, session)
                .map_err(|_| Rejected)?
            {
                return Err(Rejected);
            }
            states.house_state_mut(&runtime, actor, session, fence)
        } else {
            let binding = runtime.binding();
            let lease = runtime
                .bound_attacker_lease(actor, session)
                .map_err(|_| Rejected)?;
            if binding.runtime_scope() != fence.runtime_scope
                || binding.scope_generation() != fence.scope_ownership_generation
                || lease.character_id().as_bytes() != fence.character_id.as_bytes()
                || lease.generation() != fence.character_lease_generation
                || runtime
                    .player_control_facts(actor, session)
                    .map_err(|_| Rejected)?
                    .control_loss
                    .is_some()
            {
                return Err(Rejected);
            }
            states.get_mut(&runtime, actor, session)
        }
        .ok_or(Rejected)?;
        state.set_combat_loyalty(loyalty, formula);
        Ok(())
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn loyalty_skills_page(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
    ) -> Option<Vec<u8>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_LOYALTY_SKILLS_V1)
        {
            return None;
        }
        let fence = admitted.item_fence?;
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return None;
        }
        let native = self.active_generation?.native_gameplay()?;
        let formula = native.training_formula()?;
        let command = CommandRef::new(admitted.game_session_id, CommandId::new(command_id).ok()?);
        // Extend the existing session/content-fenced Character build owner. Its one transaction
        // resolves the account from the Character, reads the Platform projection and
        // returns the trained build; this query never awards points or trains a skill.
        let (build, loyalty_points) = self
            .root
            .read_loyalty_build_state(
                self.character,
                self.holder,
                &fence,
                command,
                native.source_digest(),
            )
            .await
            .ok()?;
        let skills = std::array::from_fn(|family| {
            let (level, progress) = if family == 0 {
                build.magic()
            } else {
                build.skills()[family - 1]
            };
            SkillLevel {
                base: level,
                with_loyalty: oteryn_gameplay_progression::loyalty::projected_level(
                    formula,
                    build.vocation(),
                    family,
                    level,
                    progress,
                    loyalty_points,
                ),
            }
        });
        LoyaltySkills {
            points: loyalty_points,
            bonus_percent: oteryn_gameplay_progression::loyalty::bonus_percent(loyalty_points),
            title: oteryn_gameplay_progression::loyalty::title(loyalty_points).map(str::to_owned),
            skills,
        }
        .encode()
    }
}

macro_rules! composed_methods {
    () => {
        fn loyalty_skills_available(&self) -> bool {
            self.active_generation
                .and_then(|active| active.native_gameplay())
                .and_then(|native| native.training_formula())
                .is_some()
        }
        async fn loyalty_skills_query(
            &self,
            admitted: connection::AdmittedSession,
            command_id: u64,
        ) -> Option<Vec<u8>> {
            self.loyalty_skills_page(admitted, command_id).await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loyalty_skills_offer_requires_the_native_formula_and_preserves_bundle_membership() {
        let base = [
            OfferedCapability {
                id: 168,
                requires: &[],
            },
            OfferedCapability {
                id: CAPABILITY_LOYALTY_SKILLS_V1,
                requires: &[],
            },
        ];
        assert_eq!(offered(&base, false).len(), 1);
        assert_eq!(offered(&base, true).len(), 2);
        assert!(offered(&[], true).is_empty());
    }
}
