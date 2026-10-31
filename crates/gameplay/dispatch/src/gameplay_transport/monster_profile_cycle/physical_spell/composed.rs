//! Current Map, creature-role and player-state facts for the existing composed cast owner.
//! Exact native source qualification and every HP mutation remain in their existing owners.
use super::*;
use crate::creature_damage_spell::composed::{
    BranchReceipt, ComposedSource, ComposedWorldReader, CreatureCombatFacts,
};
use crate::monster_summon::{CurrentSourceCreatureRole, current_source_creature_role};

pub(super) fn source_for(
    runtime: &ChannelRuntimeV1,
    native: &NativeGameplayState,
    proposal: &ProfileAbilityProposal,
) -> Option<ComposedSource> {
    let creature = runtime.companion_snapshot(proposal.issuer).ok()?;
    if !matches!(
        current_source_creature_role(runtime, proposal.issuer).ok()?,
        CurrentSourceCreatureRole::Ordinary | CurrentSourceCreatureRole::IntrinsicMonsterSummon(_)
    ) {
        return None;
    }
    let record = native.creature_profiles().records.iter().find(|record| {
        record.profile.target.key == creature.state.policy.definition_key
            && record.profile.target.revision == creature.state.policy.definition_revision
    })?;
    let definitions = record.attack_definitions.as_ref()?;
    ComposedSource::from_native_parts(
        &record.profile.target,
        proposal.entry_index,
        &definitions.records,
        &definitions.profiles,
        native.source_digest(),
    )
    .ok()
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn execute(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    source: &ComposedSource,
    proposal: &ProfileAbilityProposal,
    parties: &ReferenceParties,
    now: u64,
) {
    let Ok(stamp) = runtime.issue_owner_work() else {
        return;
    };
    let Some(base) = prepare_composed_reader(
        runtime, states, attack, room, native, map, source, proposal, parties, now, stamp,
    )
    .await
    else {
        return;
    };
    let Some(mut reader) = CurrentComposed::prepare(runtime, attack, room, source, base, now)
    else {
        return;
    };
    let Ok(catalog) =
        crate::creature_area_heal::CreatureHealCatalog::from_current_native(runtime, native)
    else {
        return;
    };
    let mut owner = std::mem::take(&mut states.monster_composed);
    let result = owner.execute_current(
        runtime,
        stamp,
        states,
        &catalog,
        source,
        proposal,
        &mut reader,
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
    );
    states.monster_composed = owner;
    let Ok(outcome) = result else {
        return;
    };
    if let Some(primary) = outcome.primary {
        record_hits(runtime, attack, &reader.base.players, primary, now);
    }
    let semantic_now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now);
    for (_, target, branch) in outcome.branches {
        if branch.is_ok_and(
            |receipt| matches!(receipt, BranchReceipt::PlayerDamage(ref hit) if hit.applied > 0),
        ) && let Some((_, session)) = reader
            .base
            .players
            .iter()
            .find(|(actor, _)| *actor == target)
        {
            attack.record_hit_taken(runtime, target, *session, semantic_now);
        }
    }
}

struct CurrentCreature {
    actor: ExactActorRef,
    non_player_side: bool,
    callback_allowed: bool,
    primary: CreatureCombatFacts,
    primary_requires_heal: bool,
}

struct CurrentComposed<'native> {
    base: CurrentTarget<'native>,
    ability: crate::content::ProjectV2DefinitionRef,
    creatures: Vec<CurrentCreature>,
    primary_conditions: bool,
}

