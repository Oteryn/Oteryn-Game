use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn save_familiar_at_actor_end(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        logout: Option<(AuthenticatedTransportRefV1, u64)>,
    ) -> actor_spell::FamiliarLogoutSave {
        // LOGOUT-WIRE-1 (#2127 Codex P1 4278730847): an ACTIVE session's own logout ends the
        // actor too; the familiar is saved on the exact transport before the terminal release.
        if let Some((transport, attempt)) = logout {
            return self
                .save_familiar_logout(
                    actor,
                    session,
                    actor_spell::FamiliarActorEnd::Logout(transport, attempt),
                )
                .await;
        }
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let Ok((current, now)) = store.current_session_at(session).await else {
            return actor_spell::FamiliarLogoutSave::Unknown;
        };
        if current.session_state() != GameSessionState::Reconnectable {
            return actor_spell::FamiliarLogoutSave::NotApplicable;
        }
        let Some(deadline) = current.current_original_grace_deadline() else {
            return actor_spell::FamiliarLogoutSave::Unknown;
        };
        if now < deadline {
            return actor_spell::FamiliarLogoutSave::NotApplicable;
        }
        let Some(epoch) = current.current_control_loss_epoch() else {
            return actor_spell::FamiliarLogoutSave::Unknown;
        };
        self.save_familiar_logout(
            actor,
            session,
            actor_spell::FamiliarActorEnd::GraceExpiry(epoch.get()),
        )
        .await
    }
}
