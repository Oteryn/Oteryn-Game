//! Spell cast wire composition (`OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1`
//! §9 step 2). The typed payload codecs live in `oteryn-protocol-oteryn::actor_spell` (#1254) and
//! are re-exported here, as `world_spatial` does; this module adds the Channel owner's per-actor
//! vitals and cooldowns and the owner work item of one cast.

pub(crate) use oteryn_protocol_oteryn::actor_spell::*;

use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, SemanticTimeMicros, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

use crate::ability::creature_bite::{CreatureBiteVitals, CreatureHit};
use crate::ability::{AbilityOccurrence, RevisionSet};
use crate::durability::character_death::PlayerDeathOccurrence;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::spell::SpellBook;
use crate::spell::cast::{CastContext, CharacterCastFacts, PlayerSpellState, cast};

/// Spell cast §4 (SPELL-D2): the Channel owner's vitals and cooldowns of its present player
/// actors, runtime-actor-local and never durable. Like the door runtime it sits beside the
/// `ChannelRuntimeV1` and is only used while that runtime is locked (runtime first), within the
/// same owner work item. An entry is keyed by the exact actor (World, Channel, scope generation,
/// slot and slot generation), so it lives exactly as long as that actor stays present: every read
/// and write first proves the actor is still the committed player of the GameSession, and an
/// entry whose actor is gone is unreachable and dropped at the next initialization.
///
/// DEATH-2 (Reference first player death decision §4.2, §4.5): the vitals owner also keeps each
/// present player's death, from the lethal write until its respawn. A dead player takes no
/// command, no damage and no credit, and its death record goes with its entry.
#[derive(Debug)]
pub(crate) struct ChannelSpellStates {
    actors: Vec<(ExactActorRef, GameSessionId, PlayerSpellState)>,
    deaths: Vec<(ExactActorRef, GameSessionId, PlayerDeath)>,
    mint_death: fn() -> Option<PlayerDeathOccurrence>,
}

impl Default for ChannelSpellStates {
    fn default() -> Self {
        Self {
            actors: Vec::new(),
            deaths: Vec::new(),
            mint_death: mint_player_death_occurrence,
        }
    }
}

/// One present player's death (§4.2): the occurrence minted at the lethal write, the idempotency
/// key of every consequence, and the cell the player died on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerDeath {
    pub(crate) occurrence: PlayerDeathOccurrence,
    pub(crate) cell: MovementLocalPosition,
}

/// A fresh UUIDv7 `PlayerDeathOccurrence`: wall-clock milliseconds and the TLS provider's secure
/// random source, like a GameSession id.
fn mint_player_death_occurrence() -> Option<PlayerDeathOccurrence> {
    let mut bytes = [0_u8; 16];
    rustls::crypto::aws_lc_rs::default_provider()
        .secure_random
        .fill(&mut bytes)
        .ok()?;
    let millis = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis(),
    )
    .ok()?;
    bytes[..6].copy_from_slice(&millis.to_be_bytes()[2..]);
    bytes[6] = 0x70 | (bytes[6] & 0x0f);
    bytes[8] = 0x80 | (bytes[8] & 0x3f);
    PlayerDeathOccurrence::from_bytes(bytes).ok()
}

impl ChannelSpellStates {
    /// Create the actor's vitals at its maxima and empty cooldowns in the owner step that makes
    /// it playable, with the durable monk values `(harmony, serene_forced_micros)` the Character
    /// owner loaded (SPELL-D8 §8.2), and run the Serene initialization evaluation at `now` before
    /// any command. A present actor that already has them (a retry, a same-GameSession
    /// reconnect) keeps them exactly: nothing is refilled or reset, and only Serene is evaluated
    /// again. `None` when the actor is not the committed player of `game_session_id`, the facts
    /// exceed the SPELL-D8 bounds, or the monk values are corrupt.
    pub(crate) fn initialize(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        facts: CharacterCastFacts,
        (harmony, serene_forced_micros): (u8, u64),
        now: SemanticTimeMicros,
    ) -> Option<&PlayerSpellState> {
        runtime.player_control_facts(actor, game_session_id).ok()?;
        self.actors.retain(|(present, session, _)| {
            runtime.player_control_facts(*present, *session).is_ok()
        });
        let actors = &self.actors;
        self.deaths.retain(|(dead, session, _)| {
            actors
                .iter()
                .any(|(present, held, _)| present == dead && held == session)
        });
        let index = match self.index(actor, game_session_id) {
            Some(index) => index,
            None => {
                let mut state = PlayerSpellState::new(facts, harmony, serene_forced_micros)?;
                state.make_playable(now).ok()?;
                self.actors.try_reserve(1).ok()?;
                self.actors.push((actor, game_session_id, state));
                return self.actors.last().map(|(_, _, state)| state);
            }
        };
        let state = &mut self.actors.get_mut(index)?.2;
        state.make_playable(now).ok()?;
        Some(state)
    }

