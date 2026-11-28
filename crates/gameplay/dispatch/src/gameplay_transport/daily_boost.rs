//! BOOSTED-1: the committed World/server-save choice shared by queries, kills and home spawns.
use super::{
    AdmissionRetryBudget, ComposedFreshAdmission, connection::AdmittedSession, operator_event,
};
use crate::content::ProjectV2AuthoringProfileData;
use crate::durability::DurabilityError;
use crate::durability::daily_boost::DailyBoostBinding;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::ChannelRuntimeV1;
use crate::foundation::RuntimeScopeRefV1;
use oteryn_protocol_oteryn::daily_boost::{self as wire, DailyBoostRefusal};

#[cfg(test)]
tokio::task_local! {
    pub(in crate::gameplay_transport) static DAILY_BOOST_RUNTIME_PROBE:
        (std::sync::Arc<tokio::sync::Notify>, std::sync::Arc<tokio::sync::Notify>);

    pub(in crate::gameplay_transport) static DAILY_BOOST_CHECKOUT_PROBE:
        std::cell::Cell<Option<std::sync::Arc<super::composed_admission::HolderCheckoutProbe>>>;
}

#[derive(Clone, Copy)]
struct DailyBoostQuery {
    admitted: AdmittedSession,
    fence: CurrentCharacterItemFence,
    command_id: u64,
}

