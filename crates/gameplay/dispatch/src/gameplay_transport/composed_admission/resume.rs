use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn resume_session(
        &self,
        attempt: connection::ResumeAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        let budget = AdmissionRetryBudget::for_root(&attempt.game_session_id, self.root);
        if let Some(admitted) = self.recover_activity_session(&budget, &attempt).await? {
            return Ok(admitted);
        }
        // Directory lookup selects an owner only. The original recovery owner
        // below still verifies the credential, exact lost epoch, transport,
        // current guards, attempt budget and generation before any mutation.
        let scoped =
            if let Some(registered) = self.activity_runtimes.connection(attempt.game_session_id) {
                self.resolve_runtime_view(registered)
                    .await
                    .map_err(|_| AdmissionRefusal::Unavailable)?
            } else {
                None
            };
        let owner = scoped.as_ref().unwrap_or(self);
        let admitted = owner.resume_session_for_scope(attempt).await?;
        self.activity_runtimes
            .remember(admitted)
            .map_err(|_| AdmissionRefusal::Unavailable)?;
        Ok(admitted)
    }

    async fn resume_session_for_scope(
        &self,
        attempt: connection::ResumeAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        // OPS-SAVE-2: no session begins while the node quiesces for its shutdown save.
        let Some(_permit) = self.states.input_gate.enter().await else {
            return Err(AdmissionRefusal::Unavailable);
        };
        // DEFLAKE-2: a step refused on a busy durability holder before the resume wrote
        // anything is repeated from fresh reads within the root recovery window.
        let budget = AdmissionRetryBudget::for_root(&attempt.game_session_id, self.root);
        let transferred = self
            .recover_transferred_house(
                &budget,
                connection::ResumeAttempt {
                    game_session_id: attempt.game_session_id,
                    recovery_material: attempt.recovery_material,
                    transport: attempt.transport,
                    last_applied_server_sequence: attempt.last_applied_server_sequence,
                    supported_capabilities: attempt.supported_capabilities,
                },
            )
            .await?;
        let recovered_transfer = transferred.is_some();
        let admitted = if let Some(admitted) = transferred {
            admitted
        } else {
            loop {
                let mut wrote = false;
                let resumed = self
                    .resume_lost(
                        &budget,
                        connection::ResumeAttempt {
                            game_session_id: attempt.game_session_id,
                            recovery_material: attempt.recovery_material,
                            transport: attempt.transport,
                            last_applied_server_sequence: attempt.last_applied_server_sequence,
                            supported_capabilities: attempt.supported_capabilities,
                        },
                        &mut wrote,
                    )
                    .await;
                match resumed {
                    Err(refusal) if !wrote && budget.retries_unavailable(refusal).await => {}
                    resumed => break resumed?,
                }
            }
        };
        self.finish_resumed_session(admitted, recovered_transfer)
            .await;
        Ok(admitted)
    }

    pub(in crate::gameplay_transport) async fn finish_resumed_session(
        &self,
        admitted: AdmittedSession,
        recovered_house_transfer: bool,
    ) {
        // ATTACK-0 §3 Reconnect: only the target is cleared; the in-fight deadline is kept.
        if let Some(actor) = admitted.runtime_actor {
            self.states
                .attack
                .lock()
                .await
                .clear_target(actor, admitted.game_session_id);
        }
        // PREM-1b: a reconnect pulls Premium again before any Premium read, without waiting.
        if let Some(controller) = admitted.controller {
            self.admit_premium(controller.account_id, admitted.game_session_id);
        }
        // Native House recovery already restored its views and retained clocks.
        if recovered_house_transfer {
            return;
        }
        let actual_house = if let Some(actor) = admitted.runtime_actor {
            self.runtime
                .lock()
                .await
                .house_actor_state(actor, admitted.game_session_id)
                .is_ok()
        } else {
            false
        };
        if actual_house {
            // CompleteReconnect has already committed and removed its lost
            // entry. Match ordinary optional observation refresh: missing
            // data cannot discard that current resumed controller.
            let _ = self.restore_house_admission_views(&admitted, None).await;
        } else {
            self.admit_quest_session(&admitted).await;
        }
    }
}
