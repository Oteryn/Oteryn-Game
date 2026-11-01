//! Native scheduled physical/holy target and qualified Summer ice/fire/energy circle damage.
//! Timing, chance, damage, replay and lethal consequences stay in their existing owners.
use super::*;
use crate::creature_auto_attack::{AttackFacts, ProfileAbilityProposal};
use crate::creature_damage_spell::{SpellCombatFacts, SpellSource, SpellWorldReader};
use crate::foundation::{ExactActorRef, GameSessionId, MovementLocalPosition, RuntimeWorkStamp};
use oteryn_gameplay_content::content::{
    ProjectV2AbilityArea,
    creature_attack_geometry::{self as geometry, Facing},
};
use std::collections::BTreeMap;

struct CurrentTarget {
    stamp: RuntimeWorkStamp,
    players: Vec<(ExactActorRef, GameSessionId)>,
    issuer: ExactActorRef,
    element: String,
    facts: Vec<SpellCombatFacts>,
    tiles: BTreeMap<(i32, i32, i16), Option<bool>>,
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
        if stamp != self.stamp || issuer != self.issuer || element != Some(self.element.as_str()) {
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
                            || runtime.binding().channel_id().is_some_and(|channel| {
                                member.channel == Some(*channel.as_bytes())
                            })) =>
                {
                    members.push((*position, *level));
                }
                Some(_) => {
                    available = false;
                    break;
                }
                None if member.presence == crate::foundation::PartyPresence::Channel
                    && runtime
                        .binding()
                        .channel_id()
                        .is_some_and(|channel| member.channel == Some(*channel.as_bytes())) =>
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
// through execution. Only qualified target damage and Summer ice/fire/energy circles enter.
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
        let elemental_area = matches!(
            (
                source.element.as_deref(),
                record.profile.target.key.as_str()
            ),
            (Some("ice"), "oteryn:creature.radiant_acolyte")
                | (Some("fire"), "oteryn:creature.radiant_paragon")
                | (Some("energy"), "oteryn:creature.radiant_templar")
        ) && !source.needs_target
            && matches!(
                source.area,
                Some(ProjectV2AbilityArea::Circle { radius_tiles: 4 })
            );
        let target_damage = (matches!(
            source.element.as_deref(),
            Some("physical" | "earth" | "death")
        ) || (source.element.as_deref() == Some("holy")
            && matches!(
                record.profile.target.key.as_str(),
                "oteryn:creature.radiant_paragon" | "oteryn:creature.radiant_templar"
            )))
            && source.area.is_none()
            && source.needs_target;
        if proposal.list != crate::ai_think::profile_schedule::ScheduleList::Attack
            || !(elemental_area || target_damage)
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
        let (Ok(from), Ok(to)) = (
            runtime.read_actor_position(proposal.issuer),
            runtime.read_actor_position(proposal.target),
        ) else {
            continue;
        };
        let from = from.position();
        let to = to.position();
        let offsets = if let Some(area) = &source.area {
            let Ok(offsets) = geometry::offsets(area, Facing::North) else {
                continue;
            };
            offsets
        } else {
            vec![(0, 0)]
        };
        let center = if elemental_area { from } else { to };
        let mut tiles = BTreeMap::new();
        for (dx, dy) in offsets {
            let (Some(x), Some(y)) = (center.x.checked_add(dx), center.y.checked_add(dy)) else {
                continue;
            };
            let at = MovementLocalPosition {
                x,
                y,
                floor: center.floor,
            };
            // An immutable source blocker is already a refusal. Do not require sight
            // through that blocker (or usable item metadata for a tile never admitted).
            if elemental_area
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
            let area_tile = if elemental_area {
                map.read(runtime, at).await
            } else {
                Some(true)
            };
            map.read_sight(runtime, from, at).await;
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
            let sight = crate::durability::native_world_read::source_sight_steps(from, at)
                .ok()
                .and_then(|steps| {
                    steps
                        .iter()
                        .all(|(at, exempt)| *exempt || map.view().known(*at))
                        .then(|| {
                            area_tile
                                .map(|admitted| admitted && allowed && map.view().sight(from, at))
                        })
                        .flatten()
                });
            tiles.insert((x, y, at.floor), sight);
        }
        if !map.validate_transaction(runtime).await {
            return;
        }
        let mut players = Vec::new();
        let mut facts = Vec::new();
        let semantic_now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now);
        for (target, _, session) in &census {
            let Some(session) = *session else {
                continue;
            };
            if !elemental_area && *target != proposal.target {
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
            let kind = match source.element.as_deref() {
                Some("earth") => Kind::Earth,
                Some("death") => Kind::Death,
                Some("energy") => Kind::Energy,
                Some("ice") => Kind::Ice,
                Some("fire") => Kind::Fire,
                Some("holy") => Kind::Holy,
                Some("physical") => Kind::Physical,
                _ => continue,
            };
            let protection = state.equipped_element_protection(kind);
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
                    reference_level_party_lowest: source.reference_level_overrides.iter()
                        .find(|area| crate::creature_auto_attack::reference_level_area_contains(area, from) && crate::creature_auto_attack::reference_level_area_contains(area, to))
                        .and_then(|area| parties.get(&target.placement_identity())
                            .and_then(|members| members.iter()
                                .filter(|(position, _)| crate::creature_auto_attack::reference_level_area_contains(area, *position))
                                .map(|(_, level)| *level).min())),
                },
                multiplier_ppm: 1_000_000,
                equipment_protection: protection,
                immune: false,
                condition_policy: None,
            });
            players.push((*target, session));
        }
        // Preserve the spell owner's bound after selecting the actual area, not the Channel roster.
        if players.is_empty() || players.len() > 64 {
            continue;
        }
        let Ok(stamp) = runtime.issue_owner_work() else {
            return;
        };
        let mut reader = CurrentTarget {
            stamp,
            players,
            issuer: proposal.issuer,
            element: source.element.clone().unwrap_or_default(),
            facts,
            tiles,
        };
        let mut spells = std::mem::take(&mut states.monster_spells);
        let execute = if elemental_area {
            crate::creature_damage_spell::DamageSpellOwner::execute_current_elemental_area
        } else {
            crate::creature_damage_spell::DamageSpellOwner::execute_current_target_damage
        };
        let result = execute(
            &mut spells,
            runtime,
            stamp,
            states,
            &source,
            proposal,
            &mut reader,
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
        );
        states.monster_spells = spells;
        if let Ok(outcome) = result {
            for (target, hit) in outcome.targets {
                if let Ok(hit) = hit
                    && let Some(damage) = hit.damage.filter(|damage| damage.applied > 0)
                    && let Some((_, session)) =
                        reader.players.iter().find(|(actor, _)| *actor == target)
                {
                    attack.kills().boss_damage_received(
                        runtime,
                        proposal.issuer,
                        target,
                        *session,
                        u64::from(damage.applied),
                        damage.vitals_revision,
                    );
                    attack.record_hit_taken(runtime, target, *session, semantic_now);
                }
            }
        }
    }
}
