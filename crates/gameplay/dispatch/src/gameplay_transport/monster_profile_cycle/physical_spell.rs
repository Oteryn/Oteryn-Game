//! First native scheduled spell consumer: single-target physical damage.
//! Timing, chance, damage, replay and lethal consequences stay in their existing owners.
use super::*;
use crate::creature_auto_attack::{AttackFacts, ProfileAbilityProposal};
use crate::creature_damage_spell::{SpellCombatFacts, SpellSource, SpellWorldReader};
use crate::foundation::{ExactActorRef, GameSessionId, MovementLocalPosition, RuntimeWorkStamp};
use oteryn_gameplay_content::content::creature_attack_geometry::Facing;

struct CurrentTarget {
    stamp: RuntimeWorkStamp,
    players: Vec<(ExactActorRef, GameSessionId)>,
    position: MovementLocalPosition,
    facts: SpellCombatFacts,
    tile_allowed: Option<bool>,
}

impl SpellWorldReader for CurrentTarget {
    fn current_players(
        &mut self,
        _: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
        (stamp == self.stamp).then(|| self.players.clone())
    }

    fn current_facing(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: RuntimeWorkStamp,
    ) -> Option<Facing> {
        None // Directional sources are refused before this reader is constructed.
    }

    fn tile_allowed(
        &mut self,
        _: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        x: i32,
        y: i32,
        floor: i16,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        if stamp == self.stamp
            && issuer == self.facts.attack.issuer
            && self.position == (MovementLocalPosition { x, y, floor })
        {
            self.tile_allowed
        } else {
            None
        }
    }

    fn combat(
        &mut self,
        _: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        element: Option<&str>,
        stamp: RuntimeWorkStamp,
    ) -> Option<SpellCombatFacts> {
        (stamp == self.stamp
            && issuer == self.facts.attack.issuer
            && target == self.facts.attack.target
            && session == self.facts.attack.session
            && element == Some("physical"))
        .then(|| self.facts.clone())
    }
}

// Runtime, Item transaction and player states stay locked from these independent reads
// through execution. This does not admit area, elemental, secondary or delayed effects.
#[allow(clippy::too_many_arguments)]
pub(super) async fn execute(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    proposals: &[ProfileAbilityProposal],
    now: u64,
) {
    for proposal in proposals {
        let Ok(creature) = runtime.companion_snapshot(proposal.issuer) else {
            continue;
        };
        if creature.state.master.is_some() || creature.state.policy.is_familiar {
            continue;
        }
        let Some(record) = native.creature_profiles().records.iter().find(|record| {
            record.profile.target.key == creature.state.policy.definition_key
                && record.profile.target.revision == creature.state.policy.definition_revision
        }) else {
            continue;
        };
        let Some(definitions) = &record.attack_definitions else {
            continue;
        };
        let Ok(source) = SpellSource::from_native(
            &record.profile.target,
            proposal.entry_index,
            &definitions.records,
            &definitions.profiles,
            native.source_digest(),
        ) else {
            continue;
        };
        if proposal.list != crate::ai_think::profile_schedule::ScheduleList::Attack
            || source.element.as_deref() != Some("physical")
            || source.area.is_some()
            || !source.needs_target
            || source.needs_direction
            || source.chain.is_some()
            || source.windup.is_some()
            || !source.variants.is_empty()
            || !source.conditions.is_empty()
            || !source.appearances.is_empty()
        {
            continue;
        }
        let Ok(census) = runtime.positioned_melee_census() else {
            continue;
        };
        let Some(session) = census
            .iter()
            .find(|(actor, _, _)| *actor == proposal.target)
            .and_then(|(_, _, session)| *session)
        else {
            continue;
        };
        // This source shape has exactly one target; read its current committed session.
        let players = vec![(proposal.target, session)];
        if states.is_dead(proposal.target)
            || states.has_pending_spell_commit(proposal.target, session)
            || runtime
                .assert_actor_spell_unreserved(proposal.target)
                .is_err()
        {
            continue;
        }
        let (Ok(from), Ok(to)) = (
            runtime.read_actor_position(proposal.issuer),
            runtime.read_actor_position(proposal.target),
        ) else {
            continue;
        };
        let from = from.position();
        let to = to.position();
        let tile = |position: MovementLocalPosition| {
            room.movement_cells().spell_tiles().lookup(
                room.movement_cells().scope(),
                crate::content::LogicalCell {
                    x: position.x,
                    y: position.y,
                    z: i32::from(position.floor),
                },
            )
        };
        let (Ok(issuer_tile), Ok(target_tile)) = (tile(from), tile(to)) else {
            continue;
        };
        let Some(state) = states.get(runtime, proposal.target, session) else {
            continue;
        };
        if state.vitals().health == 0 {
            continue;
        }
        let Ok(invisible) = state.owned_invisible_at(now) else {
            continue;
        };
        let Some(crate::content::ProjectV2AuthoringProfileData::Behavior(behavior)) =
            record.behavior.as_ref().map(|p| &p.data)
        else {
            continue;
        };
        let semantic_now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now);
        let Some(protected) =
            attack.creature_target_protection(runtime, proposal.target, session, semantic_now)
        else {
            continue;
        };
        let multiplier = match state.equipped_physical_protection() {
            Some(protection) => {
                oteryn_gameplay_items::equipment_effects::apply_protection(1_000_000, protection)
            }
            None => Some(1_000_000),
        };
        let Some(multiplier_ppm) = multiplier.and_then(|value| u32::try_from(value).ok()) else {
            continue;
        };
        let facts = SpellCombatFacts {
            attack: AttackFacts {
                issuer: proposal.issuer,
                target: proposal.target,
                session,
                revision: state.revision(),
                visible: !invisible || behavior.targeting.sense_invisible,
                issuer_pz: issuer_tile.flags().protection_zone,
                target_pz: target_tile.flags().protection_zone,
                issuer_protected: false,
                target_protected: protected,
                defense: 0,
                armor: state.equipped_armor(),
            },
            multiplier_ppm,
            immune: false,
            condition_policy: None,
        };
        map.read_sight(runtime, from, to).await;
        if !map.validate_transaction(runtime).await {
            return;
        }
        let Ok(stamp) = runtime.issue_owner_work() else {
            return;
        };
        let sight = crate::durability::native_world_read::source_sight_steps(from, to)
            .ok()
            .and_then(|steps| {
                steps
                    .iter()
                    .all(|(at, exempt)| *exempt || map.view().known(*at))
                    .then(|| map.view().sight(from, to) && !target_tile.flags().floor_change)
            });
        let mut reader = CurrentTarget {
            stamp,
            players,
            position: to,
            facts,
            tile_allowed: sight,
        };
        let mut spells = std::mem::take(&mut states.monster_spells);
        let result = spells.execute_current_physical(
            runtime,
            stamp,
            states,
            &source,
            proposal,
            &mut reader,
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
        );
        states.monster_spells = spells;
        if result.is_ok_and(|outcome| {
            outcome.targets.iter().any(|(_, hit)| {
                hit.as_ref()
                    .is_ok_and(|hit| hit.damage.as_ref().is_some_and(|damage| damage.applied > 0))
            })
        }) {
            attack.record_hit_taken(runtime, proposal.target, session, semantic_now);
        }
    }
}
