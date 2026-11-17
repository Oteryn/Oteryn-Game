use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in super::super) async fn finish_staff_world_action(
        &self,
        admitted: connection::AdmittedSession,
        principal: &str,
        request: &rules::Request,
        issuer_revision: u64,
    ) -> Result<String, ()> {
        let fence = crate::gameplay_transport::character_gameplay_fence::current(
            self,
            admitted.game_session_id,
            false,
        )
        .await
        .ok_or(())?
        .item_fence();
        if fence.connection_generation.get() != admitted.continuity.connection_generation {
            return Err(());
        }
        let state = self
            .root
            .staff_world_effect_state(self.character, self.holder, fence, &request.operation_id)
            .await
            .map_err(|error| {
                #[cfg(test)]
                eprintln!("NX1 staff_world_effect_state failed: {error:?}");
                #[cfg(not(test))]
                let _ = error;
            })?;
        match state {
            Some(StaffWorldEffectState::Applied) => {
                self.states
                    .server_messages
                    .staff_forget(&request.operation_id);
                self.states
                    .staff_world
                    .receipts
                    .lock()
                    .map_err(|_| ())?
                    .remove(&request.operation_id);
                return Ok("World action applied".into());
            }
            Some(StaffWorldEffectState::Refused) => {
                self.states
                    .server_messages
                    .staff_forget(&request.operation_id);
                self.states
                    .staff_world
                    .receipts
                    .lock()
                    .map_err(|_| ())?
                    .remove(&request.operation_id);
                return Ok("World action refused".into());
            }
            None => {
                if matches!(request.action, rules::Action::Inspect { .. }) {
                    let _ = self.refresh_staff(admitted).await;
                    let summary = self
                        .root
                        .staff_inspect_summary(
                            self.character,
                            self.holder,
                            fence,
                            principal,
                            request.action.character_id(),
                        )
                        .await
                        .map_err(|error| {
                            #[cfg(test)]
                            eprintln!("NX1 staff_inspect_summary failed: {error:?}");
                            #[cfg(not(test))]
                            let _ = error;
                        })?;
                    if let Some(proof) = self.staff_actor(request.action.character_id()).await {
                        let runtime = self.runtime.lock().await;
                        if !proof.current(&runtime) {
                            #[cfg(test)]
                            eprintln!(
                                "NX1 inspect proof stale lease={:?} revision={:?}",
                                runtime.bound_attacker_lease(proof.actor, proof.session),
                                runtime.player_control_revision(proof.actor, proof.session)
                            );
                            return Err(());
                        }
                        let actor = proof.actor;
                        let at = runtime
                            .read_actor_position(actor)
                            .map_err(|_| ())?
                            .position();
                        return Ok(format!(
                            "{summary}; position {},{},{}; group {}",
                            at.x,
                            at.y,
                            at.floor,
                            self.states.staff_world.group(actor)
                        ));
                    }
                    return Ok(format!("{summary}; offline"));
                }
                return Ok("Audited action applied".into());
            }
            Some(StaffWorldEffectState::Pending) => {}
        }
        let minimum = match request.action.minimum_role() {
            rules::StaffRole::Gm => 4,
            rules::StaffRole::SeniorGm => 5,
            rules::StaffRole::Admin => 6,
        };
        let authorized = self
            .root
            .staff_tools_actor_authorized(self.character, self.holder, fence, principal, minimum)
            .await
            .map_err(|_| ())?;
        let retained = self
            .states
            .staff_world
            .receipts
            .lock()
            .map_err(|_| ())?
            .get(&request.operation_id)
            .copied();
        let applied = if let Some(applied) = retained {
            applied
        } else if authorized {
            let observed = self.staff_runtime_snapshot().await.ok_or(())?;
            let issuer = observed
                .iter()
                .find(|p| {
                    Some(p.actor) == admitted.runtime_actor
                        && p.session == admitted.game_session_id
                        && p.control_revision == issuer_revision
                })
                .copied()
                .ok_or(())?;
            let pinned = self
                .root
                .staff_world_effect_target(
                    self.character,
                    self.holder,
                    fence,
                    &request.operation_id,
                )
                .await
                .map_err(|_| ())?;
            let target = match &pinned {
                Some(capture) => {
                    self.staff_actor_from_snapshot(
                        request.action.character_id(),
                        Some(capture),
                        &observed,
                    )
                    .await
                }
                None => None,
            };
            let destination = match &request.action {
                rules::Action::TeleportToPlayer {
                    destination_character_id,
                    ..
                } => {
                    self.staff_actor_from_snapshot(destination_character_id, None, &observed)
                        .await
                }
                rules::Action::Bring { .. } => Some(issuer),
                _ => None,
            };
            if !self
                .root
                .staff_tools_actor_authorized(
                    self.character,
                    self.holder,
                    fence,
                    principal,
                    minimum,
                )
                .await
                .map_err(|_| ())?
            {
                self.root
                    .ack_staff_world_effect(
                        self.character,
                        self.holder,
                        fence,
                        &request.operation_id,
                        false,
                    )
                    .await
                    .map_err(|_| ())?;
                return Ok("World action refused: staff permission revoked".into());
            }
            let mut runtime = self.runtime.lock().await;
            let mut receipts = self.states.staff_world.receipts.lock().map_err(|_| ())?;
            if let Some(previous) = receipts.get(&request.operation_id) {
                *previous
            } else {
                if receipts.len() >= 1024 {
                    return Err(());
                }
                let applied =
                    self.apply_staff_world(&mut runtime, issuer, request, target, destination);
                receipts.insert(request.operation_id.clone(), applied);
                applied
            }
        } else {
            false
        };
        self.root
            .ack_staff_world_effect(
                self.character,
                self.holder,
                fence,
                &request.operation_id,
                applied,
            )
            .await
            .map_err(|_| ())?;
        self.states
            .server_messages
            .staff_forget(&request.operation_id);
        self.states
            .staff_world
            .receipts
            .lock()
            .map_err(|_| ())?
            .remove(&request.operation_id);
        let _ = self.refresh_staff(admitted).await;
        Ok(if applied {
            "World action applied"
        } else {
            "World action refused"
        }
        .into())
    }
}
