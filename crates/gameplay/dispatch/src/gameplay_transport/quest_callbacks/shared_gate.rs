//! One current shared World observation under the existing frozen movement turn.
use super::*;

pub(super) enum GateObservation {
    Closed,
    Open,
    OpenUntil(u64),
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn admit_shared_switch_gate(
        &self,
        permit: &SpellLanePermit,
        context: QuestWorldContext,
        actor: ExactActorRef,
        session: GameSessionId,
        gate: &oteryn_gameplay_quests::quest::shared_switches::Gate,
        key: u64,
        revision: u64,
        cause: QuestCause,
        fence: Option<CurrentCharacterGameplayFence>,
        expected: MovementPositionSnapshot,
    ) -> Option<GateObservation> {
        let (Some(fence), QuestCause::Step(_), Some(native)) = (
            fence,
            cause,
            self.active_generation.and_then(|g| g.native_gameplay()),
        ) else {
            return None;
        };
        let context = if let Ok(mut records) = self.states.quest_callbacks.records.lock() {
            let pending = records.entry(session).or_insert(Pending {
                actor,
                session,
                expected,
                callback: gate.key.to_owned(),
                key,
                revision,
                cause,
                origin: None,
                context: None,
                paid_arrivals: Vec::new(),
                flame: None,
                ingress_settled: false,
                nest_spawn: None,
                participants: Vec::new(),
                projection_target: None,
                flame_effect_sent: false,
                selected_tool: None,
            });
            if pending.actor != actor
                || pending.expected != expected
                || pending.cause != cause
                || pending.key != key
                || pending.revision != revision
                || pending.callback != gate.key
            {
                return None;
            }
            pending.context.get_or_insert(context).clone()
        } else {
            return None;
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
                    gate: gate.key.to_owned(),
                    context,
                },
            )
            .await;
        match result {
            Ok(Some(observed)) if observed.personal_completion => Some(GateObservation::Open),
            Ok(Some(observed)) if observed.count >= gate.required => {
                let remaining = observed
                    .next_change_unix_ms?
                    .checked_sub(observed.observed_at_unix_ms)?;
                let due = started.checked_add(u64::try_from(remaining).ok()?.checked_mul(1000)?)?;
                Some(GateObservation::OpenUntil(due))
            }
            Ok(_) => Some(GateObservation::Closed),
            Err(_) => None,
        }
    }
}
