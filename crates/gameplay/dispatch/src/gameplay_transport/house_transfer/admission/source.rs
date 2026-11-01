//! Character admission owners run while the authenticated source is still a
//! real active Channel session. No temporary Channel position is published.
use crate::durability::character_last_position::ScopedLastPosition;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::fresh_admission_durability::FreshAdmissionCommitRequestV1;
use crate::foundation::{GameSessionState, RuntimeScopeRefV1};
use crate::gameplay_transport::connection::{AdmissionRefusal, AdmittedSession};
use crate::gameplay_transport::{AdmissionRetryBudget, ComposedFreshAdmission, item_fence_of};

pub(super) struct HouseSourceViews {
    pub(super) source: AdmittedSession,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn prepare_house_source(
        &self,
        source: AdmittedSession,
        request: &FreshAdmissionCommitRequestV1,
    ) -> Result<Option<(ScopedLastPosition, HouseSourceViews)>, AdmissionRefusal> {
        use AdmissionRefusal::Unavailable;
        let budget = AdmissionRetryBudget::for_root(&source.game_session_id, self.root);
        let item = source.item_fence.ok_or(Unavailable)?;
        budget
            .retry_unavailable(async || {
                current_source(self, &source).await?;
                if let Some(native) = self
                    .active_generation
                    .and_then(|active| active.native_gameplay())
                {
                    self.root
                        .initialize_admission_equipment(
                            self.character,
                            self.holder,
                            &item,
                            native.source_digest(),
                            request.operation(),
                        )
                        .await
                        .map_err(|_| Unavailable)?;
                    self.root
                        .grant_starter_items(
                            self.character,
                            self.holder,
                            &item,
                            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(
                                native,
                            ),
                            &crate::durability::starter_grant::StarterTemplate::main(),
                        )
                        .await
                        .map_err(|_| Unavailable)?;
                    let fence = self
                        .current_quest_fence(source.game_session_id)
                        .await
                        .map_err(|_| Unavailable)?
                        .ok_or(Unavailable)?;
                    self.root
                        .initialize_familiar_group(self.character, self.holder, fence)
                        .await
                        .map_err(|_| Unavailable)?;
                }
                let permit = self.spell_lane_permit().await.ok_or(Unavailable)?;
                let fence = self
                    .current_quest_fence(source.game_session_id)
                    .await
                    .map_err(|_| Unavailable)?
                    .ok_or(Unavailable)?;
                self.resume_pending_player_death_items(&permit, fence)
                    .await
                    .map_err(|_| Unavailable)?;
                drop(permit);
                if self
                    .bundle_map()
                    .and_then(|map| map.overlay.base().subareas.default_town)
                    .is_some()
                    && self.home_town(source.game_session_id, None).await.is_none()
                {
                    return Err(Unavailable);
                }
                Ok(())
            })
            .await?;

        // Use the existing source owner and its exact priority: pending respawn,
        // committed NPC arrival, tutorial placement, then stored position. A
        // remembered House never suppresses an earlier placement obligation.
        let candidate = budget
            .retry_unavailable(async || {
                current_source(self, &source).await?;
                let fence = self
                    .current_quest_fence(source.game_session_id)
                    .await
                    .map_err(|_| Unavailable)?
                    .ok_or(Unavailable)?;
                let chosen = self
                    .read_tutorial_admission_position(fence)
                    .await
                    .map_err(|_| Unavailable)?;
                let crate::gameplay_transport::last_position::AdmissionPosition::Stored(selected) =
                    chosen
                else {
                    return Ok(None);
                };
                let current = self
                    .current_quest_fence(source.game_session_id)
                    .await
                    .map_err(|_| Unavailable)?
                    .ok_or(Unavailable)?;
                let saved = self
                    .root
                    .read_scoped_character_last_position(self.character, self.holder, current)
                    .await
                    .map_err(|_| Unavailable)?;
                Ok(saved.filter(|saved| saved.house.is_some() && saved.position == selected))
            })
            .await?;
        let Some(saved) = candidate else {
            return Ok(None);
        };

        // Match the ordinary admission ordering. These session/account hooks
        // use the real source identity, before the later House handoff retires it.
        let account = source.controller.ok_or(Unavailable)?.account_id;
        self.admit_premium(account, source.game_session_id);
        self.admit_promotion(source.game_session_id).await;
        self.refresh_session_titles(source.game_session_id).await;
        self.bind_house_source_progression(&source, &budget).await?;
        budget
            .retry_unavailable(async || {
                current_source(self, &source).await?;
                let fence = self
                    .current_quest_fence(source.game_session_id)
                    .await
                    .map_err(|_| Unavailable)?
                    .ok_or(Unavailable)?;
                self.root
                    .character_hazard(self.character, self.holder, fence, None)
                    .await
                    .map_err(|_| Unavailable)?
                    .map(|(_, selected, _)| selected)
                    .ok_or(Unavailable)
            })
            .await?;
        if !budget
            .retry_until(async || self.settle_channel_source_training(item).await)
            .await
        {
            return Err(Unavailable);
        }
        budget
            .retry_unavailable(async || {
                current_source(self, &source).await?;
                self.prepare_admission_prey(source)
                    .await
                    .map_err(|_| Unavailable)
            })
            .await?;
        self.finish_house_source_quests(&source, &budget).await?;
        current_source(self, &source).await?;
        Ok(Some((saved, HouseSourceViews { source })))
    }

