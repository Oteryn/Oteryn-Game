//! Native scheduled player-targeted attacks from every admitted creature's source closure.
//! Timing, chance, damage, replay and lethal consequences stay in their existing owners.
use super::*;
use crate::creature_auto_attack::{AttackFacts, CurrentConditionPolicy, ProfileAbilityProposal};
use crate::creature_damage_spell::{SpellCombatFacts, SpellSource, SpellWorldReader};
use crate::foundation::{ExactActorRef, GameSessionId, MovementLocalPosition, RuntimeWorkStamp};
use oteryn_gameplay_content::content::creature_attack_geometry::Facing;
use std::collections::BTreeMap;

mod composed;
mod timers;

struct CurrentTarget<'native> {
    native: &'native NativeGameplayState,
    stamp: RuntimeWorkStamp,
    players: Vec<(ExactActorRef, GameSessionId)>,
    issuer: ExactActorRef,
    element: Option<String>,
    facing: Facing,
    facts: Vec<SpellCombatFacts>,
    tiles: BTreeMap<(i32, i32, i16), Option<bool>>,
}

impl SpellWorldReader for CurrentTarget<'_> {
    fn native_gameplay<'a>(
        &'a self,
        runtime: &ChannelRuntimeV1,
    ) -> Option<&'a NativeGameplayState> {
        (runtime.content_pin().server_artifact_digest() == self.native.source_digest())
            .then_some(self.native)
    }

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
        issuer: ExactActorRef,
        stamp: RuntimeWorkStamp,
    ) -> Option<Facing> {
        (stamp == self.stamp && issuer == self.issuer).then_some(self.facing)
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
        (stamp == self.stamp && issuer == self.issuer)
            .then(|| self.tiles.get(&(x, y, floor)).copied().flatten())
            .flatten()
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
        if stamp != self.stamp || issuer != self.issuer || element != self.element.as_deref() {
            return None;
        }
        self.facts
            .iter()
            .find(|facts| facts.attack.target == target && facts.attack.session == session)
            .cloned()
    }
}

// Same-transaction strong World membership; never authorize party membership from the actor cache.
type ReferenceParties = BTreeMap<[u8; 16], Vec<(MovementLocalPosition, u32)>>;
pub(crate) async fn read_reference_parties(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &crate::durability::spell_item_transaction::SpellItemScopeAuthority,
    root: &crate::durability::DurabilityRoot,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    enabled: bool,
) -> ReferenceParties {
    let mut parties = BTreeMap::new();
    if !enabled {
        return parties;
    }
    let Ok(census) = runtime.positioned_actor_census() else {
        return parties;
    };
    // Resolve each actor's Character from the journal once. Membership never comes from a cache.
    let mut views = Vec::new();
    let mut physical = BTreeMap::new();
    for (actor, position, session) in census {
        let Some(session) = session else {
            continue;
        };
        let Ok(view) =
            crate::durability::world_party::read_world_party_for_scope_actor_in_transaction(
                tx, root, authority, runtime, actor, session,
            )
            .await
        else {
            return BTreeMap::new();
        };
        physical.insert(
            view.character,
            (
                position.position(),
                states
                    .get(runtime, actor, session)
                    .map(|state| state.character_facts().level),
            ),
        );
        views.push((actor, view));
    }
    for (actor, view) in views {
        if view.party_id.is_none() {
            continue;
        }
        let mut members = Vec::new();
        let mut available = true;
        // World membership is bounded to 50 principals. Physical data is joined once per member,
        // rather than rescanning all local actors or issuing another query for each pair.
        for member in &view.members {
            match physical.get(&member.character) {
                Some((position, Some(level)))
                    if member.presence != crate::foundation::PartyPresence::Offline
                        && (member.presence != crate::foundation::PartyPresence::Channel
                            || member.channel
                                == Some(*runtime.binding().channel_id().as_bytes())) =>
                {
                    members.push((*position, *level));
                }
                Some(_) => {
                    available = false;
                    break;
                }
                None if member.presence == crate::foundation::PartyPresence::Channel
                    && member.channel == Some(*runtime.binding().channel_id().as_bytes()) =>
                {
                    available = false;
                    break;
                }
                None => {}
            }
        }
        if available {
            parties.insert(actor.placement_identity(), members);
        }
    }
    parties
}

