//! Spell cast wire composition (`OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1`
//! §9 step 2). The typed payload codecs live in `oteryn-protocol-oteryn::actor_spell` (#1254) and
//! are re-exported here, as `world_spatial` does; this module adds the Channel owner's per-actor
//! vitals and cooldowns and the owner work item of one cast.

pub(crate) use oteryn_protocol_oteryn::actor_spell::*;

use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

use crate::ability::{AbilityOccurrence, RevisionSet};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use crate::spell::SpellBook;
use crate::spell::cast::{CastContext, CharacterCastFacts, PlayerSpellState, cast};

/// Spell cast §4 (SPELL-D2): the Channel owner's vitals and cooldowns of its present player
/// actors, runtime-actor-local and never durable. Like the door runtime it sits beside the
/// `ChannelRuntimeV1` and is only used while that runtime is locked (runtime first), within the
/// same owner work item. An entry is keyed by the exact actor (World, Channel, scope generation,
/// slot and slot generation), so it lives exactly as long as that actor stays present: every read
/// and write first proves the actor is still the committed player of the GameSession, and an
/// entry whose actor is gone is unreachable and dropped at the next initialization.
#[derive(Debug, Default)]
pub(crate) struct ChannelSpellStates {
    actors: Vec<(ExactActorRef, GameSessionId, PlayerSpellState)>,
}

impl ChannelSpellStates {
    /// Create the actor's vitals at its maxima and empty cooldowns in the owner step that makes
    /// it playable. A present actor that already has them (a retry, a same-GameSession reconnect)
    /// keeps them exactly: nothing is refilled or reset. `None` when the actor is not the
    /// committed player of `game_session_id`, or the facts exceed the SPELL-D8 bounds.
    pub(crate) fn initialize(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        facts: CharacterCastFacts,
    ) -> Option<&PlayerSpellState> {
        runtime.player_control_facts(actor, game_session_id).ok()?;
        self.actors.retain(|(present, session, _)| {
            runtime.player_control_facts(*present, *session).is_ok()
        });
        let index = match self.index(actor, game_session_id) {
            Some(index) => index,
            None => {
                let state = PlayerSpellState::new(facts)?;
                self.actors.try_reserve(1).ok()?;
                self.actors.push((actor, game_session_id, state));
                self.actors.len() - 1
            }
        };
        self.actors.get(index).map(|(_, _, state)| state)
    }

    fn get(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<&PlayerSpellState> {
        runtime.player_control_facts(actor, game_session_id).ok()?;
        let index = self.index(actor, game_session_id)?;
        self.actors.get(index).map(|(_, _, state)| state)
    }

    /// Compare-commit (SPELL-D3): `next` replaces the state only as the direct successor of the
    /// current revision, so one write carries the heal and the whole anchor payment.
    fn commit(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        next: PlayerSpellState,
    ) -> bool {
        if runtime
            .player_control_facts(actor, game_session_id)
            .is_err()
        {
            return false;
        }
        let Some(index) = self.index(actor, game_session_id) else {
            return false;
        };
        let current = &mut self.actors[index].2;
        if current.revision().checked_add(1) != Some(next.revision()) {
            return false;
        }
        *current = next;
        true
    }

    fn index(&self, actor: ExactActorRef, game_session_id: GameSessionId) -> Option<usize> {
        self.actors
            .iter()
            .position(|(present, session, _)| *present == actor && *session == game_session_id)
    }
}

/// `SPELL-RL-01`: cast inputs applied per actor per Channel owner work cycle. [`cast_in_channel`]
/// is that work item and takes exactly one intent; the serve loop reads no further command of the
/// actor until it returns, so later inputs stay outstanding FND-02 commands.
#[cfg(test)]
pub(crate) const SPELL_RL_01_CAST_INPUTS_PER_WORK_CYCLE: usize = 1;

/// The owner's result of one cast: its disposition and, only when it committed, the actor's new
/// `ACTOR_VITALS` revision and value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpellCastOutcome {
    pub(crate) disposition: SpellCastDisposition,
    pub(crate) vitals: Option<(u64, ActorVitals)>,
}

impl SpellCastOutcome {
    pub(crate) const fn rejected() -> Self {
        Self {
            disposition: SpellCastDisposition::Rejected,
            vitals: None,
        }
    }
}

/// The admitted actor's current `ACTOR_VITALS` revision and value, or `None` while it has no
/// Character cast facts.
pub(crate) fn observe_vitals(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    actor: ExactActorRef,
    game_session_id: GameSessionId,
) -> Option<(u64, ActorVitals)> {
    let state = states.get(runtime, actor, game_session_id)?;
    Some((state.revision(), state.vitals()))
}

