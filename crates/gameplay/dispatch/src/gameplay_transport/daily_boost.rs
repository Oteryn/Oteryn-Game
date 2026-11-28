//! BOOSTED-1: the committed World/server-save choice shared by queries, kills and home spawns.
use super::{AdmissionRetryBudget, ComposedFreshAdmission, connection::AdmittedSession};
use crate::content::ProjectV2AuthoringProfileData;
use crate::durability::{DurabilityError, daily_boost::DailyBoostBinding};
use crate::foundation::RuntimeScopeRefV1;
use oteryn_protocol_oteryn::daily_boost as wire;

#[derive(Debug)]
pub enum DailyBoostReply {
    Rejected,
    Busy,
    Payload(Vec<u8>),
    Unavailable,
}

#[cfg(test)]
tokio::task_local! {
    pub(in crate::gameplay_transport) static DAILY_BOOST_CHECKOUT_PROBE:
        std::cell::Cell<Option<std::sync::Arc<super::composed_admission::HolderCheckoutProbe>>>;
}

pub(super) fn offered(
    base: &[super::capabilities::OfferedCapability],
    available: bool,
) -> Vec<super::capabilities::OfferedCapability> {
    let mut offers: Vec<_> = base
        .iter()
        .copied()
        .filter(|c| c.id != wire::CAPABILITY_DAILY_BOOST_V1)
        .collect();
    if available {
        let at = offers.partition_point(|c| c.id < wire::CAPABILITY_DAILY_BOOST_V1);
        offers.insert(
            at,
            super::capabilities::OfferedCapability {
                id: wire::CAPABILITY_DAILY_BOOST_V1,
                requires: &[],
            },
        );
    }
    offers
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn daily_boost_pools(&self) -> (Vec<String>, Vec<String>) {
        let mut creatures = Vec::new();
        let mut bosses = Vec::new();
        if let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        {
            for record in &native.creature_profiles().records {
                let ProjectV2AuthoringProfileData::Creature(profile) = &record.profile.data else {
                    continue;
                };
                if profile.bestiary.is_some()
                    && profile.bosstiary.is_none()
                    && profile.familiar.is_none()
                    && !profile.details.as_ref().is_some_and(|d| {
                        d.bosstiary.is_some()
                            || d.system_eligibility.reward_boss
                            || d.summoning.is_familiar
                    })
                {
                    creatures.push(record.profile.target.key.clone());
                }
                if profile
                    .bosstiary
                    .as_ref()
                    .map(|b| b.category.as_str())
                    .or_else(|| {
                        profile
                            .details
                            .as_ref()?
                            .bosstiary
                            .as_ref()
                            .map(|b| b.category.as_str())
                    })
                    .is_some_and(|category| category.eq_ignore_ascii_case("archfoe"))
                {
                    bosses.push(record.profile.target.key.clone());
                }
            }
        }
        (creatures, bosses)
    }

    pub(super) async fn initialize_daily_boost(&self) -> Result<(), ()> {
        let budget = AdmissionRetryBudget::for_command(self.root);
        self.load_daily_boost(&budget, false).await.map_err(|_| ())
    }

    async fn load_daily_boost(
        &self,
        budget: &AdmissionRetryBudget<'_>,
        refresh: bool,
    ) -> Result<(), DurabilityError> {
        let Some(map) = self.bundle_map() else {
            return Ok(());
        };
        {
            let runtime = tokio::time::timeout_at(budget.deadline, self.runtime.lock())
                .await
                .map_err(|_| DurabilityError::RootPassDeadlineExceeded)?;
            runtime.owner_fence().map_err(|_| DurabilityError::Unavailable)?;
            if self
                .active_generation
                .and_then(crate::content::ActiveGeneration::native_gameplay)
                .is_some_and(|native| {
                    native.source_digest() != runtime.content_pin().server_artifact_digest()
                })
            {
                return Err(DurabilityError::Unavailable);
            }
            if !refresh
                && runtime.daily_boost().is_some_and(|choice| choice.reset_epoch == self.reset_epoch)
            {
                return Ok(());
            }
        }
        let pools = self.daily_boost_pools();
        if pools.0.is_empty() || pools.1.is_empty() {
            return Ok(());
        }
        // This world-level selection has no player effects to reconcile. Preserve any
        // parked spell write for its owner and wait only for the lane within our budget.
        let permit = tokio::time::timeout_at(budget.deadline, self.spell_lane.acquire())
            .await
            .map_err(|_| DurabilityError::RootPassDeadlineExceeded)?
            .map_err(|_| DurabilityError::Unavailable)?;
        let (binding, content_digest) = {
            let runtime = tokio::time::timeout_at(budget.deadline, self.runtime.lock())
                .await
                .map_err(|_| DurabilityError::RootPassDeadlineExceeded)?;
            runtime.owner_fence().map_err(|_| DurabilityError::Unavailable)?;
            (runtime.binding(), runtime.content_pin().server_artifact_digest())
        };
        #[cfg(test)]
        let probe = refresh.then(|| DAILY_BOOST_CHECKOUT_PROBE.try_with(std::cell::Cell::take).ok().flatten()).flatten();
        #[cfg(test)]
        if let Some(probe) = &probe {
            probe.pause().await;
        }
        let selection = budget.retry_durability("daily_boost_selection", async || {
            budget.check_deadline()?;
            let result = self.root.prepare_daily_boost(
                self.character,
                self.holder,
                &permit,
                DailyBoostBinding {
                    scope: binding.runtime_scope(),
                    generation: binding.scope_generation().get(),
                    bundle: map.overlay.base().digest(),
                    epoch: self.reset_epoch,
                },
                pools.clone(),
            ).await;
            #[cfg(test)]
            if matches!(result, Err(DurabilityError::RootUnavailable)) && let Some(probe) = &probe {
                probe.blocked();
            }
            result
        }).await?;
        let mut runtime = tokio::time::timeout_at(budget.deadline, self.runtime.lock())
            .await
            .map_err(|_| DurabilityError::RootPassDeadlineExceeded)?;
        if runtime.binding() != binding
            || runtime.content_pin().server_artifact_digest() != content_digest
        {
            return Err(DurabilityError::Unavailable);
        }
        runtime.install_daily_boost(selection).map_err(|_| DurabilityError::Unavailable)

    }

    #[cfg(test)]
    async fn load_with_test_trade_contention(
        &self,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
        load: impl std::future::Future<Output = Result<(), DurabilityError>>,
    ) -> Result<(), DurabilityError> {
        use std::sync::atomic::Ordering;
        if !self
            .states
            .test_daily_boost_parked_trade
            .swap(false, Ordering::Relaxed)
        {
            return load.await;
        }
        // Enter after command ingress has read its own fences. The reservation
        // now belongs specifically to the lane reconciliation this query calls.
        let binding = self.runtime.lock().await.binding();
        super::monk_save::postgres_tests::maintain_authority_ready(self).await;
        let holder = self
            .root
            .try_issue_semantic_pass()
            .map_err(|_| DurabilityError::Unavailable)?;
        self.states
            .player_trades
            .retain_test_request(binding, fence)?;
        let result = load.await;
        self.states
            .player_trades
            .remove_test_request(fence.game_session_id)?;
        drop(holder);
        result
    }

    pub(super) async fn daily_boost_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> DailyBoostReply {
        let Some(fence) = admitted.item_fence else {
            return DailyBoostReply::Rejected;
        };
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
            || id == 0
            || kind != wire::COMMAND_TYPE_DAILY_BOOST_QUERY
            || !admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_DAILY_BOOST_V1)
            || wire::decode_query(payload).is_err()
        {
            return DailyBoostReply::Rejected;
        }
        let budget = AdmissionRetryBudget::for_command(self.root);
        {
            let Ok(runtime) = tokio::time::timeout_at(budget.deadline, self.runtime.lock()).await
            else {
                return DailyBoostReply::Busy;
            };
            if !Self::daily_boost_session_is_current(&runtime, admitted, fence) {
                return DailyBoostReply::Rejected;
            }
        }
        let load = self.load_daily_boost(&budget, true);
        #[cfg(test)]
        let load = self.load_with_test_trade_contention(fence, load);
        match load.await {
            Ok(()) => {}
            // The root refused checkout before this query could read or write.
            Err(DurabilityError::RootUnavailable | DurabilityError::RootPassDeadlineExceeded) => {
                return DailyBoostReply::Busy;
            }
            Err(_) => return DailyBoostReply::Unavailable,
        }
        let Ok(runtime) = tokio::time::timeout_at(budget.deadline, self.runtime.lock()).await
        else {
            return DailyBoostReply::Busy;
        };
        if !Self::daily_boost_session_is_current(&runtime, admitted, fence) {
            return DailyBoostReply::Rejected;
        }
        let Some(choice) = runtime.daily_boost() else {
            return DailyBoostReply::Rejected;
        };
        match wire::encode_view(&wire::DailyBoostView {
            reset_epoch: choice.reset_epoch,
            creature_key: choice.creature.clone(),
            boss_key: choice.boss.clone(),
        }) {
            Ok(bytes) => DailyBoostReply::Payload(bytes),
            Err(_) => DailyBoostReply::Unavailable,
        }
    }

    fn daily_boost_session_is_current(
        runtime: &crate::foundation::ChannelRuntimeV1,
        admitted: AdmittedSession,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    ) -> bool {
        runtime.owner_fence().is_ok()
            && fence.runtime_scope == RuntimeScopeRefV1::channel(admitted.world_id, admitted.channel_id)
            && fence.scope_ownership_generation == runtime.binding().scope_generation()
            && runtime.binding().world_id() == admitted.world_id
            && runtime.binding().channel_id() == Some(admitted.channel_id)
            && admitted.runtime_actor.is_some_and(|actor| {
                runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            })
    }

}

macro_rules! authority_methods {
    () => {
        fn daily_boost_available(&self) -> bool {
            false
        }
        fn daily_boost_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _id: u64,
            _kind: u32,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = $crate::gameplay_transport::daily_boost::DailyBoostReply>
        {
            async { $crate::gameplay_transport::daily_boost::DailyBoostReply::Rejected }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn daily_boost_available(&self) -> bool {
            if self.bundle_map().is_none() {
                return false;
            }
            let pools = self.daily_boost_pools();
            !pools.0.is_empty() && !pools.1.is_empty()
        }
        async fn daily_boost_command(
            &self,
            admitted: connection::AdmittedSession,
            id: u64,
            kind: u32,
            payload: &[u8],
        ) -> crate::gameplay_transport::daily_boost::DailyBoostReply {
            self.daily_boost_wire(admitted, id, kind, payload).await
        }
    };
}
pub(super) use composed_methods;
