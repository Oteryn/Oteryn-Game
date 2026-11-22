//! C20c: current-session read of the latest death's immutable protection consumption.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use oteryn_protocol_oteryn::death_protection::{
    CAPABILITY_COMBAT_OCCURRENCES_V1, CAPABILITY_DEATH_PROTECTION_V1, DeathProtection, DeathProtectionReply,
};
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || !matches!(
            entry.id, CAPABILITY_DEATH_PROTECTION_V1 | CAPABILITY_COMBAT_OCCURRENCES_V1
        ))
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
            _command_id: u64,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn death_protection_page(&self, admitted: AdmittedSession) -> Option<Vec<u8>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_DEATH_PROTECTION_V1)
        {
            return None;
        }
        let fence = query_fence(admitted)?;
        let death = self
            .root
            .read_latest_character_death(self.character, self.holder, fence)
            .await
            .ok()?;
        DeathProtectionReply(death.map(|(death, _)| {
            DeathProtection {
                occurrence: *death.occurrence.as_bytes(),
                consumed: death
                    .blessings_before
                    .into_iter()
                    .filter(|key| !death.blessings_after.contains(key))
                    .collect(),
            }
        }))
        .encode()
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
            _command_id: u64,
        ) -> Option<Vec<u8>> {
            self.death_protection_page(admitted).await
        }
    };
}
pub(super) use composed_methods;
