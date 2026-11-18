//! Crystal 201189f8 movements_arena_pit.lua:14-21.
use super::*;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind, ConditionValues,
    MovementPositionSnapshot,
};
use crate::map::{
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn step_arena_concession(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
    ) -> MovementPositionSnapshot {
        if origin.position() == entered.position() {
            return entered;
        }
        self.try_arena_concession(permit, actor, session, entered)
            .await
            .unwrap_or(entered)
    }

    async fn try_arena_concession(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) -> Option<MovementPositionSnapshot> {
        // Movement already owns the spell lane. Avoid waiting for a command
        // that could hold the arena gate while waiting for that lane.
        let _gate = self.states.arena_matches.command.try_lock().ok()?;
        let live = self
            .states
            .arena_matches
            .live
            .lock()
            .ok()?
            .get(&session)?
            .clone();
        if live.actor != actor || !matches!(live.state.phase, 1 | 2) {
            return None;
        }
        let cause = durable::NativeArenaDeparture::concession(
            actor,
            session,
            entered,
            self.reset_epoch,
            live.state.run,
            live.state.pit,
        )?;
        let at = entered.position();
        let pos = TilePos {
            x: at.x.try_into().ok()?,
            y: at.y.try_into().ok()?,
            floor: i8::try_from(-at.floor).ok()?,
        };
        let map = self.bundle_map()?;
        let base = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
        let key = placement_key(pos, 1)?;
        let (id, _) = base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .nth(1)?;
        if map.facts.base_hidden(key)
            || !(map.facts.quest_callback_native_object(key, *id, 1949)
                || map
                    .facts
                    .quest_callback_matches(key, None, Some(25300), Some(1949)))
        {
            return None;
        }
        {
            let runtime = self.runtime.lock().await;
            if !walkable(self, &runtime, position(donor::KICK)) {
                return None;
            }
        }
        self.commit_arena_departure(permit, session, &live, entered, cause)
            .await?;
        let runtime = self.runtime.lock().await;
        let kicked = runtime.read_actor_position(actor).ok()?;
        if kicked.position() == position(donor::KICK) {
            let _ = self.apply_arena_coward_outfit(&runtime, actor, session, live.state.run);
            let _ = map
                .facts
                .emit_movement_speech(crate::map::facts::MovementSpeech {
                    speaker: actor,
                    position: TilePos {
                        x: 32251,
                        y: 31098,
                        floor: -6,
                    },
                    talk_type: 36,
                    text: "Coward!".to_owned(),
                    captured_name: None,
                });
        }
        Some(kicked)
    }

    fn apply_arena_coward_outfit(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        run: [u8; 16],
    ) -> Option<()> {
        let appearance = self
            .active_generation?
            .native_gameplay()?
            .spell_appearances()?
            .for_look_type(111)?;
        let definition = ConditionDefinition::new(
            "environment.svargrond-concession",
            1,
            ConditionValues::Outfit {
                look_type: 111,
                duration_ms: 120_000,
            },
        )?
        .with_appearance(appearance.selection())?;
        let mut states = self.states.spell_states.try_lock().ok()?;
        let mut next = states.get(runtime, actor, session)?.clone();
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let facts = ApplicationFacts {
            now: self.owner_now().get(),
            base_speed: u16::try_from(next.owned_base_speed()).ok()?,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &root,
            occurrence: DecisionOccurrenceId::from_bytes(run),
        };
        next.conditions
            .apply(&definition, None, ConditionSourceKind::Field, &[], &facts)
            .ok()?;
        next.source_set_batch_revision(next.revision().checked_add(1)?);
        states.commit(runtime, actor, session, next).then_some(())
    }
}
