//! Bounded own-actor item exhaustion read; existing use commands retain all write authority.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use crate::content::{ReferenceItemConsumption, ReferenceItemField};
use oteryn_protocol_oteryn::action_bar_cooldowns::{self as wire, ItemCooldown};

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|entry| available || entry.id != wire::CAPABILITY_ACTION_BAR_COOLDOWNS_V1)
        .collect()
}
macro_rules! authority_methods {
    () => {
        fn action_bar_cooldowns_available(&self) -> bool {
            false
        }
        fn action_bar_cooldowns_query(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _command_id: u64,
            _definitions: Vec<u32>,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn action_bar_cooldowns_page(
        &self,
        admitted: AdmittedSession,
        definitions: &[u32],
    ) -> Option<Vec<u8>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_ACTION_BAR_COOLDOWNS_V1)
        {
            return None;
        }
        let fence = admitted.item_fence?;
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return None;
        }
        let actor = admitted.runtime_actor?;
        let native = self.active_generation?.native_gameplay()?;
        let runtime = self.runtime.lock().await;
        let lease = runtime
            .bound_attacker_lease(actor, admitted.game_session_id)
            .ok()?;
        if lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || lease.generation() != fence.character_lease_generation
        {
            return None;
        }
        let states = self.states.spell_states.try_lock().ok()?;
        if states.is_dead(actor) {
            return None;
        }
        let mut items = Vec::with_capacity(definitions.len());
        for id in definitions {
            let definition = super::item_look::typed_definition(
                native.item_index()?,
                std::num::NonZeroU32::new(*id)?,
            )?;
            let record = native
                .item_policy(&definition.production_key, &definition.revision_ref)?
                .record();
            let class = match &record.semantics.consumption {
                ReferenceItemField::Known(ReferenceItemConsumption::Food(_)) => {
                    super::item_use::CooldownClass::Food
                }
                ReferenceItemField::Known(ReferenceItemConsumption::Potion(_)) => {
                    super::item_use::CooldownClass::Potion
                }
                _ if oteryn_gameplay_progression::condition_potions::PotionCondition::from_key(
                    &definition.production_key,
                )
                .is_some() =>
                {
                    super::item_use::CooldownClass::Potion
                }
                _ => super::item_use::CooldownClass::Other,
            };
            let remaining_ms = states.item_uses.cooldown_remaining(
                &runtime,
                actor,
                admitted.game_session_id,
                class,
                self.owner_ms(),
            )?;
            items.push(ItemCooldown {
                definition: *id,
                remaining_ms,
            });
        }
        wire::encode_reply(&items)
    }
}
macro_rules! composed_methods {
    () => {
        fn action_bar_cooldowns_available(&self) -> bool {
            self.active_generation
                .and_then(|active| active.native_gameplay())
                .is_some_and(|native| native.item_index().is_some())
        }
        async fn action_bar_cooldowns_query(
            &self,
            admitted: connection::AdmittedSession,
            _command_id: u64,
            definitions: Vec<u32>,
        ) -> Option<Vec<u8>> {
            self.action_bar_cooldowns_page(admitted, &definitions).await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn item_cooldown_offer_requires_its_existing_native_source() {
        let base = [OfferedCapability {
            id: 463,
            requires: &[15, 287],
        }];
        assert!(offered(&base, false).is_empty());
        assert_eq!(offered(&base, true).len(), 1);
    }
}
