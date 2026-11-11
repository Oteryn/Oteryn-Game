//! Resolve the authored remote object under the same Channel owner as the offering.
use super::*;
use crate::map::facts::QuestCallbackProjection;
use oteryn_gameplay_quests::quest::world_callbacks::TargetTransform;

impl PendingCallbacks {
    /// An uncertain durable commit reserves both objects until its original cause settles.
    pub(in crate::gameplay_transport) fn source_target_available(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        cause: &QuestCause,
        key: u64,
        target: u64,
    ) -> bool {
        self.records.lock().is_ok_and(|records| {
            !records.values().any(|pending| {
                !(pending.actor == actor && pending.session == session && pending.cause == *cause)
                    && (pending.key == key
                        || pending.key == target
                        || pending
                            .projection_target
                            .is_some_and(|other| other == key || other == target))
            })
        })
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Choose disjoint source trials only as advice; SQL rechecks their full snapshot.
    pub(super) async fn select_source_phase(
        &self,
        selected: &mut Vec<&Callback>,
        actor: ExactActorRef,
        command: &UseCommand,
    ) {
        if selected.len() <= 1
            || selected
                .iter()
                .any(|c| c.effects.target_transform.is_none())
            || selected.iter().enumerate().any(|(i, a)| {
                selected[..i]
                    .iter()
                    .any(|b| !world_callbacks::phase::exclusive(a, b))
            })
        {
            return;
        }
        let Ok(id) = CommandId::new(command.command_id) else {
            selected.clear();
            return;
        };
        let cause = QuestCause::Use(CommandRef::new(command.game_session_id, id));
        if let Ok(records) = self.states.quest_callbacks.records.lock()
            && let Some(pending) = records.get(&command.game_session_id)
            && pending.actor == actor
            && pending.cause == cause
        {
            selected.retain(|callback| callback.key == pending.callback);
            return;
        }
        self.refresh_quest_session(command.game_session_id, true)
            .await;
        let Ok(sessions) = self.states.quest_sessions.lock() else {
            selected.clear();
            return;
        };
        let Some(copy) = sessions
            .get(&command.game_session_id)
            .and_then(|s| s.copy.as_ref())
        else {
            selected.clear();
            return;
        };
        selected.retain(|callback| {
            callback.storage.iter().all(|guard| {
                copy.tracks()
                    .get(&guard.track)
                    .copied()
                    .or_else(|| {
                        self.quest_catalogue
                            .as_ref()?
                            .track(&guard.track)
                            .map(|t| t.initial)
                    })
                    .is_some_and(|value| (guard.minimum..=guard.maximum).contains(&value))
            })
        });
    }
}

pub(super) fn prepare(
    map: &BundleMap,
    target: &TargetTransform,
) -> Result<QuestCallbackProjection, UseDisposition> {
    let at = tile(target.position);
    if map.facts.house_tile(at) {
        return Err(UseDisposition::Rejected);
    }
    let base = map
        .overlay
        .base()
        .tile(at.x, at.y, at.floor)
        .ok_or(UseDisposition::Rejected)?;
    let mut candidates = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .filter_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            let key = placement_key(at, ordinal)?;
            let before = map.facts.base_entry(at, ordinal, *id)?;
            (!map.facts.base_hidden(key)
                && before.appearance_id == target.from
                && !before.pickupable)
                .then_some((key, before))
        });
    let (key, restore) = candidates.next().ok_or(UseDisposition::NothingToUse)?;
    if candidates.next().is_some() {
        return Err(UseDisposition::Rejected);
    }
    let expected = map.facts.object_revision(key);
    if !map.facts.quest_callback_capacity(key, expected, 0) {
        return Err(UseDisposition::Exhausted);
    }
    let after = map
        .facts
        .quest_callback_replacement(key, target.to)
        .filter(|facts| !facts.pickupable)
        .ok_or(UseDisposition::Rejected)?;
    Ok(QuestCallbackProjection::TimedTargetTransform {
        key,
        expected,
        after,
        restore,
        due: 0,
    })
}
