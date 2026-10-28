//! Native Prey command and admission boundary. No client supplies payment or clock facts.
use super::{
    ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession,
    guild_intent::GuildReply,
};
use crate::durability::character_progression::CharacterProgressionError as Error;
use crate::durability::prey::{PreyCatalogue, PreyMutation};
use crate::durability::{DurabilityError, character_wheel::WheelPremiumGate};
use oteryn_protocol_oteryn::prey as wire;

#[path = "prey/live.rs"]
mod live;
pub(super) use live::LivePrey;

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    let mut offers: Vec<_> = base
        .iter()
        .copied()
        .filter(|c| c.id != wire::CAPABILITY_PREY_V1)
        .collect();
    if available {
        let at = offers.partition_point(|c| c.id < wire::CAPABILITY_PREY_V1);
        offers.insert(
            at,
            OfferedCapability {
                id: wire::CAPABILITY_PREY_V1,
                requires: &[],
            },
        );
    }
    offers
}
struct Premium<'a>(Option<&'a super::spell_premium_coordinator::SpellPremiumCoordinator>);
impl WheelPremiumGate for Premium<'_> {
    async fn allocation_permitted(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        account: [u8; 16],
    ) -> Result<bool, DurabilityError> {
        match self.0 {
            Some(gate) => {
                gate.transaction_permission(tx, account, |s| s.gate().premium_current())
                    .await
            }
            None => Ok(false),
        }
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn prey_catalogue(&self) -> Option<PreyCatalogue> {
        PreyCatalogue::try_from(self.active_generation?.native_gameplay()?).ok()
    }
    pub(super) async fn prey_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        payload: &[u8],
    ) -> GuildReply {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_PREY_V1)
        {
            return GuildReply::Rejected;
        }
        let Ok(request) = wire::decode_request(payload) else {
            return GuildReply::Rejected;
        };
        if self.drain_kill_rewards(admitted.game_session_id).await
            != super::kill_reward::DrainOutcome::Drained
        {
            return GuildReply::Unavailable;
        }
        let Some(permit) = self.spell_lane_permit().await else {
            return GuildReply::Unavailable;
        };
        if !self
            .flush_prey_under_lane(admitted.game_session_id, &permit)
            .await
        {
            return GuildReply::Unavailable;
        }
        let occurrence =
            super::npc_trade_command::derived(b"prey:command:v1", admitted.game_session_id, id);
        match self
            .prey_commit(admitted, occurrence, PreyMutation::Command(request))
            .await
        {
            Ok((reply, _)) => {
                let current = match self
                    .prey_commit(
                        admitted,
                        occurrence,
                        PreyMutation::Command(wire::Request::Query),
                    )
                    .await
                {
                    Ok(current) => current,
                    Err(_) => {
                        self.states
                            .spell_states
                            .lock()
                            .await
                            .clear_prey(admitted.game_session_id);
                        return GuildReply::Unavailable;
                    }
                };
                if let Some(actor) = admitted.runtime_actor {
                    let runtime = self.runtime.lock().await;
                    self.states.spell_states.lock().await.load_prey(
                        &runtime,
                        actor,
                        admitted.game_session_id,
                        current.0,
                        current.1,
                    );
                }
                match wire::encode_reply(&reply) {
                    Ok(bytes) => GuildReply::Payload(bytes),
                    Err(_) => GuildReply::Unavailable,
                }
            }
            Err(Error::Unavailable(_)) => GuildReply::Unavailable,
            Err(_) => GuildReply::Rejected,
        }
    }
    async fn prey_commit(
        &self,
        admitted: AdmittedSession,
        occurrence: [u8; 16],
        mutation: PreyMutation,
    ) -> Result<(wire::Reply, [bool; 3]), Error> {
        let catalogue = self.prey_catalogue().ok_or(Error::InvalidInput)?;
        let expected = admitted.item_fence.ok_or(Error::AuthorityRejected)?;
        let mut slot = self.revision_sequencer.acquire(expected.character_id).await;
        let fence = super::character_gameplay_fence::current(self, admitted.game_session_id, true)
            .await
            .ok_or(Error::AuthorityRejected)?;
        if fence.connection_generation != expected.connection_generation
            || fence.character_lease_generation != expected.character_lease_generation
            || fence.scope_ownership_generation != expected.scope_ownership_generation
            || fence.runtime_scope != expected.runtime_scope
            || fence.character_id != expected.character_id
        {
            return Err(Error::AuthorityRejected);
        }
        let coins = {
            let runtime = self.runtime.lock().await;
            super::charm_progression::fee_change(
                self,
                &runtime,
                None,
                [
                    super::npc_trade_command::derived(
                        b"prey:change:gold",
                        admitted.game_session_id,
                        u64::from_be_bytes(
                            occurrence[..8]
                                .try_into()
                                .map_err(|_| Error::InvalidInput)?,
                        ),
                    ),
                    super::npc_trade_command::derived(
                        b"prey:change:platinum",
                        admitted.game_session_id,
                        u64::from_be_bytes(
                            occurrence[..8]
                                .try_into()
                                .map_err(|_| Error::InvalidInput)?,
                        ),
                    ),
                ],
            )
        };
        let result = self
            .root
            .commit_prey_command(
                self.character,
                self.holder,
                fence,
                occurrence,
                mutation,
                &catalogue,
                coins,
                &Premium(self.premium_coordinator),
            )
            .await;
        slot.observe_loop_training();
        #[cfg(test)]
        if let Err(e) = &result {
            eprintln!("PREY command: {e:?}");
        }
        result
    }
    pub(super) async fn admit_prey(&self, admitted: AdmittedSession) -> bool {
        if self.prey_catalogue().is_none() {
            return true;
        }
        // Each admission may initialize newly unlocked slots. Existing typed rows are never
        // redrawn; their unique Character+slot key serializes retry and concurrent admission.
        let Some(mut occurrence) = super::SecureIdentifiers::random::<16>() else {
            return false;
        };
        occurrence[6] = (occurrence[6] & 15) | 0x70;
        occurrence[8] = (occurrence[8] & 63) | 0x80;
        if self
            .prey_commit(admitted, occurrence, PreyMutation::Initialize)
            .await
            .is_err()
        {
            return false;
        }
        // A receipt replay returns its original bytes; admission needs the current projection.
        let Ok((reply, permanent)) = self
            .prey_commit(
                admitted,
                occurrence,
                PreyMutation::Command(wire::Request::Query),
            )
            .await
        else {
            return false;
        };
        if let Some(actor) = admitted.runtime_actor {
            let runtime = self.runtime.lock().await;
            self.states.spell_states.lock().await.load_prey(
                &runtime,
                actor,
                admitted.game_session_id,
                reply,
                permanent,
            );
        }
        true
    }
}
macro_rules! authority_methods {
    () => {
        fn prey_available(&self) -> bool {
            false
        }
        fn prey_command(
            &self,
            _: $crate::gameplay_transport::connection::AdmittedSession,
            _: u64,
            _: &[u8],
        ) -> impl std::future::Future<Output = $crate::gameplay_transport::guild_intent::GuildReply>
        {
            async { $crate::gameplay_transport::guild_intent::GuildReply::Rejected }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn prey_available(&self) -> bool {
            self.prey_catalogue().is_some()
        }
        async fn prey_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            payload: &[u8],
        ) -> guild_intent::GuildReply {
            self.prey_wire(admitted, id, payload).await
        }
    };
}
pub(super) use composed_methods;

/// PREY-0 §7: the typed positive damage contribution rounds up.
pub(super) fn damage(amount: i64, percent: u8) -> i64 {
    amount.saturating_add(amount.saturating_mul(i64::from(percent)).saturating_add(99) / 100)
}
#[cfg(test)]
mod tests {
    #[test]
    fn prey_damage_boost_preserves_integer_rounding() {
        assert_eq!(super::damage(100, 25), 125);
        assert_eq!(super::damage(5, 25), 7);
        assert_eq!(super::damage(3, 25), 4);
        assert_eq!(super::damage(100, 0), 100);
    }
}
