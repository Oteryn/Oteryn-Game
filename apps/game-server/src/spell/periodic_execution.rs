//! Player periodic effects consume only ticks issued by the actual condition
//! store during this staged owner turn. They carry no client cast command.
use super::cast::PlayerSpellState;
use crate::ability::condition::{ConditionTick, TickFacts, TickKind};
use crate::foundation::{ExactActorRef, GameSessionId, RuntimeWorkStamp};
use oteryn_protocol_oteryn::actor_spell::SpellCastDisposition;
use oteryn_simulation_determinism::SemanticTimeMicros;

#[derive(Debug)]
pub(crate) struct PreparedPlayerPeriodicTurn {
    actor: ExactActorRef,
    session: GameSessionId,
    stamp: RuntimeWorkStamp,
    before: PlayerSpellState,
    next: PlayerSpellState,
    ticks: Vec<ConditionTick<String>>,
    publish: bool,
}
impl PreparedPlayerPeriodicTurn {
    pub(crate) fn actor(&self) -> ExactActorRef {
        self.actor
    }
    pub(crate) fn session(&self) -> GameSessionId {
        self.session
    }
    pub(crate) fn stamp(&self) -> RuntimeWorkStamp {
        self.stamp
    }
    pub(crate) fn predecessor(&self) -> &PlayerSpellState {
        &self.before
    }
    pub(crate) fn ticks(&self) -> &[ConditionTick<String>] {
        &self.ticks
    }
    pub(crate) fn into_successor(self) -> (PlayerSpellState, bool) {
        (self.next, self.publish)
    }
}
/// `damage_legal` is the live combat owner qualification, evaluated against
/// each real store-issued occurrence. Failure preserves the entire predecessor.
#[allow(clippy::too_many_arguments)]
pub(crate) fn stage_player_periodic_turn(
    state: &PlayerSpellState,
    actor: ExactActorRef,
    session: GameSessionId,
    stamp: RuntimeWorkStamp,
    now: SemanticTimeMicros,
    facts: TickFacts,
    mut damage_legal: impl FnMut(&ConditionTick<String>) -> bool,
) -> Result<Option<PreparedPlayerPeriodicTurn>, SpellCastDisposition> {
    let Some(cycle) =
        super::actor_conditions::stage_source_owner_cycle(state, now, Some(facts), None)?
    else {
        return Ok(None);
    };
    let mut next = cycle.next;
    for tick in &cycle.combat_ticks {
        match tick.kind {
            TickKind::Damage {
                amount,
                refused: false,
                ..
            } => {
                if !damage_legal(tick) {
                    return Err(SpellCastDisposition::TargetIllegal);
                }
                next = super::actor_conditions::stage_creature_hit(&next, amount, now.get())?.0;
            }
            TickKind::Damage { refused: true, .. }
            | TickKind::Regeneration {
                suppressed: true, ..
            } => {}
            // Food/Recovery need their actual owning source recipe; do not
            // infer a positive health or mana gain from an untyped tick.
            _ => return Err(SpellCastDisposition::Rejected),
        }
    }
    let changed_vitals = next.vitals() != state.vitals();
    let publish = cycle.publish_vitals || changed_vitals;
    next.revision = if publish {
        state
            .revision
            .checked_add(1)
            .ok_or(SpellCastDisposition::Rejected)?
    } else {
        state.revision
    };
    Ok(Some(PreparedPlayerPeriodicTurn {
        actor,
        session,
        stamp,
        before: state.clone(),
        next,
        ticks: cycle.combat_ticks,
        publish,
    }))
}
