//! One current shared World observation under the existing frozen movement turn.
use super::*;

#[cfg(test)]
tokio::task_local! {
    pub(in crate::gameplay_transport) static TIMED_GATE_READ: std::sync::Arc<(tokio::sync::Notify, tokio::sync::Notify)>;
}

pub(super) enum GateObservation {
    Denied,
    PersonalCompletion,
    Timed(u64),
}

impl GateObservation {
    pub(super) fn allows(self, now: u64) -> bool {
        match self {
            Self::Denied => false,
            Self::PersonalCompletion => true,
            Self::Timed(deadline) => now < deadline,
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn admit_shared_switch_gate(
        &self,
        permit: &SpellLanePermit,
        artifact: [u8; 32],
        placement_context: Vec<u8>,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        revision: u64,
        cause: QuestCause,
        fence: Option<CurrentCharacterGameplayFence>,
        expected: MovementPositionSnapshot,
    ) -> GateObservation {
        let Some(gate) = oteryn_gameplay_quests::quest::shared_switches::gate(&callback.key) else {
            return GateObservation::Denied;
        };
        let (Some(fence), QuestCause::Step(_), Some(native)) = (
            fence,
            cause,
            self.active_generation.and_then(|g| g.native_gameplay()),
        ) else {
            return GateObservation::Denied;
        };
        let context = QuestWorldContext {
            map_revision: crate::map::overlay::map_revision(map.overlay.base()),
            content_revision: format!("sha256:{}", hex(&artifact)),
            placement_context,
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
                paid_arrivals: Vec::new(),
                flame: None,
                ingress_settled: false,
                participants: Vec::new(),
                projection_target: None,
                flame_effect_sent: false,
                selected_tool: None,
            });
            pending.context.get_or_insert(context).clone()
        } else {
            return GateObservation::Denied;
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
            Ok(Some(observed)) if observed.personal_completion => {
                Some(GateObservation::PersonalCompletion)
            }
            Ok(Some(observed)) => {
                if observed.count < gate.required {
                    Some(GateObservation::Denied)
                } else {
                    observed
                        .next_change_unix_ms
                        .and_then(|due| due.checked_sub(observed.observed_at_unix_ms))
                        .and_then(|ms| u64::try_from(ms).ok())
                        .and_then(|ms| ms.checked_mul(1_000))
                        .and_then(|us| started.checked_add(us))
                        .filter(|deadline| *deadline > self.owner_now().get())
                        .map(GateObservation::Timed)
                }
            }
            Ok(None) => Some(GateObservation::Denied),
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
            #[cfg(test)]
            if matches!(allowed, GateObservation::Timed(_))
                && let Ok(barrier) = TIMED_GATE_READ.try_with(std::sync::Arc::clone)
            {
                barrier.0.notify_one();
                barrier.1.notified().await;
            }
            allowed
        } else {
            GateObservation::Denied
        }
    }
}