impl DailyBoostQuery {
    fn attempt(self, phase: &'static str, attempt: u32, holder: impl std::fmt::Display) {
        operator_event(&format!(
            "event=daily_boost_query_attempt command_id={} seq={} phase={phase} attempt={attempt} holder={holder}",
            self.command_id,
            self.admitted.continuity.server_sequence.saturating_add(1),
        ));
    }
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
        self.load_daily_boost(&budget, None)
            .await
            .map(|_| ())
            .map_err(|_| ())
    }

    async fn runtime_for_daily_boost(
        &self,
        budget: &AdmissionRetryBudget<'_>,
        query: Option<DailyBoostQuery>,
        busy: DailyBoostRefusal,
    ) -> Result<tokio::sync::MutexGuard<'_, ChannelRuntimeV1>, DailyBoostRefusal> {
        #[cfg(test)]
        if busy == DailyBoostRefusal::RuntimeBusyBeforeSelection
            && let Ok((query_started, runtime_held)) = DAILY_BOOST_RUNTIME_PROBE.try_with(Clone::clone)
        {
            query_started.notify_one();
            runtime_held.notified().await;
        }
        let runtime = match self.runtime.try_lock() {
            Ok(runtime) => {
                if let Some(query) = query
                    && let Some(phase) = busy.runtime_phase()
                {
                    query.attempt(phase, 1, "none");
                }
                runtime
            }
            Err(_) => {
                if let Some(query) = query
                    && let Some(phase) = busy.runtime_phase()
                {
                    query.attempt(phase, 1, "channel_runtime");
                    query.attempt(phase, 2, "channel_runtime");
                }
                // Retain the FIFO ticket until the command's deadline.
                tokio::time::timeout_at(budget.deadline, self.runtime.lock())
                    .await
                    .map_err(|_| busy)?
            }
        };
        if let Some(query) = query {
            Self::check_daily_boost_session(&runtime, query.admitted, query.fence)?;
        }
        Ok(runtime)
    }

    async fn load_daily_boost(
        &self,
        budget: &AdmissionRetryBudget<'_>,
        query: Option<DailyBoostQuery>,
    ) -> Result<Option<wire::DailyBoostView>, DailyBoostRefusal> {
        let Some(map) = self.bundle_map() else {
            return Ok(None);
        };
        let (binding, content_digest) = {
            let runtime = self
                .runtime_for_daily_boost(
                    budget,
                    query,
                    DailyBoostRefusal::RuntimeBusyBeforeSelection,
                )
                .await?;
            runtime
                .owner_fence()
                .map_err(|_| DailyBoostRefusal::OwnerUnavailable)?;
            let digest = runtime.content_pin().server_artifact_digest();
            if self
                .active_generation
                .and_then(crate::content::ActiveGeneration::native_gameplay)
                .is_some_and(|native| native.source_digest() != digest)
            {
                return Err(DailyBoostRefusal::SelectionUnavailable);
            }
            if query.is_none()
                && runtime
                    .daily_boost()
                    .is_some_and(|choice| choice.reset_epoch == self.reset_epoch)
            {
                return Ok(None);
            }
            (runtime.binding(), digest)
        };
        let pools = self.daily_boost_pools();
        if pools.0.is_empty() || pools.1.is_empty() {
            return Ok(None);
        }
        #[cfg(test)]
        let probe = query.and_then(|_| {
            DAILY_BOOST_CHECKOUT_PROBE
                .try_with(std::cell::Cell::take)
                .ok()
                .flatten()
        });
        #[cfg(test)]
        if let Some(probe) = &probe {
            probe.pause().await;
        }
        let mut attempt = 0_u32;
        let (selection, _permit) = budget
            .retry_durability("daily_boost_selection", || {
                attempt = attempt.saturating_add(1);
                if let Some(query) = query {
                    let holder = self.root.checked_out_pass_site();
                    query.attempt(
                        "selection",
                        attempt,
                        holder.map_or_else(|| "unobserved".to_owned(), |site| site.to_string()),
                    );
                }
                async {
                    budget.check_deadline()?;
                    // A holder miss releases the lane before the budget waits for readiness.
                    // Parked spell writes remain with their own reconciliation path.
                    let permit =
                        tokio::time::timeout_at(budget.deadline, self.spell_lane.acquire())
                            .await
                            .map_err(|_| DurabilityError::RootPassDeadlineExceeded)?
                            .map_err(|_| DurabilityError::Unavailable)?;
                    self.resolve_player_trades(&permit).await?;
                    let selection_binding = DailyBoostBinding {
                        scope: binding.runtime_scope(),
                        generation: binding.scope_generation().get(),
                        bundle: map.overlay.base().digest(),
                        epoch: self.reset_epoch,
                    };
                    let result = if query.is_some() {
                        self.root
                            .prepare_daily_boost_by(
                                self.character,
                                self.holder,
                                &permit,
                                selection_binding,
                                pools.clone(),
                                budget.deadline,
                            )
                            .await
                    } else {
                        self.root
                            .prepare_daily_boost(
                                self.character,
                                self.holder,
                                &permit,
                                selection_binding,
                                pools.clone(),
                            )
                            .await
                    };
                    #[cfg(test)]
                    if matches!(result, Err(DurabilityError::RootUnavailable))
                        && let Some(probe) = &probe
                    {
                        probe.blocked();
                    }
                    result.map(|selection| (selection, permit))
                }
            })
            .await
            .map_err(selection_refusal)?;
        let mut runtime = self
            .runtime_for_daily_boost(budget, query, DailyBoostRefusal::RuntimeBusyAfterSelection)
            .await?;
        if runtime.binding() != binding
            || runtime.content_pin().server_artifact_digest() != content_digest
        {
            return Err(DailyBoostRefusal::SelectionUnavailable);
        }
        runtime
            .install_daily_boost(selection)
            .map_err(|_| DailyBoostRefusal::SelectionUnavailable)?;
        // Publish and read under the same guard. A queued owner cannot take
        // the runtime between installation and the wire projection.
        Ok(query.and_then(|_| {
            runtime
                .daily_boost()
                .filter(|choice| choice.reset_epoch == self.reset_epoch)
                .map(|choice| wire::DailyBoostView {
                    reset_epoch: choice.reset_epoch,
                    creature_key: choice.creature.clone(),
                    boss_key: choice.boss.clone(),
                })
        }))
    }

    #[cfg(test)]
    async fn load_with_test_trade_contention(
        &self,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
        load: impl std::future::Future<Output = Result<Option<wire::DailyBoostView>, DailyBoostRefusal>>,
    ) -> Result<Option<wire::DailyBoostView>, DailyBoostRefusal> {
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
            .map_err(selection_refusal)?;
        self.states
            .player_trades
            .retain_test_request(binding, fence).map_err(selection_refusal)?;
        let result = load.await;
        self.states
            .player_trades
            .remove_test_request(fence.game_session_id).map_err(selection_refusal)?;
        drop(holder);
        result
    }

    pub(super) async fn daily_boost_wire(
        &self,
        admitted: AdmittedSession,
        id: u64,
        kind: u32,
        payload: &[u8],
    ) -> Result<Vec<u8>, DailyBoostRefusal> {
        if id == 0 {
            return Err(DailyBoostRefusal::InvalidCommandId);
        }
        if kind != wire::COMMAND_TYPE_DAILY_BOOST_QUERY {
            return Err(DailyBoostRefusal::WrongCommand);
        }
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_DAILY_BOOST_V1)
        {
            return Err(DailyBoostRefusal::CapabilityNotSelected);
        }
        wire::decode_query(payload).map_err(|_| DailyBoostRefusal::MalformedQuery)?;
        let fence = admitted.item_fence.ok_or(DailyBoostRefusal::MissingFence)?;
        if fence.game_session_id != admitted.game_session_id {
            return Err(DailyBoostRefusal::SessionMismatch);
        }
        if fence.connection_generation.get() != admitted.continuity.connection_generation {
            return Err(DailyBoostRefusal::ConnectionGenerationMismatch);
        }
        let budget =
            AdmissionRetryBudget::for_session_read(self.root).map_err(selection_refusal)?;
        let load = self.load_daily_boost(
            &budget,
            Some(DailyBoostQuery { admitted, fence, command_id: id }),
        );
        #[cfg(test)]
        let load = self.load_with_test_trade_contention(fence, load);
        let view = load.await?.ok_or(DailyBoostRefusal::SelectionMissing)?;
        wire::encode_view(&view).map_err(|_| DailyBoostRefusal::ViewEncoding)
    }

    fn check_daily_boost_session(
        runtime: &ChannelRuntimeV1,
        admitted: AdmittedSession,
        fence: CurrentCharacterItemFence,
    ) -> Result<(), DailyBoostRefusal> {
        runtime
            .owner_fence()
            .map_err(|_| DailyBoostRefusal::OwnerUnavailable)?;
        if fence.runtime_scope != RuntimeScopeRefV1::channel(admitted.world_id, admitted.channel_id)
        {
            return Err(DailyBoostRefusal::ScopeMismatch);
        }
        if fence.scope_ownership_generation != runtime.binding().scope_generation() {
            return Err(DailyBoostRefusal::ScopeGenerationMismatch);
        }
        if runtime.binding().world_id() != admitted.world_id {
            return Err(DailyBoostRefusal::WorldMismatch);
        }
        if runtime.binding().channel_id() != Some(admitted.channel_id) {
            return Err(DailyBoostRefusal::ChannelMismatch);
        }
        let actor = admitted
            .runtime_actor
            .ok_or(DailyBoostRefusal::MissingActor)?;
        let control = runtime
            .player_control_facts(actor, admitted.game_session_id)
            .map_err(|_| DailyBoostRefusal::ActorNotCurrent)?;
        if control.control_loss.is_some() {
            return Err(DailyBoostRefusal::ControlLost);
        }
        Ok(())
    }
}

fn selection_refusal(error: DurabilityError) -> DailyBoostRefusal {
    match error {
        DurabilityError::RootUnavailable => DailyBoostRefusal::HolderBusy,
        DurabilityError::RootPassDeadlineExceeded => DailyBoostRefusal::DeadlineExceeded,
        _ => DailyBoostRefusal::SelectionUnavailable,
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
        ) -> impl std::future::Future<
            Output = Result<Vec<u8>, oteryn_protocol_oteryn::daily_boost::DailyBoostRefusal>,
        > {
            async { Err(oteryn_protocol_oteryn::daily_boost::DailyBoostRefusal::Unavailable) }
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
        ) -> Result<Vec<u8>, oteryn_protocol_oteryn::daily_boost::DailyBoostRefusal> {
            self.daily_boost_wire(admitted, id, kind, payload).await
        }
    };
}
pub(super) use composed_methods;
