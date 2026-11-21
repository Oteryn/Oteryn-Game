impl <'a> ActivityRuntimeRegistry<'a> {
    pub(super) fn register_soulpit_template(
        &self,
        template: &'a crate::content::soulpit_activity::QualifiedSoulpitActivityTemplate,
    ) -> Result<(), ()> {
        let mut current = self.soulpit_template.lock().map_err(|_| ())?;
        if current.is_some_and(|existing| !std::ptr::eq(existing, template)) {
            return Err(());
        }
        *current = Some(template);
        Ok(())
    }

    pub(super) fn soulpit_template(
        &self,
    ) -> Option<&'a crate::content::soulpit_activity::QualifiedSoulpitActivityTemplate> {
        *self.soulpit_template.lock().ok()?
    }
    pub(super) fn gc_rotten_area_caches(&self, world: [u8; 16], now_us: u64) {
        let Ok(mut caches) = self.rotten_area_caches.lock() else {
            return;
        };
        caches.retain(|key, cache| {
            if key.0 != world {
                return true;
            }
            let Ok(mut current) = cache.lock() else {
                return true;
            };
            if current.busy.is_some() {
                return true;
            }
            current.data.gc(now_us);
            !current.data.empty() || Arc::strong_count(cache) > 1
        });
    }
    pub(super) fn rotten_area_cache(
        &self,
        key: super::monster_profile_cycle::rotten_area::CacheKey,
    ) -> Option<Arc<Mutex<super::monster_profile_cycle::rotten_area::SharedCache>>> {
        let mut caches = self.rotten_area_caches.lock().ok()?;
        Some(Arc::clone(caches.entry(key).or_default()))
    }
    pub(super) fn register_source_room(
        &self,
        source: crate::content::bosses::activity_template::PinnedActivitySourceRoom,
        room: &'a crate::content::QualifiedNativeEntryRoom,
    ) -> Result<(), ()> {
        let mut rooms = self.source_rooms.lock().map_err(|_| ())?;
        if let Some(previous) = rooms.get(source.name()) {
            return std::ptr::eq(*previous, room).then_some(()).ok_or(());
        }
        rooms.insert(source.name(), room);
        Ok(())
    }

    pub(super) fn source_room(
        &self,
        source: crate::content::bosses::activity_template::PinnedActivitySourceRoom,
    ) -> Option<&'a crate::content::QualifiedNativeEntryRoom> {
        self.source_rooms.lock().ok()?.get(source.name()).copied()
    }

    /// A new mutable Map owner for a fresh Instance, rebuilt only from the sealed
    /// donor capture and the current compatible Item/Content owner. No allocation
    /// or placement grant is issued by this factual factory.
    pub(super) fn fresh_source_room(
        &self,
        source: crate::content::bosses::activity_template::PinnedActivitySourceRoom,
        active: &crate::content::ActiveGeneration,
        scope: RuntimeScopeRefV1,
    ) -> Result<Arc<crate::content::QualifiedNativeEntryRoom>, ()> {
        if !matches!(scope, RuntimeScopeRefV1::Instance { .. }) {
            return Err(());
        }
        let template = self.source_room(source).ok_or(())?;
        let source_room = active
            .qualify_activity_source_room(template, source)
            .map_err(|_| ())?;
        source_bundle::qualify_for_scope(&source_room, source, active, scope).map(Arc::new)
    }

    pub(super) fn register_template(
        &self,
        template: &'a crate::content::bosses::activity_template::QualifiedActivityRoomTemplate,
    ) -> Result<(), ()> {
        let mut templates = self.templates.lock().map_err(|_| ())?;
        if let Some(existing) = templates.get(template.template_key()) {
            return if std::ptr::eq(*existing, template) {
                Ok(())
            } else {
                Err(())
            };
        }
        templates.insert(template.template_key().into(), template);
        Ok(())
    }
    pub(super) fn template(
        &self,
        key: &str,
        revision: &str,
    ) -> Option<&'a crate::content::bosses::activity_template::QualifiedActivityRoomTemplate> {
        let template = *self.templates.lock().ok()?.get(key)?;
        (template.template_revision() == revision).then_some(template)
    }

    pub(super) fn get(
        &self,
        scope: RuntimeScopeRefV1,
        generation: u64,
    ) -> Option<RegisteredRuntime<'a>> {
        self.runtimes
            .lock()
            .ok()?
            .get(&ScopeKey::new(scope, generation))
            .cloned()
    }

    fn insert(&self, owner: RegisteredRuntime<'a>) -> Result<(), ()> {
        let key = ScopeKey::new(owner.scope, owner.generation);
        let mut owners = self.runtimes.lock().map_err(|_| ())?;
        if let Some(existing) = owners.get(&key) {
            return if std::ptr::eq(&*existing.runtime, &*owner.runtime)
                && Arc::ptr_eq(&existing.states, &owner.states)
                && existing.template == owner.template
            {
                Ok(())
            } else {
                Err(())
            };
        }
        // A new ownership generation needs a separately retired old runtime;
        // registration never silently swaps a live owner under its sessions.
        if owners
            .values()
            .any(|existing| existing.scope == owner.scope)
        {
            return Err(());
        }
        owners.try_reserve(1).map_err(|_| ())?;
        owners.insert(key, owner);
        self.changed.notify_waiters();
        Ok(())
    }

    fn retire_and_remove(
        &self,
        owner: &RegisteredRuntime<'a>,
        runtime: &mut crate::foundation::ChannelRuntimeV1,
    ) -> Result<(), ()> {
        let key = ScopeKey::new(owner.scope, owner.generation);
        let mut owners = self.runtimes.lock().map_err(|_| ())?;
        let Some(current) = owners.get(&key) else {
            return Ok(());
        };
        if current.template.is_none()
            || !std::ptr::eq(&*current.runtime, &*owner.runtime)
            || !runtime.activity_retirement_empty().map_err(|_| ())?
        {
            return Err(());
        }
        // No cancellation point or fallible acquisition separates invalidating
        // the native owner from removing its exact registry entry.
        runtime.retire_owner_cycle();
        owners.remove(&key);
        self.changed.notify_waiters();
        Ok(())
    }

    pub(super) fn activity_owners(&self) -> Result<Vec<RegisteredRuntime<'a>>, ()> {
        let owners = self.runtimes.lock().map_err(|_| ())?;
        Ok(owners
            .values()
            .filter(|owner| owner.template.is_some())
            .cloned()
            .collect())
    }

    pub(super) fn remember(&self, admitted: AdmittedSession) -> Result<(), ()> {
        let (Some(actor), Some(fence), Some(controller)) = (
            admitted.runtime_actor,
            admitted.item_fence,
            admitted.controller,
        ) else {
            return Ok(());
        };
        if actor.runtime_scope() != fence.runtime_scope
            || actor.scope_generation() != fence.scope_ownership_generation
        {
            // House custody retains its separate existing carrier/transport path.
            return Ok(());
        }
        if matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
            && !admitted
                .continuity
                .selected_capabilities
                .contains(oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1)
        {
            // A Channel connection without the activity capability cannot join a
            // handoff. Its existing command owner retains its own typed refusals.
            return Ok(());
        }
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return Err(());
        }
        let mut sessions = self.connections.lock().map_err(|_| ())?;
        if let Some(old) = sessions.get(&admitted.game_session_id)
            && (old.continuity.connection_generation > admitted.continuity.connection_generation
                || (old.continuity.connection_generation
                    == admitted.continuity.connection_generation
                    && old
                        .controller
                        .is_some_and(|old| old.transport != controller.transport)))
        {
            return Err(());
        }
        let mut accounts = self.offline_accounts.lock().map_err(|_| ())?;
        sessions.try_reserve(1).map_err(|_| ())?;
        sessions.insert(admitted.game_session_id, admitted);
        accounts.remove(&admitted.game_session_id);
        Ok(())
    }

    /// This follows only this process's exact controlled connection chain. A
    /// later reconnect has another transport/generation and cannot be selected
    /// for control loss by an obsolete socket's cancellation.
    pub(super) fn current_connection(&self, source: AdmittedSession) -> AdmittedSession {
        let Ok(successors) = self.successors.lock() else {
            return source;
        };
        current_connection(&successors, source)
    }

    /// An already committed offline return has no transport and emits no wire
    /// barrier. Retain its exact predecessor so the old grace lifecycle follows
    /// the new owner without selecting a later reconnect generation.
    pub(super) fn remember_offline_return(
        &self,
        source: AdmittedSession,
        next: AdmittedSession,
    ) -> Result<(), ()> {
        let (Some(old_fence), Some(new_fence), Some(actor)) =
            (source.item_fence, next.item_fence, next.runtime_actor)
        else {
            return Err(());
        };
        if next.controller.is_some()
            || source.game_session_id == next.game_session_id
            || old_fence.character_id != new_fence.character_id
            || old_fence.game_session_id != source.game_session_id
            || new_fence.game_session_id != next.game_session_id
            || old_fence.runtime_scope == new_fence.runtime_scope
            || old_fence.runtime_scope.world_id() != new_fence.runtime_scope.world_id()
            || old_fence.character_lease_generation.checked_add(1)
                != Some(new_fence.character_lease_generation)
            || new_fence.connection_generation.get() != 1
            || next.continuity.connection_generation != 1
            || actor.runtime_scope() != new_fence.runtime_scope
            || actor.scope_generation() != new_fence.scope_ownership_generation
            || source.continuity.selected_capabilities != next.continuity.selected_capabilities
        {
            return Err(());
        }
        let mut pending = self.pending.lock().map_err(|_| ())?;
        let mut successors = self.successors.lock().map_err(|_| ())?;
        let mut sessions = self.connections.lock().map_err(|_| ())?;
        let Some(registered) = sessions.get(&source.game_session_id) else {
            // A completed attempt is published repeatedly until room retirement.
            // Only its first publication has a registered source; a later replay
            // must not recreate removed aliases, offline state or playable control.
            return Ok(());
        };
        if !same_connection(*registered, source) {
            return Err(());
        }
        if let Some(lineage) = successors.get(&source.game_session_id) {
            return if same_connection(lineage.source, source) && lineage.destination == next {
                Ok(())
            } else {
                Err(())
            };
        }
        if sessions.contains_key(&next.game_session_id) {
            return Err(());
        }
        let mut accounts = self.offline_accounts.lock().map_err(|_| ())?;
        let account = match source.controller {
            Some(controller) => controller.account_id,
            None => accounts
                .get(&source.game_session_id)
                .filter(|(fence, _)| Some(*fence) == source.item_fence)
                .map(|(_, account)| *account)
                .ok_or(())?,
        };
        successors.try_reserve(1).map_err(|_| ())?;
        sessions.try_reserve(1).map_err(|_| ())?;
        accounts.try_reserve(1).map_err(|_| ())?;
        advance_lineage(&mut successors, source, next);
        // The existing loss/grace loop may retain this source snapshot after
        // its transport ended. Keep that lifecycle root until its proven end.
        successors.insert(
            source.game_session_id,
            ActivityConnectionLineage {
                source,
                destination: next,
                root: true,
            },
        );
        sessions.remove(&source.game_session_id);
        sessions.insert(next.game_session_id, next);
        accounts.remove(&source.game_session_id);
        accounts.insert(next.game_session_id, (new_fence, account));
        pending.remove(&source.game_session_id);
        prune_delivered_lineage(&mut successors, &pending);
        drop(accounts);
        drop(sessions);
        drop(successors);
        drop(pending);
        self.forget_source_commands(source);
        self.changed.notify_waiters();
        Ok(())
    }

    /// Account identity is retained for the exact current offline successor,
    /// independently of playable control and of historical wire aliases.
    pub(super) fn account_for(&self, admitted: AdmittedSession) -> Option<[u8; 16]> {
        if let Some(controller) = admitted.controller {
            return Some(controller.account_id);
        }
        let accounts = self.offline_accounts.lock().ok()?;
        let (fence, account) = accounts.get(&admitted.game_session_id)?;
        (Some(*fence) == admitted.item_fence).then_some(*account)
    }

    /// Called synchronously after the existing terminal owner removed this exact
    /// actor. A mere disconnect, handoff receipt or failed release never gets here.
    pub(super) fn forget_terminal_connection(&self, session: GameSessionId, actor: ExactActorRef) {
        if let Ok(mut sessions) = self.connections.lock()
            && sessions
                .get(&session)
                .is_some_and(|held| held.runtime_actor == Some(actor))
        {
            sessions.remove(&session);
            if let Ok(mut accounts) = self.offline_accounts.lock() {
                accounts.remove(&session);
            }
        }
        // The listener may still hold an earlier snapshot while reconciling
        // this terminal result. Its completion hook, not actor removal alone,
        // releases the last exact socket/lifecycle alias and pending barrier.
    }

    /// Forget ingress only after the old session's committed handoff. An open
    /// source stream owns its Arc independently and can finish its result; the
    /// directory cannot retain one obsolete stream for every past room visit.
    fn forget_source_commands(&self, source: AdmittedSession) {
        let Some(fence) = source.item_fence else {
            return;
        };
        if let Some(owner) = self.get(fence.runtime_scope, fence.scope_ownership_generation.get()) {
            owner.states.command_ingress.forget(source.game_session_id);
        }
    }

    /// The listener calls this only once no more socket/loss/grace callbacks can
    /// reference this exact chain. Unknown operations and cancelled lifecycles
    /// do not call it. Durable rows, current connections and owner tokens stay.
    pub(super) fn finish_connection_lineage(&self, ended: AdmittedSession) -> Result<(), ()> {
        let mut pending = self.pending.lock().map_err(|_| ())?;
        let mut successors = self.successors.lock().map_err(|_| ())?;
        let mut sessions = self.connections.lock().map_err(|_| ())?;
        let target = current_connection(&successors, ended);
        let mut old_sources = Vec::new();
        old_sources.try_reserve(pending.len()).map_err(|_| ())?;
        pending.retain(|_, switch| {
            if same_connection(current_connection(&successors, switch.source), target) {
                if sessions
                    .get(&switch.source.game_session_id)
                    .is_some_and(|held| same_connection(*held, switch.source))
                {
                    sessions.remove(&switch.source.game_session_id);
                }
                old_sources.push(switch.source);
                false
            } else {
                true
            }
        });
        successors.retain(|_, lineage| !same_connection(lineage.destination, target));
        prune_delivered_lineage(&mut successors, &pending);
        drop(sessions);
        drop(successors);
        drop(pending);
        for source in old_sources {
            self.forget_source_commands(source);
        }
        Ok(())
    }

    pub(super) fn connection(&self, session: GameSessionId) -> Option<AdmittedSession> {
        self.connections.lock().ok()?.get(&session).copied()
    }

    pub(super) fn stage_switch(&self, switch: ActivityConnectionSwitch) -> Result<(), ()> {
        let source = switch.source;
        let next = switch.destination;
        let (Some(old_controller), Some(new_controller), Some(old_fence), Some(new_fence)) = (
            source.controller,
            next.controller,
            source.item_fence,
            next.item_fence,
        ) else {
            return Err(());
        };
        if source.game_session_id == next.game_session_id
            || old_controller.account_id != new_controller.account_id
            || old_controller.transport == new_controller.transport
            || old_fence.character_id != new_fence.character_id
            || old_fence.runtime_scope == new_fence.runtime_scope
            || old_fence.character_lease_generation.checked_add(1)
                != Some(new_fence.character_lease_generation)
            || new_fence.game_session_id != next.game_session_id
            || new_fence.connection_generation.get() != next.continuity.connection_generation
            || next.continuity.next_command_id != 1
            || next.continuity.server_sequence != 0
            || next.continuity.selected_capabilities != source.continuity.selected_capabilities
            || next.runtime_actor.is_none_or(|actor| {
                actor.runtime_scope() != new_fence.runtime_scope
                    || actor.scope_generation() != new_fence.scope_ownership_generation
            })
        {
            return Err(());
        }
        let mut pending = self.pending.lock().map_err(|_| ())?;
        if let Some(old) = pending.get(&source.game_session_id) {
            return if old.source == source
                && old.destination == next
                && old.payload == switch.payload
            {
                Ok(())
            } else {
                Err(())
            };
        }
        let mut successors = self.successors.lock().map_err(|_| ())?;
        let mut sessions = self.connections.lock().map_err(|_| ())?;
        let Some(registered) = sessions.get(&source.game_session_id) else {
            // A retained committed result can outlive its delivered/pruned
            // source. It is already installed, never a fresh publication grant.
            return Ok(());
        };
        if !same_connection(*registered, source) {
            return Err(());
        }
        if let Some(lineage) = successors.get(&source.game_session_id) {
            return if same_connection(lineage.source, source) && lineage.destination == next {
                Ok(())
            } else {
                Err(())
            };
        }
        if sessions
            .get(&next.game_session_id)
            .is_some_and(|held| !same_connection(*held, next))
        {
            return Err(());
        }
        pending.try_reserve(1).map_err(|_| ())?;
        successors.try_reserve(1).map_err(|_| ())?;
        sessions.try_reserve(1).map_err(|_| ())?;
        // SQL and physical installation already committed this destination.
        // Keep the original physical socket root, but compress delivered hops
        // instead of retaining two AdmittedSession values for every past visit.
        let continued = advance_lineage(&mut successors, source, next);
        sessions.insert(next.game_session_id, next);
        successors.insert(
            source.game_session_id,
            ActivityConnectionLineage {
                source,
                destination: next,
                root: !continued,
            },
        );
        pending.insert(source.game_session_id, switch);
        prune_delivered_lineage(&mut successors, &pending);
        self.changed.notify_waiters();
        Ok(())
    }

    pub(super) fn pending_switch(
        &self,
        source: AdmittedSession,
    ) -> Result<Option<ActivityConnectionSwitch>, ()> {
        let pending = self.pending.lock().map_err(|_| ())?;
        let Some(switch) = pending.get(&source.game_session_id) else {
            return Ok(None);
        };
        if switch.source.controller != source.controller
            || switch.source.item_fence != source.item_fence
            || switch.source.runtime_actor != source.runtime_actor
            || switch.source.continuity.connection_generation
                != source.continuity.connection_generation
        {
            return Err(());
        }
        Ok(Some(switch.clone()))
    }

    pub(super) fn finish_switch(
        &self,
        source: AdmittedSession,
        destination: AdmittedSession,
    ) -> Result<(), ()> {
        let mut pending = self.pending.lock().map_err(|_| ())?;
        let current = pending.get(&source.game_session_id).ok_or(())?;
        if !same_connection(current.source, source) || current.destination != destination {
            return Err(());
        }
        let mut successors = self.successors.lock().map_err(|_| ())?;
        let mut sessions = self.connections.lock().map_err(|_| ())?;
        // First publication made the actual destination visible. Repeating
        // remember here could downgrade a reconnect or resurrect an actor that
        // already returned/terminated while this earlier barrier was in flight.
        pending.remove(&source.game_session_id);
        if sessions
            .get(&source.game_session_id)
            .is_some_and(|held| same_connection(*held, source))
        {
            sessions.remove(&source.game_session_id);
        }
        prune_delivered_lineage(&mut successors, &pending);
        drop(sessions);
        drop(successors);
        drop(pending);
        self.forget_source_commands(source);
        Ok(())
    }
}