/// One cast of `intent` by the admitted actor, as one Channel-owner work item under the runtime
/// lock the caller holds (and `states` beside it): read the actor's state, resolve, plan, and compare-commit the next state
/// with its anchor payment (SPELL-D3). `command_id` binds the occurrence and its draws, so the
/// outcome never depends on anything the client chooses beyond its intent. An actor without
/// Character cast facts, a stale binding or an unknown spell is `REJECTED` with no effect.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cast_in_channel(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    book: &SpellBook,
    actor: ExactActorRef,
    game_session_id: GameSessionId,
    command_id: u64,
    intent: &SpellCastIntent,
    now: SemanticTimeMicros,
) -> SpellCastOutcome {
    let Some(state) = states.get(runtime, actor, game_session_id) else {
        return SpellCastOutcome::rejected();
    };
    let placement = actor.placement_identity();
    let occurrence_bytes: [u8; 16] = {
        let digest = Sha256::new()
            .chain_update(b"oteryn:spell-cast-occurrence:v1")
            .chain_update(placement)
            .chain_update(game_session_id.as_bytes())
            .chain_update(command_id.to_be_bytes())
            .finalize();
        let mut bytes = [0_u8; 16];
        bytes.copy_from_slice(&digest[..16]);
        bytes
    };
    let caster = format!("actor:{}", hex(&placement));
    let occurrence_id = format!("spell-cast:{}", hex(&occurrence_bytes));
    let pin = runtime.content_pin();
    let content = format!("content:{}", pin.activation_sequence());
    let Ok(occurrence) = RevisionSet::new(
        "ruleset:spell-v1",
        &content,
        "world:spell-v1",
        "formula:spell-p3a",
        "simulation:v1",
    )
    .and_then(|revisions| AbilityOccurrence::new(&occurrence_id, revisions)) else {
        return SpellCastOutcome::rejected();
    };
    // SIM determinism: every magnitude is drawn from the owner stream bound to this occurrence.
    let root = GameplayDecisionRoot::from_bytes(pin.server_artifact_digest());
    let decision = DecisionOccurrenceId::from_bytes(occurrence_bytes);
    let mut draw_index = 0_u64;
    let mut draw = |minimum: i64, maximum: i64| {
        let value =
            deterministic_decision_u64(&root, decision, "spell.cast.draw", draw_index).unwrap_or(0);
        draw_index = draw_index.saturating_add(1);
        crate::spell::uniform_draw(value, minimum, maximum)
    };
    let next = cast(
        book,
        state,
        intent,
        CastContext {
            caster: &caster,
            owner_scope: "channel-owner",
            occurrence,
            now,
            draw: &mut draw,
        },
    );
    match next {
        Ok(next) => {
            let vitals = (next.revision(), next.vitals());
            if states.commit(runtime, actor, game_session_id, next) {
                SpellCastOutcome {
                    disposition: SpellCastDisposition::Cast,
                    vitals: Some(vitals),
                }
            } else {
                SpellCastOutcome::rejected()
            }
        }
        Err(disposition) => SpellCastOutcome {
            disposition,
            vitals: None,
        },
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
pub(crate) mod tests {
    use std::num::NonZeroU32;

    use super::*;
    use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId};
    use crate::spell::Vocation;

    const fn uuid_v7(tag: u8) -> [u8; 16] {
        [
            0x01, 0x90, 0x00, 0x00, 0x00, tag, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, tag,
        ]
    }

    /// A Channel runtime with one committed player actor bound to the session `tag`.
    pub(crate) fn runtime_with_player(tag: u8) -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
        let world = WorldId::decode(&uuid_v7(0x60)).expect("world");
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&uuid_v7(0x61)).expect("channel"),
            NodeId::decode(&uuid_v7(0x62)).expect("node"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            ChannelContentPin::test(world),
        )
        .expect("channel runtime");
        let session = GameSessionId::decode(&uuid_v7(tag)).expect("session");
        let reservation = runtime.reserve_fresh_session(session).expect("reserve");
        let actor = runtime.commit_fresh_session(reservation).expect("commit");
        (runtime, actor, session)
    }

    /// Level 8 druid facts, as the Character owner would supply them.
    pub(crate) const FACTS: CharacterCastFacts = CharacterCastFacts {
        vocation: Vocation::Druid,
        level: 8,
        magic_level: 0,
        max_health: 185,
        max_mana: 90,
        max_soul: 100,
    };

    /// `exura` (index 3 of the canonical V1 book) with no target.
    pub(crate) fn exura() -> SpellCastIntent {
        SpellCastIntent {
            spell: NonZeroU32::new(3).expect("index"),
            target: SpellTarget::None,
            aim_at_target: false,
        }
    }

    fn now(millis: u64) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(millis * 1000)
    }

    fn cast_at(
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        command_id: u64,
        millis: u64,
    ) -> SpellCastOutcome {
        let book = crate::spell::cast::v1_spell_book().expect("book");
        cast_in_channel(
            runtime,
            states,
            &book,
            actor,
            session,
            command_id,
            &exura(),
            now(millis),
        )
    }

    #[test]
    fn spell_rl_01_matches_the_registered_row() {
        let registry: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json"
        ))
        .expect("registry");
        let row = registry["entries"]
            .as_array()
            .and_then(|entries| entries.iter().find(|entry| entry["id"] == "SPELL-RL-01"))
            .and_then(|entry| entry["hard_maximum"].as_u64());
        assert_eq!(
            row,
            u64::try_from(SPELL_RL_01_CAST_INPUTS_PER_WORK_CYCLE).ok()
        );
    }

    #[test]
    fn a_cast_commits_once_and_a_repeated_proposal_never_pays_twice() {
        let (runtime, actor, session) = runtime_with_player(0x63);
        let mut states = ChannelSpellStates::default();
        states
            .initialize(&runtime, actor, session, FACTS)
            .expect("initialized");
        let full = ActorVitals {
            health: 185,
            max_health: 185,
            mana: 90,
            max_mana: 90,
            soul: 100,
            harmony: 0,
            serene: false,
        };
        assert_eq!(
            observe_vitals(&runtime, &states, actor, session),
            Some((1, full))
        );
        let cast = cast_at(&runtime, &mut states, actor, session, 1, 0);
        assert_eq!(cast.disposition, SpellCastDisposition::Cast);
        let (revision, vitals) = cast.vitals.expect("committed vitals");
        assert_eq!((revision, vitals.mana, vitals.health), (2, 70, 185));
        // A second input while cooling down is answered and pays nothing.
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 2, 500),
            SpellCastOutcome {
                disposition: SpellCastDisposition::CoolingDown,
                vitals: None,
            }
        );
        assert_eq!(
            observe_vitals(&runtime, &states, actor, session),
            Some((2, vitals))
        );
        // The committed state proposed again is not a successor: compare-commit refuses it.
        let committed = states.get(&runtime, actor, session).expect("state").clone();
        assert!(!states.commit(&runtime, actor, session, committed));
        assert_eq!(
            observe_vitals(&runtime, &states, actor, session),
            Some((2, vitals))
        );
    }

    #[test]
    fn reinitializing_a_present_actor_keeps_its_vitals_and_cooldowns() {
        let (runtime, actor, session) = runtime_with_player(0x64);
        let mut states = ChannelSpellStates::default();
        states
            .initialize(&runtime, actor, session, FACTS)
            .expect("initialized");
        cast_at(&runtime, &mut states, actor, session, 1, 0);
        // A same-GameSession reconnect initializes again: no refill, no cooldown reset.
        let kept = states
            .initialize(&runtime, actor, session, FACTS)
            .expect("kept")
            .clone();
        assert_eq!((kept.revision(), kept.vitals().mana), (2, 70));
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 2, 1),
            SpellCastOutcome {
                disposition: SpellCastDisposition::CoolingDown,
                vitals: None,
            }
        );
    }

    #[test]
    fn a_foreign_session_or_an_ended_actor_is_rejected_and_its_state_dropped() {
        let (mut runtime, actor, session) = runtime_with_player(0x65);
        let other = GameSessionId::decode(&uuid_v7(0x66)).expect("session");
        let mut states = ChannelSpellStates::default();
        assert!(states.initialize(&runtime, actor, other, FACTS).is_none());
        states
            .initialize(&runtime, actor, session, FACTS)
            .expect("initialized");
        assert_eq!(
            cast_at(&runtime, &mut states, actor, other, 1, 0),
            SpellCastOutcome::rejected()
        );
        assert_eq!(observe_vitals(&runtime, &states, actor, other), None);
        // The actor ends: its state is unreachable and the next initialization drops it.
        runtime
            .remove_terminal_session(session, actor)
            .expect("removed");
        assert_eq!(observe_vitals(&runtime, &states, actor, session), None);
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 2, 0),
            SpellCastOutcome::rejected()
        );
        let reservation = runtime.reserve_fresh_session(other).expect("reserve");
        let successor = runtime.commit_fresh_session(reservation).expect("commit");
        let fresh = states
            .initialize(&runtime, successor, other, FACTS)
            .expect("fresh actor")
            .clone();
        assert_eq!((fresh.revision(), fresh.vitals().mana), (1, 90));
        assert_eq!(states.actors.len(), 1);
    }

    #[test]
    fn without_character_cast_facts_casting_stays_gated() {
        let (runtime, actor, session) = runtime_with_player(0x67);
        let mut states = ChannelSpellStates::default();
        assert_eq!(observe_vitals(&runtime, &states, actor, session), None);
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 1, 0),
            SpellCastOutcome::rejected()
        );
    }

    #[test]
    fn the_occurrence_and_its_draws_are_bound_to_the_command() {
        let outcome = |command_id| {
            let (runtime, actor, session) = runtime_with_player(0x68);
            let mut states = ChannelSpellStates::default();
            states
                .initialize(&runtime, actor, session, FACTS)
                .expect("state");
            cast_at(&runtime, &mut states, actor, session, command_id, 0)
        };
        assert_eq!(outcome(7), outcome(7));
    }
}
