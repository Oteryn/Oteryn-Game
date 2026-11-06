//! Retained intentions cross the dynamic activity callback; the shared owner performs SQL.
use super::*;

#[derive(Clone, PartialEq, Eq)]
pub(in crate::gameplay_transport) struct SoulpitReturnIntent {
    pub scope: RuntimeScopeRefV1,
    pub generation: u64,
    pub session: GameSessionId,
    pub admitted: crate::gameplay_transport::connection::AdmittedSession,
    pub command: Option<u64>,
    pub exit_target: Option<oteryn_protocol_oteryn::world_object::WorldObjectTarget>,
    pub reason: ActivityReturnReason,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn queue_soulpit_return(&self, intent: SoulpitReturnIntent) -> Result<(), ()> {
        let mut pending = self
            .activity_runtimes
            .soulpit_returns
            .lock()
            .map_err(|_| ())?;
        if let Some(previous) = pending.iter().find(|previous| {
            previous.scope == intent.scope
                && previous.generation == intent.generation
                && previous.session == intent.session
        }) {
            // One retained intention per admitted participant. A new command cannot
            // replace an uncertain command's publication or grow the queue.
            let same_command = previous.command == intent.command
                && previous.exit_target == intent.exit_target
                && previous.reason == intent.reason
                && previous.admitted.runtime_actor == intent.admitted.runtime_actor
                && previous.admitted.controller == intent.admitted.controller
                && previous.admitted.item_fence == intent.admitted.item_fence;
            return if same_command || intent.command.is_none() {
                Ok(())
            } else {
                Err(())
            };
        }
        pending.push(intent);
        Ok(())
    }

    pub(in crate::gameplay_transport) async fn settle_soulpit_returns(
        &self,
        only_session: Option<GameSessionId>,
    ) -> Result<(), ()> {
        let _cycle = self.activity_runtimes.soulpit_return_cycle.lock().await;
        let pending = self
            .activity_runtimes
            .soulpit_returns
            .lock()
            .map_err(|_| ())?
            .iter()
            .filter(|intent| only_session.is_none_or(|session| session == intent.session))
            .cloned()
            .collect::<Vec<_>>();
        for intent in pending {
            let original_fence = intent.admitted.item_fence.ok_or(())?;
            let published = self.activity_runtimes.current_connection(intent.admitted);
            if published.game_session_id != intent.session
                && published.item_fence.is_some_and(|fence| {
                    fence.character_id == original_fence.character_id
                        && fence.character_lease_generation
                            > original_fence.character_lease_generation
                })
            {
                // A different shared lifecycle cause already published this exact
                // captured source's return. The exact retained lineage also covers
                // a later entry after that return; a fresh connection is no proof.
                self.activity_runtimes
                    .soulpit_returns
                    .lock()
                    .map_err(|_| ())?
                    .retain(|pending| pending != &intent);
                continue;
            }
            let registered = self
                .activity_runtimes
                .get(intent.scope, intent.generation)
                .ok_or(())?;
            let view = self.runtime_view(&registered);
            let retained = view
                .has_bound_retained_activity_return(intent.admitted, intent.command, intent.reason)
                .await?;
            if !retained {
                let run = self
                    .activity_runtimes
                    .soulpit_runs
                    .lock()
                    .map_err(|_| ())?
                    .get(&ScopeKey::new(intent.scope, intent.generation))
                    .cloned();
                if let Some(run) = run {
                    let binding = run.binding;
                    let mut owner = LiveRuntime {
                        admission: &view,
                        run,
                        survivors: Vec::new(),
                        now: self.owner_now().get(),
                    };
                    runtime::settle_lethal_bosses(&mut owner, binding)
                        .await
                        .map_err(|_| ())?;
                }
            }
            let admitted = intent.admitted;
            let result = match intent.command {
                Some(command) => {
                    view.return_native_activity(admitted, command, intent.reason)
                        .await?
                }
                None => view.return_due_activity(admitted, intent.reason).await?,
            };
            match result {
                NativeActivityOutcome::Committed(commit) => {
                    view.publish_activity_commit(&commit)?
                }
                NativeActivityOutcome::Refused(_) => {
                    // The generic owner has restored the source and retained its
                    // exact known refusal. A client command cannot block later
                    // commands through this separate intention queue forever.
                    if intent.command.is_some() {
                        self.activity_runtimes
                            .soulpit_returns
                            .lock()
                            .map_err(|_| ())?
                            .retain(|pending| pending != &intent);
                    }
                    return Err(());
                }
            }
            self.activity_runtimes
                .soulpit_returns
                .lock()
                .map_err(|_| ())?
                .retain(|pending| pending != &intent);
        }
        Ok(())
    }
}
