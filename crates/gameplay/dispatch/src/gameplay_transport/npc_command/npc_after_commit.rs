//! Source effects owned by the successful native NPC dialogue receipt.
use super::super::{ComposedFreshAdmission, connection::UseCommand};
use super::*;
use crate::durability::reward_claim_mint::QuestActorConditionProof;
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionProvenance, ConditionSourceKind, DamageSchedule,
    DamageSegment,
};
use oteryn_gameplay_quests::quest::reward_callbacks::{NpcAfterCommit, NpcFireDamage};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

const NPC_FIRE: &str = "npc.dialogue.fire";

pub(in crate::gameplay_transport) fn is_npc_fire_damage(
    provenance: &ConditionProvenance<String>,
) -> bool {
    provenance.source_kind == ConditionSourceKind::SelfUse
        && provenance.definition_key == NPC_FIRE
        && provenance.source.is_none()
        && provenance.historical_source.is_none()
        && provenance.lineage.is_none()
}

pub(super) fn temporary_look(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    command: UseCommand,
) -> Option<u32> {
    let native = admission.active_generation?.native_gameplay()?;
    let states = admission.states.spell_states.try_lock().ok()?;
    let state = states.get(runtime, actor, command.game_session_id)?;
    QuestActorConditionProof::current_temporary_look(
        runtime,
        actor,
        command.game_session_id,
        native,
        state.owned_conditions(),
        admission.owner_now().get(),
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "the receipt retains the exact NPC and current customer"
)]
pub(super) fn publish(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    command: UseCommand,
    intent: &NpcTalkIntent,
    context: &quest_dialogue::Context,
    effects: &NpcAfterCommit,
    response: &mut NpcTalkOutcome,
) {
    let session = command.game_session_id;
    let Some(fence) = command.item_fence else {
        return;
    };
    if !runtime
        .bound_attacker_lease(actor, session)
        .is_ok_and(|lease| {
            lease.character_id().as_bytes() == fence.character_id.as_bytes()
                && lease.generation() == fence.character_lease_generation
        })
    {
        return;
    }
    if let Some(fire) = &effects.fire_damage {
        apply_fire(admission, runtime, actor, command, fire);
    }
    // Crystal 201189f8 data-global/npc/the_orc_king.lua:48-59 births eight public creatures.
    if !effects.nearby_creatures.is_empty()
        && let Some(npc) =
            super::service_context::live_npc(runtime, actor, session, intent.npc_actor)
    {
        spawn_near_npc(
            admission,
            runtime,
            command,
            npc.position,
            &effects.nearby_creatures,
        );
    }
    // Crystal 201189f8 data-global/npc/kazzan.lua:53-67.
    if let Some(delay) = effects.end_conversation_delay_ms
        && admission
            .states
            .npc_conversations
            .current_npc(&admission.states.chat, runtime, actor, session, intent)
            .as_ref()
            == Some(context)
        && let Some(npc) = admission.states.chat.engaged(session)
    {
        super::catalogue::end_response(admission, session, npc, u64::from(delay), response);
    }
}

// Crystal 201189f8 data-global/npc/wyda.lua:52-54,87-103.
fn apply_fire(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    command: UseCommand,
    fire: &NpcFireDamage,
) -> Option<()> {
    use crate::domain::condition::DotElement;
    let session = command.game_session_id;
    let mut states = admission.states.spell_states.try_lock().ok()?;
    if states.is_dead(actor) || !states.spell_idle(runtime, actor, session) {
        return None;
    }
    let state = states.get(runtime, actor, session)?;
    let before = state.owned_conditions();
    let mut after = before.clone();
    let condition = ConditionDefinition::new_damage_schedule(
        NPC_FIRE,
        1,
        DotElement::Fire,
        DamageSchedule::Fixed {
            delayed: true,
            segments: vec![DamageSegment {
                count: u32::from(fire.ticks),
                interval_ms: fire.interval_ms,
                amount: u32::from(fire.amount),
            }],
        },
    )?;
    let now = admission.owner_now().get();
    let mut occurrence = actor.placement_identity();
    occurrence[8..].copy_from_slice(&command.command_id.to_be_bytes());
    let decision_root =
        GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    after
        .apply(
            &condition,
            None,
            ConditionSourceKind::SelfUse,
            &[],
            &ApplicationFacts {
                now,
                base_speed: u16::try_from(state.owned_base_speed()).ok()?,
                mana_shield_capacity: 0,
                target_reentry_protected: runtime
                    .current_player_reentry_protection(actor, session, now)
                    .ok()?,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &decision_root,
                occurrence: DecisionOccurrenceId::from_bytes(occurrence),
            },
        )
        .ok()?;
    let mut next = state.clone();
    next.apply_batch_conditions(before, &after).ok()?;
    next.advance_batch_revision().ok()?;
    states.commit(runtime, actor, session, next).then_some(())
}

fn spawn_near_npc(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    command: UseCommand,
    origin: super::service_context::Position,
    creatures: &[String],
) {
    use oteryn_gameplay_quests::quest::world_callbacks::{CreatureSpawn, Position};
    let (Some(map), Ok(states)) = (
        admission.bundle_map(),
        admission.states.spell_states.try_lock(),
    ) else {
        return;
    };
    let Some(floor) = origin
        .floor
        .checked_neg()
        .and_then(|floor| i8::try_from(floor).ok())
    else {
        return;
    };
    // Crystal src/map/map.cpp:234-304 tries the center then one ring of neighbors.
    // Offset the ring's starting point per occurrence so a fixed compass direction
    // does not monopolize the first placement.
    let ring = [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ];
    let offset = (command.command_id % 8) as usize;
    for creature in creatures {
        let positions =
            std::iter::once((0, 0)).chain(ring[offset..].iter().chain(&ring[..offset]).copied());
        for (dx, dy) in positions {
            let (Some(x), Some(y)) = (
                origin.x.checked_add(dx).and_then(|x| u16::try_from(x).ok()),
                origin.y.checked_add(dy).and_then(|y| u16::try_from(y).ok()),
            ) else {
                continue;
            };
            if admission.spawn_quest_creature(
                runtime,
                &states,
                map,
                &CreatureSpawn {
                    creature: creature.clone(),
                    position: Position { x, y, floor },
                    lifetime_ms: None,
                },
            ) {
                break;
            }
        }
    }
}
