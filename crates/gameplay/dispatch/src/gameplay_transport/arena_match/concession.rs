//! Crystal 201189f8 movements_arena_pit.lua:1-21,70-72.
//! The map action is the native binding; the existing departure owner releases the pit.
use super::*;
use crate::foundation::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionValues,
    MovementPositionSnapshot,
};
use crate::map::view::MapFacts;
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

const ACTION: u16 = 25300;
const LOOK_TYPE: u32 = 111;
const DURATION_MS: u32 = 120_000;

/// The imported B3 forcefields have no action attribute. An authored STEP
/// counterpart supplies the same source action and its exact map-stack selector.
pub(in crate::gameplay_transport) fn owns_callback(
    callback: &oteryn_gameplay_quests::quest::world_callbacks::Callback,
) -> bool {
    use oteryn_gameplay_quests::quest::world_callbacks::{ActorKind, Trigger};

    callback.quest == "oteryn:quest.barbarian_arena_quest"
        && callback.trigger == Trigger::StepIn
        && callback.actor_kind == ActorKind::Player
        && callback.selector.action == Some(ACTION)
        && callback
            .selector
            .native
            .as_ref()
            .is_some_and(|source| source.appearance == 1949)
        && callback.effects.is_empty()
        && !callback.has_state()
        && callback.teleport.is_none()
        && callback.passages.is_empty()
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn step_arena_concession(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
    ) -> Option<MovementPositionSnapshot> {
        if origin.position() == entered.position()
            || !(1..=10).any(|pit| in_pit(entered.position(), pit))
        {
            return None;
        }
        let map = self.bundle_map()?;
        let at = super::super::quest_callbacks::native(entered.position())?;
        let tile = map.overlay.base().tile(at.x, at.y, at.floor)?;
        let mut portals = (0..tile.depths().iter().filter(|d| **d == 0).count())
            .filter_map(|ordinal| {
                oteryn_world_bundle::bundle::placement_key(
                    at.floor,
                    at.x,
                    at.y,
                    ordinal.try_into().ok()?,
                )
            })
            .filter(|key| {
                !map.facts.base_hidden(*key)
                    && (map
                        .facts
                        .quest_callback_matches(*key, None, Some(ACTION), None)
                        || self.callback_content().iter().any(|callback| {
                            owns_callback(callback)
                                && super::super::quest_callbacks::matches(map, callback, *key)
                        }))
            });
        let source = portals.next()?;
        if portals.next().is_some() {
            return None;
        }

        // STEP already holds the movement spell lane. Do not wait for a wire
        // command holding the arena gate while it waits to enter that lane.
        let _command = self.states.arena_matches.command.try_lock().ok()?;
        let live = self
            .states
            .arena_matches
            .live
            .lock()
            .ok()?
            .get(&session)?
            .clone();
        if live.actor != actor
            || live.deadline <= Instant::now()
            || !matches!(live.state.phase, 1 | 2)
            || !in_pit(entered.position(), live.state.pit)
            || !in_pit(origin.position(), live.state.pit)
        {
            return None;
        }
        let kick = position(donor::KICK);
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime.read_actor_position(actor) != Ok(entered)
            || runtime.binding().scope_generation().get() != live.state.scope_generation
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, session)
            || !walkable(self, &runtime, kick)
            || map.facts.base_hidden(source)
        {
            return None;
        }
        runtime
            .relocate_arena_player(actor, session, entered, kick)
            .ok()?;
        let now = self.owner_now().get();
        let mut occurrence = [0; 16];
        occurrence[..8].copy_from_slice(&source.to_be_bytes());
        occurrence[8..].copy_from_slice(&entered.revision().to_be_bytes());
        if let Some(current) = states.get(&runtime, actor, session)
            && let Some(definition) = self.concession_outfit()
            && let Ok(base_speed) = u16::try_from(current.owned_base_speed())
        {
            let root =
                GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
            let facts = ApplicationFacts {
                now,
                base_speed,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: DecisionOccurrenceId::from_bytes(occurrence),
            };
            let old = current.owned_conditions();
            let mut conditions = old.clone();
            let mut next = current.clone();
            if conditions
                .apply(
                    &definition,
                    Some(crate::spell::combat_execution::actor_atom(actor)),
                    ConditionSourceKind::SelfUse,
                    &[],
                    &facts,
                )
                .is_ok()
                && next.apply_batch_conditions(old, &conditions).is_ok()
                && next.advance_batch_revision().is_ok()
            {
                let _ = states.commit(&runtime, actor, session, next);
            }
        }
        if let Some(at) = super::super::quest_callbacks::native(kick) {
            let _ = map
                .facts
                .emit_movement_speech(crate::map::facts::MovementSpeech {
                    speaker: actor,
                    position: at,
                    talk_type: 36,
                    text: "Coward!".into(),
                    captured_name: None,
                });
        }
        // Keep the live run until the native departure receipt settles. The
        // existing scheduler retries a failed write and releases its lease.
        states.owner_wake.notify_one();
        runtime.read_actor_position(actor).ok()
    }

    fn concession_outfit(&self) -> Option<ConditionDefinition> {
        let appearance = self
            .active_generation?
            .native_gameplay()?
            .spell_appearances()?
            .for_look_type(LOOK_TYPE)?;
        ConditionDefinition::new(
            "quest.svargrond.arena.concession",
            1,
            ConditionValues::Outfit {
                look_type: LOOK_TYPE,
                duration_ms: DURATION_MS,
            },
        )?
        .with_appearance(appearance.selection())
    }
}
