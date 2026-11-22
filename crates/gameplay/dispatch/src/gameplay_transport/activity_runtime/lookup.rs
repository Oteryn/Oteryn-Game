impl <'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    pub(super) fn runtime_view(&self, owner: &RegisteredRuntime<'a>) -> Self {
        let mut view = self.clone();
        view.runtime = owner.runtime.clone();
        view.door = owner.door.clone();
        view.states = Arc::clone(&owner.states);
        view.required_capabilities = Arc::clone(&owner.required_capabilities);
        view.spell_lane = owner.lane.clone();
        view.movement_cells = owner.assets.movement_cells.clone();
        view.rooms = owner.assets.rooms.clone();
        if let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = owner.scope
        {
            view.world_id = world_id;
            view.channel_id = channel_id;
        }
        view
    }

    pub(super) async fn resolve_runtime_view(
        &self,
        admitted: AdmittedSession,
    ) -> Result<Option<Self>, ()> {
        let Some(actor) = admitted.runtime_actor else {
            return Ok(None);
        };
        let Some(fence) = admitted.item_fence else {
            return Ok(None);
        };
        if actor.runtime_scope() != fence.runtime_scope {
            return Ok(None);
        } // Existing House owner.
        let runtime = self.runtime.lock().await;
        // Legacy Channel commands still reach their existing fence validator.
        // This never routes an Instance or a selected activity connection around
        // its exact generation check; the actual Channel and owner stay current.
        let ordinary_channel = matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
            && !admitted
                .continuity
                .selected_capabilities
                .contains(oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1);
        if runtime.binding().runtime_scope() == fence.runtime_scope
            && (runtime.binding().scope_generation() == fence.scope_ownership_generation
                || ordinary_channel)
        {
            runtime.owner_fence().map_err(|_| ())?;
            return Ok(None);
        }
        drop(runtime);
        let target = self
            .activity_runtimes
            .get(fence.runtime_scope, fence.scope_ownership_generation.get())
            .ok_or(())?;
        let next = self.runtime_view(&target);
        {
            let runtime = next.runtime.lock().await;
            let guard = runtime.owner_fence().map_err(|_| ())?;
            if !guard.is_current_for_scope(fence.runtime_scope, fence.scope_ownership_generation)
                || runtime
                    .player_control_facts(actor, admitted.game_session_id)
                    .is_err()
            {
                return Err(());
            }
        }
        Ok(Some(next))
    }
}