    /// FND-04B §20/§21 recovery made the present actor playable again: the Serene
    /// initialization evaluation runs before its first command (§8.2). `false` when the actor has
    /// no spell state or the evaluation failed.
    pub(crate) fn resume(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        now: SemanticTimeMicros,
    ) -> bool {
        self.get_mut(runtime, actor, game_session_id)
            .is_some_and(|state| state.make_playable(now).is_ok())
    }

    /// The session left the actor (control loss): no command is accepted until [`Self::resume`].
    pub(crate) fn detach(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) {
        if let Some(state) = self.get_mut(runtime, actor, game_session_id) {
            state.detach();
        }
    }

    /// The periodic 1000 ms Serene evaluation of the actor (§8.2): the new `ACTOR_VITALS`
    /// revision and value when Serene changed, which the owner publishes.
    pub(crate) fn tick(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        now: SemanticTimeMicros,
    ) -> Option<(u64, ActorVitals)> {
        if self.is_dead(actor) {
            return None;
        }
        let state = self.get_mut(runtime, actor, game_session_id)?;
        state
            .tick(now)
            .ok()
            .filter(|changed| *changed)
            .map(|_| (state.revision(), state.vitals()))
    }

    /// The monk values the actor-end save writes (§8.2), or `None` when the actor has no spell
    /// state or is not a monk.
    pub(crate) fn monk_save_values(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        now: SemanticTimeMicros,
    ) -> Option<(u8, u64)> {
        self.get(runtime, actor, game_session_id)?
            .monk_save_values(now)
    }