// Runtime, Item transaction and player states stay locked from these independent reads
// through execution. Each pulse uses facts for its own source, target and current owner stamp.
#[allow(clippy::too_many_arguments)]
pub(super) async fn execute(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    proposals: &[ProfileAbilityProposal],
    parties: &ReferenceParties,
    now: u64,
) {
    for proposal in proposals {
        if proposal.list != crate::ai_think::profile_schedule::ScheduleList::Attack {
            continue;
        }
        let Some(parent_source) = source_for(runtime, native, proposal) else {
            if let Some(source) = composed::source_for(runtime, native, proposal) {
                composed::execute(
                    runtime, states, attack, room, native, map, &source, proposal, parties, now,
                )
                .await;
            }
            continue;
        };
        let source = match parent_source.selected_variant(proposal) {
            Ok(Some((_, child))) => child,
            Ok(None) => &parent_source,
            Err(_) => continue,
        };
        let Ok(stamp) = runtime.issue_owner_work() else {
            return;
        };
        let Some(mut reader) = prepare_reader(
            runtime,
            states,
            attack,
            room,
            native,
            map,
            source,
            proposal,
            parties,
            now,
            stamp,
            source.chain.is_some(),
            None,
        )
        .await
        else {
            continue;
        };
        if source.chain.is_some() {
            let Some(mut facts) =
                timers::prepare_chain_map(runtime, map, &reader, proposal, stamp, attack, now)
                    .await
            else {
                continue;
            };
            if states.monster_chains.is_none() {
                states.monster_chains =
                    crate::creature_chain_attack::ChainOwner::new_current(runtime).ok();
            }
            if let Some(chains) = &mut states.monster_chains {
                let _ = chains.schedule_current(
                    runtime,
                    stamp,
                    source,
                    proposal,
                    &mut reader,
                    &mut facts,
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
                );
            }
        } else {
            let mut spells = std::mem::take(&mut states.monster_spells);
            let result = spells.execute_current(
                runtime,
                stamp,
                states,
                &parent_source,
                proposal,
                &mut reader,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
            );
            states.monster_spells = spells;
            if let Ok(outcome) = result {
                record_hits(runtime, attack, &reader.players, outcome, now);
            }
        }
    }
    timers::drain(runtime, states, attack, room, native, map, parties, now).await;
}

fn source_for(
    runtime: &ChannelRuntimeV1,
    native: &NativeGameplayState,
    proposal: &ProfileAbilityProposal,
) -> Option<SpellSource> {
    let creature = runtime.companion_snapshot(proposal.issuer).ok()?;
    let role =
        crate::monster_summon::current_source_creature_role(runtime, proposal.issuer).ok()?;
    if !matches!(
        role,
        crate::monster_summon::CurrentSourceCreatureRole::Ordinary
            | crate::monster_summon::CurrentSourceCreatureRole::IntrinsicMonsterSummon(_)
    ) {
        return None;
    }
    let record = native.creature_profiles().records.iter().find(|record| {
        record.profile.target.key == creature.state.policy.definition_key
            && record.profile.target.revision == creature.state.policy.definition_revision
    })?;
    let definitions = record.attack_definitions.as_ref()?;
    SpellSource::from_native(
        &record.profile.target,
        proposal.entry_index,
        &definitions.records,
        &definitions.profiles,
        native.source_digest(),
    )
    .ok()
}

fn record_hits(
    runtime: &ChannelRuntimeV1,
    attack: &mut ChannelAttackStates,
    players: &[(ExactActorRef, GameSessionId)],
    outcome: crate::creature_damage_spell::SpellOutcome,
    now: u64,
) {
    let semantic_now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now);
    for (target, hit) in outcome.targets {
        if hit.is_ok_and(|hit| hit.damage.is_some_and(|damage| damage.applied > 0))
            && let Some((_, session)) = players.iter().find(|(actor, _)| *actor == target)
        {
            attack.record_hit_taken(runtime, target, *session, semantic_now);
        }
    }
}

