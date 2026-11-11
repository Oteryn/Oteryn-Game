//! One current shared World observation under the existing frozen movement turn.
use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn admit_shared_switch_gate(
        &self,
        permit: &SpellLanePermit,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        revision: u64,
        cause: QuestCause,
        fence: Option<CurrentCharacterGameplayFence>,
        expected: MovementPositionSnapshot,
    ) -> bool {
        let Some(gate) = oteryn_gameplay_quests::quest::shared_switches::gate(&callback.key) else {
            return true;
        };
        let (Some(fence), QuestCause::Step(_), Some(native)) = (
            fence,
            cause,
            self.active_generation.and_then(|g| g.native_gameplay()),
        ) else {
            return false;
        };
        let context = QuestWorldContext {
            map_revision: crate::map::overlay::map_revision(map.overlay.base()),
            content_revision: format!(
                "sha256:{}",
                hex(&runtime.content_pin().server_artifact_digest())
            ),
            placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
            reset_epoch: self.reset_epoch,
            actor_placement: actor.placement_identity(),
            movement_revision: expected.revision(),
            party: Vec::new(),
        };
        let context = if let Ok(mut records) = self.states.quest_callbacks.records.lock() {
            let pending = records.entry(session).or_insert(Pending {
                actor,
                session,
                expected,
                callback: callback.key.clone(),
                key,
                revision,
                cause,
                origin: None,
                context: None,
                flame: None,
                ingress_settled: false,
                participants: Vec::new(),
                projection_target: None,
                flame_effect_sent: false,
            });
            pending.context.get_or_insert(context).clone()
        } else {
            return false;
        };
        let started = self.owner_now().get();
        let result = self
            .root
            .read_shared_switch_count(
                permit,
                self.character,
                self.holder,
                fence,
                native,
                crate::durability::quest_world_callback::shared_count::Request {
                    gate: callback.key.clone(),
                    context,
                },
            )
            .await;
        let decision = match result {
            // The optional flag is a canonical one-way completion fact observed
            // for this actor in the same fenced statement snapshot as the count.
            Ok(Some(observed)) if observed.personal_completion => Some(true),
            Ok(Some(observed)) => {
                let elapsed = self.owner_now().get().saturating_sub(started);
                // Anchor the bridge before the RPC, conservatively subtracting all
                // service time. A possibly expired positive read remains pending.
                if observed.count >= gate.required
                    && observed.next_change_unix_ms.is_none_or(|due| {
                        due.saturating_sub(observed.observed_at_unix_ms)
                            .saturating_mul(1000)
                            <= i64::try_from(elapsed).unwrap_or(i64::MAX)
                    })
                {
                    None
                } else {
                    Some(observed.count >= gate.required)
                }
            }
            Ok(None) => Some(false),
            Err(error) => {
                #[cfg(test)]
                eprintln!("SHARED_GATE read error: {error:?}");
                #[cfg(not(test))]
                let _ = error;
                None
            }
        };
        if let Some(allowed) = decision {
            if let Ok(mut records) = self.states.quest_callbacks.records.lock() {
                records.remove(&session);
            }
            allowed
        } else {
            false
        }
    }
}
