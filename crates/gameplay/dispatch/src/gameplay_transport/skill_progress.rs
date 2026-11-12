//! C14 authoritative trained progress projection.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::skill_progress::{
    CAPABILITY_SKILL_PROGRESS_V1, SkillProgress, SkillProgressSnapshot,
};

/// Preserve bundle membership and withhold the read when the training formula is absent.
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != CAPABILITY_SKILL_PROGRESS_V1)
        .collect()
}

macro_rules! authority_methods {
    () => {
        fn skill_progress_available(&self) -> bool {
            false
        }
        fn skill_progress_query(
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
    pub(super) async fn skill_progress_page(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
    ) -> Option<Vec<u8>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_SKILL_PROGRESS_V1)
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
        let (build, _) = self
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
            project_skill(formula, build.vocation(), family, level, progress)
        });
        SkillProgressSnapshot { skills }.encode()
    }
}

/// Crystal player.cpp:3799-3809 rounds to the nearest hundredth of a percent.
/// The integer numerator also handles every bounded durable progress value without overflow.
pub(super) fn project_skill(
    formula: &impl oteryn_gameplay_domain::domain::character_build::BuildFormula,
    vocation: &str,
    family: usize,
    level: u16,
    progress: u64,
) -> SkillProgress {
    let required = level
        .checked_add(1)
        .and_then(|next| formula.required(vocation, family, next))
        .unwrap_or(0);
    SkillProgress {
        level,
        progress,
        required,
        percent: percent(progress, required),
    }
}

pub(super) fn percent(progress: u64, required: u64) -> u16 {
    let rounded = if required == 0 {
        0
    } else {
        (u128::from(progress) * 10_000 + u128::from(required) / 2) / u128::from(required)
    };
    u16::try_from(rounded)
        .ok()
        .filter(|value| *value <= 10_000)
        .unwrap_or(0)
}

macro_rules! composed_methods {
    () => {
        fn skill_progress_available(&self) -> bool {
            self.active_generation
                .and_then(|active| active.native_gameplay())
                .and_then(|native| native.training_formula())
                .is_some()
        }
        async fn skill_progress_query(
            &self,
            admitted: connection::AdmittedSession,
            command_id: u64,
        ) -> Option<Vec<u8>> {
            self.skill_progress_page(admitted, command_id).await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_progress_offer_requires_the_native_formula_and_preserves_bundle_membership() {
        let base = [
            OfferedCapability {
                id: 168,
                requires: &[],
            },
            OfferedCapability {
                id: CAPABILITY_SKILL_PROGRESS_V1,
                requires: &[],
            },
        ];
        assert_eq!(offered(&base, false).len(), 1);
        assert_eq!(offered(&base, true).len(), 2);
        assert!(offered(&[], true).is_empty());
    }
    struct Formula;
    impl oteryn_gameplay_domain::domain::character_build::BuildFormula for Formula {
        fn required(&self, _vocation: &str, family: usize, level: u16) -> Option<u64> {
            (level <= 1000).then_some(if family == 0 { 20_000 } else { 8 })
        }
        fn digest(&self) -> [u8; 32] {
            [0; 32]
        }
        fn content_revision(&self) -> &str {
            "skill-progress-test"
        }
    }

    #[test]
    fn skill_progress_matches_donor_rounding_unreachable_and_overflow_edges() {
        assert_eq!(project_skill(&Formula, "paladin", 0, 30, 1).percent, 1);
        assert_eq!(project_skill(&Formula, "paladin", 3, 100, 5).percent, 6250);
        assert_eq!(project_skill(&Formula, "paladin", 3, 100, 8).percent, 10000);
        assert_eq!(project_skill(&Formula, "paladin", 3, 100, 9).percent, 0);
        assert_eq!(
            project_skill(&Formula, "paladin", 3, 100, i64::MAX.unsigned_abs()).percent,
            0
        );
        let capped = project_skill(&Formula, "paladin", 0, 1000, 123);
        assert_eq!((capped.required, capped.percent), (0, 0));
    }
}
