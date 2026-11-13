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
    let profiles = super::native_source_profiles(
        native,
        record,
        matches!(
            runtime.binding().runtime_scope(),
            crate::foundation::RuntimeScopeRefV1::Instance { .. }
        ),
    )?;
    ComposedSource::from_native_parts(
        &record.profile.target,
        proposal.entry_index,
        &definitions.records,
        &profiles,
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
    charms: &crate::content::charm_source::CanonicalCharmCatalogue,
    native_health_names: &crate::gameplay_transport::source_names::NativeHealthNamesData,
) -> Result<(), ()> {
    let stamp = runtime.issue_owner_work().map_err(|_| ())?;
    let prepared = prepare_source_reader_checked(
        runtime,
        states,
        attack,
        room,
        native,
        map,
        SourceShape::Composed(source),
        proposal,
        parties,
        now,
        stamp,
        false,
        None,
        None,
    )
    .await;
    let base = match prepared {
        Ok(base) => base,
        Err(PreparationError::Unavailable) => return Err(()),
        Err(PreparationError::Refused) => {
            if !map.validate_transaction(runtime).await {
                return Err(());
            }
            return Ok(());
        }
    };
    let Some(mut reader) = CurrentComposed::prepare(runtime, attack, room, source, base, now)
    else {
        return Ok(());
    };
    if reader
        .bind_native_health_names(runtime, native_health_names)
        .is_none()
    {
        return Ok(());
    }
    let Ok(catalog) =
        crate::creature_area_heal::CreatureHealCatalog::from_current_native(runtime, native)
    else {
        return Ok(());
    };
    let mut owner = std::mem::take(&mut states.monster_composed);
    let result = owner.execute_current(
        runtime,
        stamp,
        &mut super::super::super::attack::charm_combat::source_vitals::CharmSourceVitals {
            states,
            attack,
            catalogue: charms,
            root: GameplayDecisionRoot::from_bytes(native.source_digest()),
        },
        &catalog,
        source,
        proposal,
        &mut reader,
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
    );
    states.monster_composed = owner;
    let Ok(outcome) = result else {
        return Ok(());
    };
    if let Some(primary) = outcome.primary {
        record_hits(
            runtime,
            attack,
            Some(proposal.issuer),
            &reader.base.players,
            primary,
            now,
        );
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
    Ok(())
}

struct CurrentCreature {
    actor: ExactActorRef,
    non_player_side: bool,
    callback_allowed: bool,
    primary: CreatureCombatFacts,
    primary_requires_heal: bool,
}

pub(in crate::gameplay_transport::monster_profile_cycle) struct CurrentComposed<'native, 'turn> {
    pub(in crate::gameplay_transport::monster_profile_cycle) base: CurrentTarget<'native>,
    ability: crate::content::ProjectV2DefinitionRef,
    creatures: Vec<CurrentCreature>,
    primary_conditions: bool,
    native_health_names:
        Option<&'turn crate::gameplay_transport::source_names::NativeHealthNamesData>,
}

impl<'native, 'turn> CurrentComposed<'native, 'turn> {
    pub(in crate::gameplay_transport::monster_profile_cycle) fn prepare(
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
            native_health_names: None,
        })
    }

    pub(in crate::gameplay_transport::monster_profile_cycle) fn bind_native_health_names(
        &mut self,
        runtime: &ChannelRuntimeV1,
        names: &'turn crate::gameplay_transport::source_names::NativeHealthNamesData,
    ) -> Option<()> {
        names.validate_frame(runtime).ok()?;
        self.native_health_names = Some(names);
        Some(())
    }

    fn is_current(&self, runtime: &ChannelRuntimeV1, stamp: RuntimeWorkStamp) -> bool {
        stamp == self.base.stamp
            && runtime.content_pin().server_artifact_digest() == self.base.native.source_digest()
            && runtime
                .owner_fence()
                .is_ok_and(|fence| fence.accepts_stamp(stamp))
    }
}

impl SpellWorldReader for CurrentComposed<'_, '_> {
    fn source_damage_draw(
        &mut self,
        minimum: u32,
        maximum: u32,
        draw: u64,
        target: Option<ExactActorRef>,
    ) -> Result<Option<u32>, crate::creature_auto_attack::AttackError> {
        self.base.source_damage_draw(minimum, maximum, draw, target)
    }
    fn source_combat_component_damage(
        &mut self,
        runtime: &ChannelRuntimeV1,
        issuer: ExactActorRef,
        target: ExactActorRef,
        magnitude: u32,
        now: u64,
        stamp: RuntimeWorkStamp,
    ) -> Option<u32> {
        self.is_current(runtime, stamp).then_some(())?;
        self.base
            .source_combat_component_damage(runtime, issuer, target, magnitude, now, stamp)
    }
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

impl ComposedWorldReader for CurrentComposed<'_, '_> {
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
        let prepared = crate::creature_damage_spell::native_event::prepare_native_creature_damage(
            &snapshot.state.policy,
            element,
            amount,
        )?;
        if runtime.native_health_event(target).ok()?.is_some_and(|event| {
            event.kind() == oteryn_gameplay_content::content::native_monster_callbacks::NativeHealthEventKind::HealthForgotten
        }) {
            let observation = self.native_health_names?.observe(runtime, target).ok()??;
            runtime.validate_native_health_change_observation(&observation).ok()?;
            let signed = observation.primary(-i64::from(prepared)).ok()?;
            u32::try_from(signed.checked_neg()?).ok()
        } else {
            Some(prepared)
        }
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
        source_top_creature(runtime, target)
    }
}

fn source_top_creature(runtime: &ChannelRuntimeV1, target: ExactActorRef) -> Option<bool> {
    runtime.is_current_top_actor(target).ok()
}

#[cfg(test)]
mod source_top_tests {
    use super::*;
    #[test]
    fn srx3_composed_source_callback_reads_actual_top_of_stacked_actors()
    -> Result<(), Box<dyn std::error::Error>> {
        let id = |tag: u8| {
            let mut value = [tag; 16];
            value[6] = 0x70;
            value[8] = 0x80;
            value
        };
        let world = crate::foundation::WorldId::decode(&id(0x81))?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            crate::foundation::ChannelId::decode(&id(0x82))?,
            crate::foundation::NodeId::decode(&id(0x83))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            crate::foundation::ChannelContentPin::from_activation(
                world,
                1,
                [1; 32],
                [2; 32],
                [3; 32],
                [4; 32],
                (10, 10, 7),
            ),
        )?;
        let mut actors = Vec::new();
        for tag in [0x84, 0x85] {
            let reservation = runtime.reserve_fresh_session(GameSessionId::decode(&id(tag))?)?;
            let actor = runtime.commit_fresh_session(reservation)?;
            runtime.initialize_pinned_test_position(
                actor,
                MovementLocalPosition {
                    x: 10,
                    y: 10,
                    floor: 7,
                },
            )?;
            actors.push(actor);
        }
        assert_eq!(
            source_top_creature(&runtime, actors[1]),
            Some(true),
            "source callback must see latest canonical tile entry even with another actor below it"
        );
        assert_eq!(
            source_top_creature(&runtime, actors[0]),
            Some(false),
            "lower actor is known not-top, not missing combat facts"
        );
        Ok(())
    }
}