    /// Commit a resolved health credit under the Channel owner's runtime lock.
    /// The occurrence owner suppresses replay before calling this method.
    /// Zero/full-pool credit writes nothing; stale bindings and exhaustion refuse.
    #[allow(
        dead_code,
        reason = "prepared native combat credit awaits the owning occurrence caller"
    )]
    pub(crate) fn apply_health_gain(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        amount: u64,
    ) -> Option<(u32, u64)> {
        if self.is_dead(actor) {
            return None;
        }
        let state = self.get(runtime, actor, game_session_id)?;
        let (next, gained) = state.after_health_gain(amount)?;
        let revision = next.revision();
        if gained == 0 {
            return Some((0, revision));
        }
        self.commit(runtime, actor, game_session_id, next)
            .then_some((gained, revision))
    }

    /// Commit a resolved mana credit under the Channel owner's runtime lock.
    /// The occurrence owner suppresses replay before calling this method.
    /// Zero/full-pool credit writes nothing; stale bindings and exhaustion refuse.
    #[allow(
        dead_code,
        reason = "prepared native combat credit awaits the owning occurrence caller"
    )]
    pub(crate) fn apply_mana_gain(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        amount: u64,
    ) -> Option<(u32, u64)> {
        if self.is_dead(actor) {
            return None;
        }
        let state = self.get(runtime, actor, game_session_id)?;
        let (next, gained) = state.after_mana_gain(amount)?;
        let revision = next.revision();
        if gained == 0 {
            return Some((0, revision));
        }
        self.commit(runtime, actor, game_session_id, next)
            .then_some((gained, revision))
    }

    /// DEATH-2 §4.2: true from the lethal write until the respawn. A dead player takes no input.
    pub(crate) fn is_dead(&self, actor: ExactActorRef) -> bool {
        self.deaths.iter().any(|(dead, _, _)| *dead == actor)
    }

    /// The death of the committed player of `game_session_id`, while it waits for its respawn.
    pub(crate) fn player_death(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<PlayerDeath> {
        runtime.player_control_facts(actor, game_session_id).ok()?;
        self.deaths
            .iter()
            .find(|(dead, session, _)| *dead == actor && *session == game_session_id)
            .map(|(_, _, death)| *death)
    }

    /// DEATH-2 §4.5 (D63): the respawn of `occurrence` in one owner step, after the caller placed
    /// the actor at its respawn position. Health and mana are refilled to their maxima under one
    /// new `ACTOR_VITALS` revision, a monk's Harmony and forced Serene are reset with its Serene
    /// evaluation initialized at `now`, and the death record ends; the new revision and value are
    /// returned for the owner to publish. `None` changes nothing: the actor is not this death's
    /// committed player.
    pub(crate) fn respawn(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
        occurrence: PlayerDeathOccurrence,
        now: SemanticTimeMicros,
    ) -> Option<(u64, ActorVitals)> {
        let death = self.deaths.iter().position(|(dead, session, death)| {
            *dead == actor && *session == game_session_id && death.occurrence == occurrence
        })?;
        let next = self.get(runtime, actor, game_session_id)?.respawned(now)?;
        let vitals = (next.revision(), next.vitals());
        if !self.commit(runtime, actor, game_session_id, next) {
            return None;
        }
        self.deaths.swap_remove(death);
        Some(vitals)
    }

    fn get_mut(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        game_session_id: GameSessionId,
    ) -> Option<&mut PlayerSpellState> {
        runtime.player_control_facts(actor, game_session_id).ok()?;
        let index = self.index(actor, game_session_id)?;
        self.actors.get_mut(index).map(|(_, _, state)| state)
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

/// GAME-AI-01 slice §4.6/§4.7: the vitals owner's side of a creature bite. One hit is one
/// compare-committed vitals revision. DEATH-2 (§4.1, §4.2): a hit to 0 is lethal; the death
/// occurrence is minted and the death recorded with the cell in the same write, and a dead
/// player is no target. A failed mint or position read refuses the hit with nothing written.
impl CreatureBiteVitals for ChannelSpellStates {
    fn apply_creature_damage(
        &mut self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        target_session: GameSessionId,
        magnitude: u32,
    ) -> Option<CreatureHit> {
        if self.is_dead(target) {
            return None;
        }
        let state = self.get(runtime, target, target_session)?;
        let (next, damage) = state.after_creature_damage(magnitude)?;
        let death = if damage.health_after == 0 {
            let cell = runtime.read_actor_position(target).ok()?.position();
            self.deaths.try_reserve(1).ok()?;
            Some(PlayerDeath {
                occurrence: (self.mint_death)()?,
                cell,
            })
        } else {
            None
        };
        let vitals_revision = next.revision();
        if !self.commit(runtime, target, target_session, next) {
            return None;
        }
        if let Some(death) = death {
            self.deaths.push((target, target_session, death));
        }
        Some(CreatureHit {
            damage,
            vitals_revision,
            death: death.map(|death| *death.occurrence.as_bytes()),
        })
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
    if states.is_dead(actor) {
        return SpellCastOutcome::rejected();
    }
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

    /// Test only: no damage owner exists yet, so a wounded actor is staged by writing its health.
    pub(crate) fn wound(
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        health: u32,
    ) {
        let index = states.index(actor, session).expect("present");
        states.actors[index].2.set_health_for_test(health);
    }

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
    fn health_credit_commits_native_vitals_and_preserves_noops() {
        let (runtime, actor, session) = runtime_with_player(0x73);
        let mut states = ChannelSpellStates::default();
        states
            .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
            .expect("initialized");
        wound(&mut states, actor, session, 100);
        assert_eq!(
            states.apply_health_gain(&runtime, actor, session, 0),
            Some((0, 1))
        );
        assert_eq!(
            states.apply_health_gain(&runtime, actor, session, 7),
            Some((7, 2))
        );
        let (revision, vitals) =
            observe_vitals(&runtime, &states, actor, session).expect("observed");
        assert_eq!(
            (revision, vitals.health, vitals.mana, vitals.soul),
            (2, 107, 90, 100)
        );
        assert_eq!(
            states.apply_health_gain(&runtime, actor, session, u64::MAX),
            Some((78, 3))
        );
        let before = states
            .get(&runtime, actor, session)
            .expect("present")
            .clone();
        assert_eq!(
            states.apply_health_gain(&runtime, actor, session, 1),
            Some((0, 3))
        );
        assert_eq!(states.get(&runtime, actor, session), Some(&before));
    }

    #[test]
    fn mana_credit_commits_native_vitals_and_preserves_noops() {
        let (runtime, actor, session) = runtime_with_player(0x74);
        let mut states = ChannelSpellStates::default();
        states
            .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
            .expect("initialized");
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 1, 0).disposition,
            SpellCastDisposition::Cast
        );
        assert_eq!(
            states.apply_mana_gain(&runtime, actor, session, 0),
            Some((0, 2))
        );
        assert_eq!(
            states.apply_mana_gain(&runtime, actor, session, 7),
            Some((7, 3))
        );
        let (revision, vitals) =
            observe_vitals(&runtime, &states, actor, session).expect("observed");
        assert_eq!(
            (revision, vitals.mana, vitals.health, vitals.soul),
            (3, 77, 185, 100)
        );
        assert_eq!(
            states.apply_mana_gain(&runtime, actor, session, u64::MAX),
            Some((13, 4))
        );
        let before = states
            .get(&runtime, actor, session)
            .expect("present")
            .clone();
        assert_eq!(
            states.apply_mana_gain(&runtime, actor, session, 1),
            Some((0, 4))
        );
        assert_eq!(states.get(&runtime, actor, session), Some(&before));
    }

    #[test]
    fn vitals_credit_refuses_foreign_sessions_and_ended_generations() {
        let (mut runtime, actor, session) = runtime_with_player(0x75);
        let other = GameSessionId::decode(&uuid_v7(0x76)).expect("other session");
        let mut states = ChannelSpellStates::default();
        assert!(
            states
                .apply_health_gain(&runtime, actor, session, 7)
                .is_none()
        );
        assert!(
            states
                .apply_mana_gain(&runtime, actor, session, 7)
                .is_none()
        );
        states
            .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
            .expect("initialized");
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 1, 0).disposition,
            SpellCastDisposition::Cast
        );
        wound(&mut states, actor, session, 100);
        let before = states.actors.clone();
        assert!(
            states
                .apply_health_gain(&runtime, actor, other, 7)
                .is_none()
        );
        assert!(states.apply_mana_gain(&runtime, actor, other, 7).is_none());
        assert_eq!(states.actors, before);

        runtime
            .remove_terminal_session(session, actor)
            .expect("ended");
        assert!(
            states
                .apply_health_gain(&runtime, actor, session, 7)
                .is_none()
        );
        assert!(
            states
                .apply_mana_gain(&runtime, actor, session, 7)
                .is_none()
        );
        assert_eq!(states.actors, before);

        let reservation = runtime.reserve_fresh_session(other).expect("reserve");
        let successor = runtime
            .commit_fresh_session(reservation)
            .expect("successor");
        assert_ne!(actor, successor);
        states
            .initialize(&runtime, successor, other, FACTS, (0, 0), now(0))
            .expect("fresh state");
        assert!(
            states
                .apply_health_gain(&runtime, actor, session, 7)
                .is_none()
        );
        assert!(
            states
                .apply_mana_gain(&runtime, actor, session, 7)
                .is_none()
        );
        assert_eq!(
            states.apply_health_gain(&runtime, successor, other, 7),
            Some((0, 1))
        );
        assert_eq!(
            states.apply_mana_gain(&runtime, successor, other, 7),
            Some((0, 1))
        );
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
            .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
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
            .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
            .expect("initialized");
        cast_at(&runtime, &mut states, actor, session, 1, 0);
        // A same-GameSession reconnect initializes again: no refill, no cooldown reset.
        let kept = states
            .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
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
        assert!(
            states
                .initialize(&runtime, actor, other, FACTS, (0, 0), now(0))
                .is_none()
        );
        states
            .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
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
            .initialize(&runtime, successor, other, FACTS, (0, 0), now(0))
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
                .initialize(&runtime, actor, session, FACTS, (0, 0), now(0))
                .expect("state");
            cast_at(&runtime, &mut states, actor, session, command_id, 0)
        };
        assert_eq!(outcome(7), outcome(7));
    }

    /// SPELL-D8 §8.2 in the Channel owner: a new monk actor loads the durable values and is
    /// Serene from its initialization evaluation; a reconnect keeps Harmony and the forced time,
    /// and the actor-end save reads the live values.
    #[test]
    fn a_monk_actor_loads_its_durable_values_and_keeps_them_across_a_reconnect() {
        let (runtime, actor, session) = runtime_with_player(0x69);
        let monk = CharacterCastFacts {
            vocation: Vocation::Monk,
            ..FACTS
        };
        let mut states = ChannelSpellStates::default();
        assert!(
            states
                .initialize(&runtime, actor, session, monk, (6, 0), now(0))
                .is_none(),
            "a corrupt durable Harmony fails the actor closed"
        );
        let state = states
            .initialize(&runtime, actor, session, monk, (3, 4_000_000), now(1000))
            .expect("monk actor");
        assert_eq!((state.vitals().harmony, state.vitals().serene), (3, true));
        assert_eq!(
            states.monk_save_values(&runtime, actor, session, now(2000)),
            Some((3, 3_000_000))
        );
        // A solo monk's periodic evaluation changes nothing, so nothing is published.
        assert_eq!(states.tick(&runtime, actor, session, now(2000)), None);
        // Control loss detaches the actor: its casts are refused until a recovery.
        states.detach(&runtime, actor, session);
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 1, 2500),
            SpellCastOutcome::rejected()
        );
        assert!(states.resume(&runtime, actor, session, now(3000)));
        // A present actor keeps its values: a second initialization reloads nothing.
        let kept = states
            .initialize(&runtime, actor, session, monk, (0, 0), now(3000))
            .expect("kept");
        assert_eq!(kept.vitals().harmony, 3);
        assert_eq!(
            states.monk_save_values(&runtime, actor, session, now(3000)),
            Some((3, 2_000_000))
        );
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 2, 3000).disposition,
            SpellCastDisposition::Cast
        );
        // A druid has no monk values to save.
        let (druid_runtime, druid, druid_session) = runtime_with_player(0x6a);
        let mut druids = ChannelSpellStates::default();
        druids
            .initialize(&druid_runtime, druid, druid_session, FACTS, (0, 0), now(0))
            .expect("druid");
        assert_eq!(
            druids.monk_save_values(&druid_runtime, druid, druid_session, now(0)),
            None
        );
    }

    /// A runtime with one positioned player at (10, 10), its vitals at a wounded `health`, and one
    /// live creature beside it.
    fn bitten_player(
        tag: u8,
        health: u32,
    ) -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        ExactActorRef,
        GameSessionId,
        ExactActorRef,
    ) {
        bitten_actor(tag, health, FACTS, (0, 0))
    }

    /// [`bitten_player`] with the actor's cast facts and durable monk values.
    fn bitten_actor(
        tag: u8,
        health: u32,
        facts: CharacterCastFacts,
        monk: (u8, u64),
    ) -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        ExactActorRef,
        GameSessionId,
        ExactActorRef,
    ) {
        use crate::foundation::MovementLocalPosition;
        let (mut runtime, actor, session) = runtime_with_player(tag);
        let at = |x, y| MovementLocalPosition { x, y, floor: 7 };
        runtime
            .initialize_movement_test_position(actor, at(10, 10))
            .expect("player position");
        let creature = runtime.admit_test_creature(at(11, 10)).expect("creature");
        let mut states = ChannelSpellStates::default();
        states
            .initialize(&runtime, actor, session, facts, monk, now(0))
            .expect("vitals");
        wound(&mut states, actor, session, health);
        (runtime, states, actor, session, creature)
    }

    fn bite_at(
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        ledger: &mut crate::ability::creature_bite::CreatureBiteLedger,
        (creature, actor, session): (ExactActorRef, ExactActorRef, GameSessionId),
        sequence: u64,
        micros: u64,
    ) -> Result<
        crate::ability::creature_bite::AppliedBite,
        crate::ability::creature_bite::BiteRejection,
    > {
        use crate::ability::AiAbilityAdapter;
        use crate::ability::creature_bite::{
            CreatureBiteDefinition, ReentryProtection, commit_ai_bite,
        };
        use crate::foundation::owner_timer::SemanticTimeMicros as OwnerTime;
        commit_ai_bite(
            ledger,
            runtime,
            states,
            AiAbilityAdapter::bite(creature, sequence, actor, session),
            CreatureBiteDefinition::new(2_000_000, 8).expect("definition"),
            RevisionSet::new(
                "ruleset:ai-v1",
                "content:1",
                "world:ai-v1",
                "formula:bite-v1",
                "simulation:v1",
            )
            .expect("revisions"),
            ReentryProtection {
                protected_until: None,
            },
            OwnerTime::from_micros(micros),
        )
    }

    /// DEATH-2 §4.1/§4.2 on the real vitals owner: a creature bite to 0 is lethal, the death
    /// occurrence is minted and the death cell recorded in the same vitals revision, and the dead
    /// player takes no further bite, cast, credit or Serene change.
    #[test]
    fn a_lethal_creature_bite_records_the_death_and_the_dead_player_takes_nothing() {
        use crate::ability::creature_bite::{BiteRejection, CreatureBiteLedger, CreatureDamage};
        use crate::foundation::MovementLocalPosition;

        let (runtime, mut states, actor, session, creature) = bitten_player(0x31, 5);
        let mut ledger = CreatureBiteLedger::default();
        let actors = (creature, actor, session);
        let first = bite_at(&runtime, &mut states, &mut ledger, actors, 0, 0).expect("bite");
        assert_eq!(
            first.damage,
            CreatureDamage {
                applied: 5,
                health_after: 0
            }
        );
        assert_eq!(first.vitals_revision, 2);
        let death = states
            .player_death(&runtime, actor, session)
            .expect("recorded death");
        assert_eq!(first.death, Some(*death.occurrence.as_bytes()));
        assert_eq!(
            death.cell,
            MovementLocalPosition {
                x: 10,
                y: 10,
                floor: 7
            }
        );
        assert!(states.is_dead(actor));
        // The lethal think occurrence replays its first result; a later bite has no target.
        assert_eq!(
            bite_at(&runtime, &mut states, &mut ledger, actors, 0, 0),
            Ok(first)
        );
        assert_eq!(
            bite_at(&runtime, &mut states, &mut ledger, actors, 1, 2_000_000),
            Err(BiteRejection::StaleTarget)
        );
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 7, 3000),
            SpellCastOutcome::rejected()
        );
        assert_eq!(states.apply_health_gain(&runtime, actor, session, 50), None);
        assert_eq!(states.apply_mana_gain(&runtime, actor, session, 5), None);
        assert_eq!(states.tick(&runtime, actor, session, now(4000)), None);
        let (revision, vitals) = observe_vitals(&runtime, &states, actor, session).expect("vitals");
        assert_eq!((revision, vitals.health), (2, 0));
    }

    /// DEATH-2 §4.5 (D63): the respawn refills health and mana under exactly one new vitals
    /// revision and ends the death; only the recorded occurrence of the exact session respawns.
    #[test]
    fn the_respawn_refills_health_and_mana_in_one_revision_and_ends_the_death() {
        use crate::ability::creature_bite::CreatureBiteLedger;

        let (runtime, mut states, actor, session, creature) = bitten_player(0x32, 8);
        let mut ledger = CreatureBiteLedger::default();
        // A heal spends mana, so the respawn has both pools to refill.
        let healed = cast_at(&runtime, &mut states, actor, session, 1, 0);
        assert_eq!(healed.disposition, SpellCastDisposition::Cast);
        let spent = healed.vitals.expect("vitals").1;
        assert!(spent.mana < FACTS.max_mana);
        wound(&mut states, actor, session, 8);
        bite_at(
            &runtime,
            &mut states,
            &mut ledger,
            (creature, actor, session),
            0,
            0,
        )
        .expect("lethal bite");
        let death = states
            .player_death(&runtime, actor, session)
            .expect("death");
        let other = PlayerDeathOccurrence::from_bytes(uuid_v7(0x77)).expect("occurrence");
        assert_eq!(
            states.respawn(&runtime, actor, session, other, now(0)),
            None
        );
        let (_, other_actor, other_session) = runtime_with_player(0x33);
        assert_eq!(
            states.respawn(
                &runtime,
                other_actor,
                other_session,
                death.occurrence,
                now(0)
            ),
            None
        );
        let (revision, vitals) = states
            .respawn(&runtime, actor, session, death.occurrence, now(0))
            .expect("respawned");
        assert_eq!(revision, 4);
        assert_eq!(
            (vitals.health, vitals.mana),
            (FACTS.max_health, FACTS.max_mana)
        );
        assert!(!states.is_dead(actor));
        assert_eq!(states.player_death(&runtime, actor, session), None);
        // The respawn happens once; the living actor takes commands again.
        assert_eq!(
            states.respawn(&runtime, actor, session, death.occurrence, now(0)),
            None
        );
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 8, 10_000).disposition,
            SpellCastDisposition::Cast
        );
    }

    /// DEATH-2 §4.5 with SPELL-D8 §8.2: a monk respawns with Harmony and forced Serene reset, as
    /// the death commit writes them, and its Serene evaluation initialized again, so a monk that
    /// lost control while dead is playable after the respawn.
    #[test]
    fn a_monk_respawns_with_harmony_and_forced_serene_reset_and_initialized() {
        use crate::ability::creature_bite::CreatureBiteLedger;

        let monk = CharacterCastFacts {
            vocation: Vocation::Monk,
            ..FACTS
        };
        let (runtime, mut states, actor, session, creature) =
            bitten_actor(0x35, 8, monk, (3, 4_000_000));
        bite_at(
            &runtime,
            &mut states,
            &mut CreatureBiteLedger::default(),
            (creature, actor, session),
            0,
            0,
        )
        .expect("lethal bite");
        let death = states
            .player_death(&runtime, actor, session)
            .expect("death");
        assert_eq!(
            states.monk_save_values(&runtime, actor, session, now(1000)),
            Some((3, 3_000_000))
        );
        // Control loss detaches the dead monk; only the respawn initializes it again.
        states.detach(&runtime, actor, session);
        let (_, vitals) = states
            .respawn(&runtime, actor, session, death.occurrence, now(2000))
            .expect("respawned");
        assert_eq!((vitals.harmony, vitals.serene), (0, true));
        assert_eq!(
            states.monk_save_values(&runtime, actor, session, now(2000)),
            Some((0, 0))
        );
        assert_eq!(
            cast_at(&runtime, &mut states, actor, session, 1, 2000).disposition,
            SpellCastDisposition::Cast
        );
    }

    /// A lethal hit whose death occurrence cannot be minted is refused with nothing written.
    #[test]
    fn a_lethal_hit_without_an_occurrence_writes_nothing() {
        use crate::ability::creature_bite::{BiteRejection, CreatureBiteLedger};

        let (runtime, mut states, actor, session, creature) = bitten_player(0x34, 8);
        states.mint_death = || None;
        let mut ledger = CreatureBiteLedger::default();
        assert_eq!(
            bite_at(
                &runtime,
                &mut states,
                &mut ledger,
                (creature, actor, session),
                0,
                0
            ),
            Err(BiteRejection::StaleTarget)
        );
        assert!(!states.is_dead(actor));
        let (revision, vitals) = observe_vitals(&runtime, &states, actor, session).expect("vitals");
        assert_eq!((revision, vitals.health), (1, 8));
    }
}
