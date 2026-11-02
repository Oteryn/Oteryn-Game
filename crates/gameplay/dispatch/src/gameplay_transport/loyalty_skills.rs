//! P13 read-only account loyalty and trained-skill display.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::foundation::{CommandId, CommandRef};
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
