use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn staff_runtime_snapshot(&self) -> Option<Vec<StaffActorProof>> {
        let runtime = self.runtime.lock().await;
        Some(
            runtime
                .positioned_actor_census()
                .ok()?
                .into_iter()
                .filter_map(|(actor, _, session)| {
                    let session = session?;
                    let control_revision = runtime.player_control_revision(actor, session).ok()?;
                    runtime
                        .player_control_facts(actor, session)
                        .ok()?
                        .control_loss
                        .is_none()
                        .then_some(StaffActorProof {
                            actor,
                            session,
                            control_revision,
                        })
                })
                .collect(),
        )
    }
    pub(super) async fn staff_actor(&self, character: &str) -> Option<StaffActorProof> {
        let observed = self.staff_runtime_snapshot().await?;
        self.staff_actor_from_snapshot(character, None, &observed)
            .await
    }
    pub(super) async fn staff_actor_from_snapshot(
        &self,
        character: &str,
        pinned: Option<&crate::durability::staff::StaffWorldEffectTarget>,
        observed: &[StaffActorProof],
    ) -> Option<StaffActorProof> {
        let sessions = observed.iter().map(|p| p.session).collect();
        let snapshots =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(self.root.clone())
                .current_sessions_at(sessions)
                .await
                .ok()?;
        for (proof, snapshot) in observed.iter().zip(snapshots) {
            let Some(snapshot) = snapshot else {
                continue;
            };
            if snapshot.session_state() != crate::foundation::GameSessionState::Active {
                continue;
            }
            if pinned.is_some_and(|p| {
                p.session != proof.session
                    || p.connection_generation != snapshot.current_connection_generation().get()
            }) {
                continue;
            }
            let id = snapshot
                .commit()
                .character_id()
                .as_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            if id == character {
                return Some(*proof);
            }
        }
        None
    }
}