    /// Publish only data after the native handoff and current House proof. The
    /// native mutation owners still require their existing current authorities.
    pub(super) async fn install_house_source_views(
        &self,
        admitted: &AdmittedSession,
        prepared: HouseSourceViews,
    ) -> Result<(), AdmissionRefusal> {
        self.carry_house_session_views(&prepared.source, admitted)
            .await?;
        self.restore_house_actor_admission_views(admitted).await
    }

    /// Lost acknowledgement recovery reads the already committed Character
    /// state under its actual destination. It never repeats source settlement,
    /// starter grants, quest obligations or automatic-spell repair.
    pub(in crate::gameplay_transport) async fn restore_house_admission_views(
        &self,
        admitted: &AdmittedSession,
        retired_source: Option<crate::foundation::GameSessionId>,
    ) -> Result<(), AdmissionRefusal> {
        if let Some(source) = retired_source {
            self.carry_recovered_house_session_views(source, admitted)
                .await?;
        }
        self.restore_house_actor_admission_views(admitted).await?;
        self.restore_house_progression_views(admitted).await?;
        if let Some(source) = retired_source {
            self.cleanup_house_source_progression(source, admitted.game_session_id)?;
        }
        Ok(())
    }

    async fn restore_house_actor_admission_views(
        &self,
        admitted: &AdmittedSession,
    ) -> Result<(), AdmissionRefusal> {
        use AdmissionRefusal::Unavailable;
        let actor = admitted.runtime_actor.ok_or(Unavailable)?;
        let fence = admitted.item_fence.ok_or(Unavailable)?;
        let controller = admitted.controller.ok_or(Unavailable)?;
        let (current, _) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(admitted.game_session_id)
            .await
            .map_err(|_| Unavailable)?;
        if current.session_state() != GameSessionState::Active
            || current.current_transport() != Some(controller.transport)
            || item_fence_of(current) != Some(fence)
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Instance { .. })
        {
            return Err(AdmissionRefusal::Rejected);
        }
        let revision = self
            .root
            .read_current_character(self.character, fence.character_id)
            .await
            .map_err(|_| Unavailable)?
            .revision;
        let (_, hazard) = self
            .root
            .read_house_hazard_selection(self.character, self.holder, fence, admitted.channel_id)
            .await
            .map_err(|_| Unavailable)?
            .ok_or(Unavailable)?;
        let prey = self
            .read_house_admission_prey(admitted)
            .await
            .map_err(|_| Unavailable)?;
        let daily = self
            .root
            .read_daily_reward_streak(self.character, fence.character_id, self.world_id)
            .await
            .map_err(|_| Unavailable)?;
        // Preserve ordinary admission's optional Charm snapshot: unavailable
        // data leaves Charm evaluation closed, rather than granting a default.
        let charm = self
            .root
            .read_character_charm_state(self.character, fence.character_id)
            .await
            .ok()
            .map(|state| {
                crate::durability::charm_state::CombatCharmState::new(fence.character_id, state)
            });
        let (current, _) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(admitted.game_session_id)
            .await
            .map_err(|_| Unavailable)?;
        if current.session_state() != GameSessionState::Active
            || current.current_transport() != Some(controller.transport)
            || item_fence_of(current) != Some(fence)
            || self
                .root
                .read_current_character(self.character, fence.character_id)
                .await
                .map_err(|_| Unavailable)?
                .revision
                != revision
        {
            return Err(Unavailable);
        }
        let mut runtime = self.runtime.lock().await;
        let house = runtime
            .house_actor_state(actor, admitted.game_session_id)
            .map_err(|_| Unavailable)?;
        if house.scope != fence.runtime_scope
            || house.generation != fence.scope_ownership_generation
            || house.lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || house.lease.generation() != fence.character_lease_generation
        {
            return Err(AdmissionRefusal::Rejected);
        }
        runtime
            .install_house_hazard_level(actor, admitted.game_session_id, hazard)
            .map_err(|_| Unavailable)?;
        let mut states = self.states.spell_states.lock().await;
        if let Some((reply, permanent)) = prey {
            states
                .load_house_prey(
                    &runtime,
                    actor,
                    admitted.game_session_id,
                    fence,
                    reply,
                    permanent,
                )
                .ok_or(Unavailable)?;
        }
        states
            .install_house_charm_state(&runtime, actor, admitted.game_session_id, fence, charm)
            .ok_or(Unavailable)?;
        self.states
            .resting_areas
            .refresh(admitted.game_session_id, actor, daily);
        drop(states);
        drop(runtime);
        Ok(())
    }
}

async fn current_source(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    source: &AdmittedSession,
) -> Result<(), AdmissionRefusal> {
    let fence = source.item_fence.ok_or(AdmissionRefusal::Unavailable)?;
    let controller = source.controller.ok_or(AdmissionRefusal::Unavailable)?;
    let (current, _) = FreshAdmissionStore::from_root(owner.root.clone())
        .current_session_at(source.game_session_id)
        .await
        .map_err(|_| AdmissionRefusal::Unavailable)?;
    if source.world_id != owner.world_id
        || source.channel_id != owner.channel_id
        || current.session_state() != GameSessionState::Active
        || current.current_transport() != Some(controller.transport)
        || item_fence_of(current) != Some(fence)
        || fence.runtime_scope != RuntimeScopeRefV1::channel(owner.world_id, owner.channel_id)
    {
        return Err(AdmissionRefusal::Rejected);
    }
    Ok(())
}
