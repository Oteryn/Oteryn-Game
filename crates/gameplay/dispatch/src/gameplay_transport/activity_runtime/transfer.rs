impl<'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    /// The lifecycle calls this only after its fresh durable retirement witness
    /// and Ground Complete. Minimal session lineage survives lost wire receipts;
    /// completed tokens must not retain the retired physical runtime forever.
    pub(super) async fn retire_and_unregister_activity(
        &self,
        scope: RuntimeScopeRefV1,
        generation: u64,
    ) -> Result<(), ()> {
        let Some(owner) = self.activity_runtimes.get(scope, generation) else {
            return Ok(());
        };
        if owner.template.is_none() {
            return Err(());
        }
        self.prune_completed_activity_entries(scope, generation)
            .await?;
        self.prune_completed_activity_returns(scope, generation)
            .await?;
        let mut runtime = owner.runtime.lock().await;
        self.activity_runtimes
            .retire_and_remove(&owner, &mut runtime)
    }

    pub(super) async fn transfer_activity_session_state(
        &self,
        destination: &Self,
        source: AdmittedSession,
        next: AdmittedSession,
    ) -> Result<(), ()> {
        let old = source.game_session_id;
        let new = next.game_session_id;
        let actor = next.runtime_actor.ok_or(())?;
        let character = source.item_fence.ok_or(())?.character_id;
        if next.item_fence.ok_or(())?.character_id != character {
            return Err(());
        }
        self.transfer_activity_equipment_clocks(destination, old, new)
            .await;
        move_session_entry(
            &self.states.progression_sessions,
            &destination.states.progression_sessions,
            old,
            new,
        )?;
        move_session_entry(
            &self.states.quest_sessions,
            &destination.states.quest_sessions,
            old,
            new,
        )?;
        move_session_entry(
            &self.states.wheel_sessions,
            &destination.states.wheel_sessions,
            old,
            new,
        )?;
        move_session_entry(
            &self.states.atelier_sessions,
            &destination.states.atelier_sessions,
            old,
            new,
        )?;
        move_session_entry(
            &self.states.proficiencies,
            &destination.states.proficiencies,
            old,
            new,
        )?;
        // The frozen equipment witness belongs to the same contributor owner as
        // later kills. Rebind only this authenticated physical successor; pending
        // old deaths retain their original contributor/session witnesses.
        let source_actor = source.runtime_actor.ok_or(())?;
        let equipment = self
            .states
            .attack
            .lock()
            .await
            .kills()
            .activity_proficiency_equipment(source_actor, old);
        if let Some((revision, weapon)) = equipment {
            destination
                .states
                .attack
                .lock()
                .await
                .kills()
                .proficiency_equipment(actor, new, revision, weapon.as_deref());
        }
        {
            let mut from = self.states.premium_sessions.lock().map_err(|_| ())?;
            let mut to = destination.states.premium_sessions.lock().map_err(|_| ())?;
            from.transfer_activity(&mut to, old, new);
        }
        self.states.last_positions.transfer_activity(
            &destination.states.last_positions,
            old,
            new,
            actor,
        )?;
        if next.controller.is_none() {
            self.states
                .transfer_lost_to(&destination.states, old, next)?;
        }
        // Settlement publishes these effects before releasing its recipient
        // revisions/lane. These same guards remain held across this move, so a
        // committed kill cannot strand a late mark on the retired source owner.
        self.states
            .level_refresh
            .transfer_activity(&destination.states.level_refresh, character)?;
        self.states
            .title_refresh
            .transfer_activity(&destination.states.title_refresh, character)?;
        self.states
            .soul_kills
            .transfer_activity(&destination.states.soul_kills, character)?;
        self.states
            .cyclopedia_tracker_refresh
            .transfer_activity(&destination.states.cyclopedia_tracker_refresh, character)?;
        destination.states.command_ingress.admit(new);
        self.states.appearance_commands.forget(old);
        self.states.chat.forget(old);
        self.states.player_trades.forget(old);
        self.states.inspection_permissions.forget(old);
        self.states.npc_conversations.forget(old);
        self.states.tutorial_sessions.forget(old);
        self.states.newhaven_travel.forget(old);
        if Arc::ptr_eq(
            &self.states.server_messages,
            &destination.states.server_messages,
        ) {
            self.states
                .server_messages
                .transfer_connection(source, next)?;
        }
        destination.states.gameplay_idle_presence.observe(next);
        self.states.gameplay_idle_presence.forget(old);
        // The physical transfer clears session-bound projections. Reload the
        // durable selection under the new actor/session fence before publishing
        // an active successor; the old session's cache cannot authorize it.
        // An offline return is hydrated by its ordinary fresh admission later.
        if next.controller.is_some() && !destination.admit_hazard(next).await {
            return Err(());
        }
        Ok(())
    }
}
