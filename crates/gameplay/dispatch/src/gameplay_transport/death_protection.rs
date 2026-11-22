//! C20c: current-session read of the latest death's immutable protection consumption.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use oteryn_protocol_oteryn::death_protection::{
    CAPABILITY_COMBAT_OCCURRENCES_V1,
    CAPABILITY_DEATH_PROTECTION_V1, CAPABILITY_DEATH_AND_BLESSINGS_V1, DeathProtection, DeathProtectionReply,
    DeathReport, DeathReportReply, DeathType,
};
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || !matches!(entry.id, CAPABILITY_DEATH_PROTECTION_V1 | CAPABILITY_DEATH_AND_BLESSINGS_V1 | CAPABILITY_COMBAT_OCCURRENCES_V1))
        .collect()
}
pub(super) fn query_fence(
    admitted: AdmittedSession,
) -> Option<crate::durability::item_transfer::CurrentCharacterItemFence> {
    let fence = admitted.item_fence?;
    (fence.game_session_id == admitted.game_session_id
        && fence.connection_generation.get() == admitted.continuity.connection_generation)
        .then_some(fence)
}

macro_rules! authority_methods {
    () => {
        fn death_protection_available(&self) -> bool {
            false
        }
        fn death_protection_query(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _report: bool,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn death_protection_page(&self, admitted: AdmittedSession, report: bool) -> Option<Vec<u8>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_DEATH_PROTECTION_V1)
            || (report && !admitted.continuity.selected_capabilities.contains(CAPABILITY_DEATH_AND_BLESSINGS_V1))
        {
            return None;
        }
        let fence = query_fence(admitted)?;
        let death = self
            .root
            .read_latest_character_death(self.character, self.holder, fence)
            .await
            .ok()?;
        let Some((death, pvp)) = death else {
            return Some(vec![0]);
        };
        let protection = DeathProtection {
            occurrence: *death.occurrence.as_bytes(),
            consumed: death
                .blessings_before
                .into_iter()
                .filter(|key| !death.blessings_after.contains(key))
                .collect(),
        };
        if report {
            DeathReportReply(Some(DeathReport {
                protection,
                death_type: match pvp {
                    Some(true) => DeathType::Pvp,
                    Some(false) => DeathType::Pve,
                    None => DeathType::Unknown,
                },
                experience_lost: u64::try_from(death.experience_lost.get()).ok()?,
            }))
            .encode()
        } else {
            DeathProtectionReply(Some(protection)).encode()
        }
    }
}
macro_rules! composed_methods {
    () => {
        fn death_protection_available(&self) -> bool {
            true
        }
        async fn death_protection_query(
            &self,
            admitted: connection::AdmittedSession,
            report: bool,
        ) -> Option<Vec<u8>> {
            self.death_protection_page(admitted, report).await
        }
    };
}
pub(super) use composed_methods;
