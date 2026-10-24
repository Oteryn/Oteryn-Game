use std::num::NonZeroU32;

use super::*;
use crate::foundation::{ChannelContentPin, ChannelId, NodeId, WorldId};
use crate::spell::Vocation;
#[cfg(test)]
use oteryn_gameplay_domain::domain::condition::ConditionGate;

const fn uuid_v7(tag: u8) -> [u8; 16] {
    [
        0x01, 0x90, 0x00, 0x00, 0x00, tag, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, tag,
    ]
}

/// A Channel runtime with one committed player actor bound to the session `tag`.
pub fn runtime_with_player(tag: u8) -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
    runtime_with_capacity(tag, 2)
}

// Source multi-child fixtures use the SAME existing committed-assignment owner with enough
// physical slots; legacy two-slot tests keep their original capacity. No production grant.
pub fn runtime_with_capacity(
    tag: u8,
    capacity: usize,
) -> (ChannelRuntimeV1, ExactActorRef, GameSessionId) {
    let world = WorldId::decode(&uuid_v7(0x60)).expect("world");
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&uuid_v7(0x61)).expect("channel"),
        NodeId::decode(&uuid_v7(0x62)).expect("node"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        capacity,
        ChannelContentPin::test(world),
    )
    .expect("channel runtime");
    let session = GameSessionId::decode(&uuid_v7(tag)).expect("session");
    let reservation = runtime.reserve_fresh_session(session).expect("reserve");
    let actor = runtime.commit_fresh_session(reservation).expect("commit");
    (runtime, actor, session)
}

/// Level 8 druid facts, as the Character owner would supply them.
pub const FACTS: CharacterCastFacts = CharacterCastFacts {
    vocation: Vocation::Druid,
    level: 8,
    magic_level: 0,
    max_health: 185,
    max_mana: 90,
    max_soul: 100,
};

/// Test only: no damage owner exists yet, so a wounded actor is staged by writing its health.
pub fn wound(
    states: &mut ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    health: u32,
) {
    let index = states.index(actor, session).expect("present");
    states.actors[index].2.set_health_for_test(health);
}

/// `exura` (index 3 of the canonical V1 book) with no target.
pub fn exura() -> SpellCastIntent {
    SpellCastIntent {
        spell: NonZeroU32::new(3).expect("index"),
        target: SpellTarget::None,
        aim_at_target: false,
    }
}

#[cfg(test)]
fn now(millis: u64) -> SemanticTimeMicros {
    SemanticTimeMicros::from_micros(millis * 1000)
}

#[cfg(test)]
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

/// Test only: `gate` (a §3.5 key) for 10 s from time 0 in the player store.
#[cfg(test)]
pub(in crate::gameplay_transport) fn gate_player(
    states: &mut ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    gate: &str,
) {
    use oteryn_gameplay_domain::domain::condition::ConditionValues;
    use oteryn_gameplay_foundation::foundation::condition::ApplicationFacts;
    use oteryn_gameplay_foundation::foundation::condition::ConditionDefinition;
    use oteryn_gameplay_foundation::foundation::condition::ConditionSourceKind;
    let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([5; 32]);
    let facts = ApplicationFacts {
        now: 0,
        base_speed: 220,
        mana_shield_capacity: 0,
        target_reentry_protected: false,
        source_reentry_protected: false,
        target_is_player: true,
        decision_root: &root,
        occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([6; 16]),
    };
    let values = ConditionValues::gate(gate, 10_000).expect("gate family");
    let definition = ConditionDefinition::new("cond.gate", 1, values).expect("definition");
    let index = states.index(actor, session).expect("present");
    let state = &mut states.actors[index].2;
    let before = state.owned_conditions().clone();
    let mut next = before.clone();
    next.apply(
        &definition,
        Some("creature:test".to_owned()),
        ConditionSourceKind::Creature,
        &[],
        &facts,
    )
    .expect("applied");
    state
        .apply_batch_conditions(&before, &next)
        .expect("installed");
}