impl<'native> CurrentComposed<'native> {
    fn prepare(
        runtime: &ChannelRuntimeV1,
        attack: &mut ChannelAttackStates,
        room: &QualifiedNativeEntryRoom,
        source: &ComposedSource,
        base: CurrentTarget<'native>,
        now: u64,
    ) -> Option<Self> {
        let from = runtime.read_actor_position(base.issuer).ok()?;
        let issuer_tile = room
            .movement_cells()
            .spell_tiles()
            .lookup(
                room.movement_cells().scope(),
                crate::content::LogicalCell {
                    x: from.position().x,
                    y: from.position().y,
                    z: i32::from(from.position().floor),
                },
            )
            .ok()?;
        let primary = source.primary_source();
        let mut creatures = Vec::new();
        let semantic_now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now);
        for (actor, position, session) in runtime.positioned_actor_census().ok()? {
            if session.is_some() || position.context() != from.context() {
                continue;
            }
            let at = position.position();
            let Some(tile) = base.tiles.get(&(at.x, at.y, at.floor)) else {
                continue;
            };
            if !(*tile)? {
                continue;
            }
            if creatures.len() >= 64 {
                return None;
            }
            let snapshot = runtime.companion_snapshot(actor).ok()?;
            let role = current_source_creature_role(runtime, actor).ok()?;
            let non_player_side = matches!(
                role,
                CurrentSourceCreatureRole::Ordinary
                    | CurrentSourceCreatureRole::IntrinsicMonsterSummon(_)
            );
            // The same independently current player protection protects its owned creature.
            // A missing/stale master never turns a companion into an unprotected monster.
            let protected = if non_player_side {
                false
            } else {
                let master = snapshot.state.master?;
                let control = runtime
                    .player_control_facts(master.actor, master.session)
                    .ok()?;
                control.control_loss.is_some()
                    || attack.creature_target_protection(
                        runtime,
                        master.actor,
                        master.session,
                        semantic_now,
                    )?
            };
            let allowed = !issuer_tile.flags().protection_zone
                && !protected
                && runtime.assert_actor_spell_unreserved(actor).is_ok()
                && runtime.assert_actor_spell_unreserved(base.issuer).is_ok();
            // Crystal 201189f combat.cpp:545-563: an ordinary monster's primary
            // combat does not hit another monster on its side. Explicit source
            // callbacks retain their separately authored named/side filters.
            let primary_allowed =
                allowed && !non_player_side && snapshot.state.policy.flags.attackable;
            let armor = if primary_allowed && primary.is_some_and(|p| p.armor) {
                snapshot.state.policy.armor?
            } else {
                0
            };
            let immune = base.element.as_deref().is_some_and(|element| {
                snapshot
                    .state
                    .policy
                    .damage_immunities
                    .iter()
                    .any(|key| key == element)
            });
            let primary_requires_heal = base.element.as_deref().is_some_and(|element| {
                snapshot
                    .state
                    .policy
                    .healing_from_damage
                    .iter()
                    .any(|response| {
                        response.damage_type == element
                            && (response.percent.numerator != 0
                                || response.percent.denominator == 0)
                    })
            });
            creatures.push(CurrentCreature {
                actor,
                non_player_side,
                callback_allowed: allowed,
                primary: CreatureCombatFacts {
                    allowed: primary_allowed,
                    immune,
                    armor,
                    // The response hook below delegates the actual magnitude to
                    // the existing creature rule; this factor is never reapplied.
                    multiplier_ppm: 1_000_000,
                },
                primary_requires_heal,
            });
        }
        Some(Self {
            base,
            ability: source.ability.clone(),
            creatures,
            primary_conditions: primary.is_some_and(|p| !p.conditions.is_empty()),
        })
    }

    fn is_current(&self, runtime: &ChannelRuntimeV1, stamp: RuntimeWorkStamp) -> bool {
        stamp == self.base.stamp
            && runtime.content_pin().server_artifact_digest() == self.base.native.source_digest()
            && runtime
                .owner_fence()
                .is_ok_and(|fence| fence.accepts_stamp(stamp))
    }
}