enum SourceShape<'a> {
    Spell(&'a SpellSource),
    Composed(&'a crate::creature_damage_spell::composed::ComposedSource),
}
impl SourceShape<'_> {
    fn spell(&self) -> Option<&SpellSource> {
        match self {
            Self::Spell(source) => Some(source),
            Self::Composed(source) => source.primary_source(),
        }
    }
    fn creature_key(&self) -> &str {
        match self {
            Self::Spell(source) => &source.creature_key,
            Self::Composed(source) => source.creature_key(),
        }
    }
    fn element(&self) -> Option<&str> {
        self.spell().and_then(|source| source.element.as_deref())
    }
    fn reference_level_overrides(&self) -> &[crate::content::ProjectV2ReferenceLevelOverride] {
        self.spell()
            .map_or(&[], |source| source.reference_level_overrides.as_slice())
    }
    fn has_area(&self) -> bool {
        match self {
            Self::Spell(source) => source.area.is_some(),
            Self::Composed(source) => source.details().area.is_some(),
        }
    }
    fn allows_empty_players(&self) -> bool {
        match self {
            Self::Spell(source) => source.appearance_cast,
            Self::Composed(_) => true,
        }
    }
    fn cast_center(
        &self,
        from: MovementLocalPosition,
        to: MovementLocalPosition,
        facing: Option<Facing>,
    ) -> Result<MovementLocalPosition, crate::creature_auto_attack::AttackError> {
        match self {
            Self::Spell(source) => source.cast_center(from, to, facing),
            Self::Composed(source) => source.cast_center(from, to, facing),
        }
    }
    fn target_tiles(
        &self,
        from: MovementLocalPosition,
        to: MovementLocalPosition,
        facing: Option<Facing>,
    ) -> Result<Vec<(i32, i32, i16)>, crate::creature_auto_attack::AttackError> {
        match self {
            Self::Spell(source) => source.target_tiles(from, to, facing),
            Self::Composed(source) => source.target_tiles(from, to, facing),
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn prepare_reader<'native>(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &'native NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    source: &SpellSource,
    proposal: &ProfileAbilityProposal,
    parties: &ReferenceParties,
    now: u64,
    stamp: RuntimeWorkStamp,
    chain_candidates: bool,
    chain_origin: Option<MovementLocalPosition>,
) -> Option<CurrentTarget<'native>> {
    prepare_source_reader(
        runtime,
        states,
        attack,
        room,
        native,
        map,
        SourceShape::Spell(source),
        proposal,
        parties,
        now,
        stamp,
        chain_candidates,
        chain_origin,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn prepare_composed_reader<'native>(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &'native NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    source: &crate::creature_damage_spell::composed::ComposedSource,
    proposal: &ProfileAbilityProposal,
    parties: &ReferenceParties,
    now: u64,
    stamp: RuntimeWorkStamp,
) -> Option<CurrentTarget<'native>> {
    prepare_source_reader(
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
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn prepare_source_reader<'native>(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &'native NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    source: SourceShape<'_>,
    proposal: &ProfileAbilityProposal,
    parties: &ReferenceParties,
    now: u64,
    stamp: RuntimeWorkStamp,
    chain_candidates: bool,
    chain_origin: Option<MovementLocalPosition>,
) -> Option<CurrentTarget<'native>> {
    let record = native
        .creature_profiles()
        .records
        .iter()
        .find(|record| record.profile.target.key == source.creature_key())?;
    let Ok(census) = runtime.positioned_melee_census() else {
        return None;
    };
    let (Ok(from), Ok(to)) = (
        runtime.read_actor_position(proposal.issuer),
        runtime.read_actor_position(proposal.target),
    ) else {
        return None;
    };
    let from = from.position();
    let to = to.position();
    let facing = states
        .monster_melee
        .creature_ai
        .state(proposal.issuer)
        .map(|state| match state.attack_facing() {
            crate::movement::CardinalStep::North => Facing::North,
            crate::movement::CardinalStep::East => Facing::East,
            crate::movement::CardinalStep::South => Facing::South,
            crate::movement::CardinalStep::West => Facing::West,
        })?;
    let (Ok(center), Ok(mut cast_tiles)) = (
        source.cast_center(from, to, Some(facing)),
        source.target_tiles(from, to, Some(facing)),
    ) else {
        return None;
    };
    if chain_candidates {
        let chain = source.spell()?.chain.as_ref()?;
        let reach = i64::from(chain.range_tiles) * i64::from(chain.max_targets);
        cast_tiles = census
            .iter()
            .filter_map(|(actor, _, session)| {
                session.as_ref()?;
                let position = runtime.read_actor_position(*actor).ok()?.position();
                (position.floor == to.floor
                    && (i64::from(position.x) - i64::from(to.x))
                        .abs()
                        .max((i64::from(position.y) - i64::from(to.y)).abs())
                        <= reach)
                    .then_some((position.x, position.y, position.floor))
            })
            .collect();
    }
    let area = source.has_area();
    // Crystal combat.cpp:2337-2375 casts area sight from the cell next to
    // the chosen center in the direction of the caster, including diagonals.
    let sight_origin = (if area {
        area_sight_origin(from, center)
    } else {
        Some(chain_origin.unwrap_or(from))
    })?;
    let mut tiles = BTreeMap::new();
    for (x, y, floor) in cast_tiles {
        let at = MovementLocalPosition { x, y, floor };
        // An immutable source blocker is already a refusal. Do not require sight
        // through that blocker (or usable item metadata for a tile never admitted).
        if area
            && room
                .movement_cells()
                .spell_tiles()
                .lookup(
                    room.movement_cells().scope(),
                    crate::content::LogicalCell {
                        x,
                        y,
                        z: i32::from(at.floor),
                    },
                )
                .is_ok_and(|tile| {
                    tile.flags().block_projectile
                        || tile.flags().block_solid
                        || tile.flags().protection_zone
                        || tile.flags().floor_change
                })
        {
            tiles.insert((x, y, at.floor), Some(false));
            continue;
        }
        // Sight excludes its endpoints. An area needs the actual current tile too:
        // ground, PZ, floor changes and Item blockers come from the same transaction.
        let area_tile = if area {
            map.read(runtime, at).await
        } else {
            Some(true)
        };
        map.read_sight(runtime, sight_origin, at).await;
        let allowed = room
            .movement_cells()
            .spell_tiles()
            .lookup(
                room.movement_cells().scope(),
                crate::content::LogicalCell {
                    x,
                    y,
                    z: i32::from(at.floor),
                },
            )
            .is_ok_and(|tile| !tile.flags().protection_zone && !tile.flags().floor_change);
        let sight = crate::durability::native_world_read::source_sight_steps(sight_origin, at)
            .ok()
            .and_then(|steps| {
                steps
                    .iter()
                    .all(|(at, exempt)| *exempt || map.view().known(*at))
                    .then(|| {
                        area_tile.map(|admitted| {
                            admitted && allowed && map.view().sight(sight_origin, at)
                        })
                    })
                    .flatten()
            });
        tiles.insert((x, y, at.floor), sight);
    }
    if !map.validate_transaction(runtime).await {
        return None;
    }
    let mut players = Vec::new();
    let mut facts = Vec::new();
    let semantic_now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now);
    for (target, _, session) in &census {
        let Some(session) = *session else {
            continue;
        };
        if !area && !chain_candidates && *target != proposal.target {
            continue;
        }
        let Ok(position) = runtime.read_actor_position(*target) else {
            continue;
        };
        let to = position.position();
        if !tiles.contains_key(&(to.x, to.y, to.floor)) {
            continue;
        }
        if states.is_dead(*target)
            || states.has_pending_spell_commit(*target, session)
            || runtime.assert_actor_spell_unreserved(*target).is_err()
        {
            continue;
        }
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
        let Some(state) = states.get(runtime, *target, session) else {
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
        let Some(protected) =
            attack.creature_target_protection(runtime, *target, session, semantic_now)
        else {
            continue;
        };
        use crate::content::ReferenceResistanceKind as Kind;
        let kind = match source.element() {
            Some("earth") => Some(Kind::Earth),
            Some("death") => Some(Kind::Death),
            Some("energy") => Some(Kind::Energy),
            Some("ice") => Some(Kind::Ice),
            Some("fire") => Some(Kind::Fire),
            Some("holy") => Some(Kind::Holy),
            Some("physical") => Some(Kind::Physical),
            Some("life_drain") => Some(Kind::LifeDrain),
            Some("mana_drain") => Some(Kind::ManaDrain),
            Some("drowning") => Some(Kind::Drown),
            None => None,
            _ => continue,
        };
        let protection = kind.and_then(|kind| state.equipped_element_protection(kind));
        let Ok(base_speed) = u16::try_from(state.owned_base_speed()) else {
            continue;
        };
        facts.push(SpellCombatFacts {
            attack: AttackFacts {
                issuer: proposal.issuer,
                target: *target,
                session,
                revision: state.revision(),
                visible: !invisible || behavior.targeting.sense_invisible,
                issuer_pz: issuer_tile.flags().protection_zone,
                target_pz: target_tile.flags().protection_zone,
                issuer_protected: false,
                target_protected: protected,
                defense: 0,
                armor: state.equipped_armor(),
                player_level: state.character_facts().level,
                reference_level_party_lowest: source
                    .reference_level_overrides()
                    .iter()
                    .find(|area| {
                        crate::creature_auto_attack::reference_level_area_contains(area, from)
                            && crate::creature_auto_attack::reference_level_area_contains(area, to)
                    })
                    .and_then(|area| {
                        parties
                            .get(&target.placement_identity())
                            .and_then(|members| {
                                members
                                    .iter()
                                    .filter(|(position, _)| {
                                        crate::creature_auto_attack::reference_level_area_contains(
                                            area, *position,
                                        )
                                    })
                                    .map(|(_, level)| *level)
                                    .min()
                            })
                    }),
            },
            multiplier_ppm: 1_000_000,
            equipment_protection: protection,
            immune: false,
            // Player equipment suppressions are already installed in this state's
            // ConditionStore by the equipment owner. Players have no creature-type
            // intrinsic immunities; the existing store applies its suppression policy.
            condition_policy: Some(CurrentConditionPolicy {
                immunities: Vec::new(),
                base_speed,
            }),
        });
        players.push((*target, session));
    }
    // Preserve the spell owner's bound after selecting the actual area, not the Channel roster.
    if (players.is_empty() && !source.allows_empty_players()) || players.len() > 64 {
        return None;
    }
    Some(CurrentTarget {
        native,
        stamp,
        players,
        issuer: proposal.issuer,
        element: source.element().map(str::to_owned),
        facing,
        facts,
        tiles,
    })
}

/// Crystal 201189f combat.cpp:2337-2375 and tools.cpp:642-684: the area's
/// visibility origin is one tile from its center toward the actual caster.
fn area_sight_origin(
    caster: MovementLocalPosition,
    center: MovementLocalPosition,
) -> Option<MovementLocalPosition> {
    if caster.floor != center.floor {
        return None;
    }
    let dx = i64::from(caster.x) - i64::from(center.x);
    let dy = i64::from(caster.y) - i64::from(center.y);
    let (sx, sy) = match dx.abs().cmp(&dy.abs()) {
        std::cmp::Ordering::Greater => (dx.signum() as i32, 0),
        std::cmp::Ordering::Less => (0, dy.signum() as i32),
        std::cmp::Ordering::Equal => (dx.signum() as i32, dy.signum() as i32),
    };
    Some(MovementLocalPosition {
        x: center.x.checked_add(sx)?,
        y: center.y.checked_add(sy)?,
        floor: center.floor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: i32, y: i32) -> MovementLocalPosition {
        MovementLocalPosition { x, y, floor: 7 }
    }

    #[test]
    fn area_sight_uses_the_center_neighbor_from_the_donor() {
        assert_eq!(area_sight_origin(at(0, 0), at(8, 2)), Some(at(7, 2)));
        assert_eq!(area_sight_origin(at(0, 0), at(2, 8)), Some(at(2, 7)));
        assert_eq!(area_sight_origin(at(0, 0), at(8, 8)), Some(at(7, 7)));
        assert_eq!(area_sight_origin(at(8, 8), at(0, 0)), Some(at(1, 1)));
        assert_eq!(area_sight_origin(at(0, 0), at(0, 0)), Some(at(0, 0)));
    }

    #[test]
    fn area_sight_refuses_another_floor_and_handles_coordinate_extremes() {
        let mut other_floor = at(0, 0);
        other_floor.floor = 8;
        assert_eq!(area_sight_origin(other_floor, at(0, 0)), None);
        assert_eq!(
            area_sight_origin(at(i32::MIN, i32::MIN), at(i32::MAX, i32::MAX)),
            Some(at(i32::MAX - 1, i32::MAX - 1)),
        );
    }
}