impl SpellWorldReader for CurrentComposed<'_> {
    fn native_gameplay<'a>(
        &'a self,
        runtime: &ChannelRuntimeV1,
    ) -> Option<&'a NativeGameplayState> {
        self.base.native_gameplay(runtime)
    }

    fn current_players(
        &mut self,
        runtime: &ChannelRuntimeV1,
        stamp: RuntimeWorkStamp,
    ) -> Option<Vec<(ExactActorRef, GameSessionId)>> {
        self.is_current(runtime, stamp).then_some(())?;
        self.base.current_players(runtime, stamp)
    }

    fn current_facing(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        stamp: RuntimeWorkStamp,
    ) -> Option<Facing> {
        self.is_current(runtime, stamp).then_some(())?;
        self.base.current_facing(runtime, issuer, stamp)
    }

    fn tile_allowed(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        x: i32,
        y: i32,
        floor: i16,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.is_current(runtime, stamp).then_some(())?;
        self.base.tile_allowed(runtime, issuer, x, y, floor, stamp)
    }

    fn combat(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        target: ExactActorRef,
        session: GameSessionId,
        element: Option<&str>,
        stamp: RuntimeWorkStamp,
    ) -> Option<SpellCombatFacts> {
        self.is_current(runtime, stamp).then_some(())?;
        // A source callback reads the very same current player/PZ/visibility facts.
        // Its raw health operation does not invoke the primary elemental reduction.
        let primary_element = self.base.element.clone();
        self.base.combat(
            runtime,
            issuer,
            target,
            session,
            element.or(primary_element.as_deref()),
            stamp,
        )
    }
}

impl ComposedWorldReader for CurrentComposed<'_> {
    fn creature_combat(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        target: ExactActorRef,
        element: Option<&str>,
        stamp: RuntimeWorkStamp,
    ) -> Option<CreatureCombatFacts> {
        if !self.is_current(runtime, stamp)
            || issuer != self.base.issuer
            || element != self.base.element.as_deref()
        {
            return None;
        }
        let creature = self.creatures.iter().find(|facts| facts.actor == target)?;
        let facts = creature.primary;
        // This owner has no atomic creature primary-plus-condition commit. Never
        // silently drop such a secondary branch for a player-owned creature.
        if facts.allowed && (self.primary_conditions || creature.primary_requires_heal) {
            return None;
        }
        Some(facts)
    }

    fn creature_damage_response(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        target: ExactActorRef,
        element: Option<&str>,
        amount: u32,
        _: CreatureCombatFacts,
        stamp: RuntimeWorkStamp,
    ) -> Option<u32> {
        self.creature_combat(runtime, issuer, target, element, stamp)?;
        if amount == 0 {
            return Some(0);
        }
        let element = element?;
        let snapshot = runtime.companion_snapshot(target).ok()?;
        // A damage-derived heal needs its own existing composed receipt. Do not
        // degrade that source policy into ordinary elemental damage.
        if snapshot
            .state
            .policy
            .damage_healing(element, i64::from(amount))
            .ok()?
            != 0
        {
            return None;
        }
        u32::try_from(
            oteryn_gameplay_combat_rules::resolution::creature_damage_response(
                &snapshot.state.policy,
                i64::from(amount),
                element,
            )?,
        )
        .ok()
    }

    fn callback_allowed(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        target: ExactActorRef,
        ability: &crate::content::ProjectV2DefinitionRef,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        if !self.is_current(runtime, stamp)
            || issuer != self.base.issuer
            || ability != &self.ability
        {
            return None;
        }
        self.creatures
            .iter()
            .find(|facts| facts.actor == target)
            .map(|facts| facts.callback_allowed)
    }

    fn non_player_side(
        &mut self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.is_current(runtime, stamp).then_some(())?;
        self.creatures
            .iter()
            .find(|facts| facts.actor == target)
            .map(|facts| facts.non_player_side)
    }

    fn top_creature(
        &mut self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        stamp: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.is_current(runtime, stamp).then_some(())?;
        // The existing runtime has no same-tile actor stack order. One exact
        // occupant proves top; the existing owner also counts temporarily
        // uncontrolled/dead/unqualified occupants, not only combat candidates.
        runtime
            .is_sole_current_actor_on_cell(target)
            .ok()?
            .then_some(true)
    }
}
