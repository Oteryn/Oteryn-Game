//! Native scheduled player-targeted attacks from every admitted creature's source closure.
//! Timing, chance, damage, replay and lethal consequences stay in their existing owners.
mod smelly_reader;
use super::*;
use crate::creature_auto_attack::{AttackFacts, CurrentConditionPolicy, ProfileAbilityProposal};
use crate::creature_damage_spell::{SpellCombatFacts, SpellSource, SpellWorldReader};
use crate::foundation::{ExactActorRef, GameSessionId, MovementLocalPosition, RuntimeWorkStamp};
use oteryn_gameplay_content::content::creature_attack_geometry::Facing;
pub(super) use smelly_reader::prepare_smelly_reader;
use std::collections::BTreeMap;

pub(super) mod composed;
mod timers;

enum SourceDraw {
    Callback(
        oteryn_gameplay_content::content::native_monster_callbacks::QualifiedNativeCallbackStage,
    ),
    Omrafir(crate::content::native_omrafir::QualifiedOmrafirBeamStage),
}
pub(in crate::gameplay_transport) struct CurrentTarget<'native> {
    native: &'native NativeGameplayState,
    source_draw: Option<SourceDraw>,
    retained_caster: Option<oteryn_gameplay_entities::NativeRetainedCallbackCaster>,
    stamp: RuntimeWorkStamp,
    pub(super) players: Vec<(ExactActorRef, GameSessionId)>,
    issuer: ExactActorRef,
    element: Option<String>,
    facing: Facing,
    facts: Vec<SpellCombatFacts>,
    tiles: BTreeMap<(i32, i32, i16), Option<bool>>,
    source_damage: Vec<(
        ExactActorRef,
        crate::gameplay_transport::soul_war_callbacks::QualifiedThirdTaintPolicy,
    )>,
}

impl CurrentTarget<'_> {
    pub(in crate::gameplay_transport) fn event_players(&self) -> &[(ExactActorRef, GameSessionId)] {
        &self.players
    }

    pub(super) fn source_visual_positions(&self) -> Vec<MovementLocalPosition> {
        self.tiles
            .iter()
            .filter_map(|(&(x, y, floor), allowed)| {
                (*allowed == Some(true)).then_some(MovementLocalPosition { x, y, floor })
            })
            .collect()
    }
}
impl SpellWorldReader for CurrentTarget<'_> {
    fn source_damage_draw(
        &mut self,
        minimum: u32,
        maximum: u32,
        draw: u64,
        target: Option<ExactActorRef>,
    ) -> Result<Option<u32>, crate::creature_auto_attack::AttackError> {
        let Some(stage) = &self.source_draw else {
            return Ok(None);
        };
        let (source_minimum, source_maximum, per_target) = match stage {
            SourceDraw::Callback(stage) => (
                stage.minimum(),
                stage.maximum(),
                matches!(
                    stage.parent().entry().creature.key.as_str(),
                    "oteryn:creature.gaz_haragoth"
                        | "oteryn:creature.lady_tenebris"
                        | "oteryn:creature.eruption_of_destruction"
                ),
            ),
            SourceDraw::Omrafir(stage) => (stage.minimum(), stage.maximum(), false),
        };
        if minimum != source_minimum || maximum != source_maximum {
            return Err(crate::creature_auto_attack::AttackError::InvalidSource);
        }
        if target.is_some() && !per_target {
            return Ok(None);
        }
        oteryn_gameplay_domain::domain::source_normal_draw::source_normal_draw(
            draw, minimum, maximum,
        )
        .map(Some)
        .ok_or(crate::creature_auto_attack::AttackError::InvalidSource)
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
        if stamp != self.stamp || issuer != self.issuer {
            return None;
        }
        let policy = &self
            .source_damage
            .iter()
            .find(|(actor, _)| *actor == target)?
            .1;
        let result = match (&self.retained_caster, &self.source_draw) {
            (Some(caster), Some(SourceDraw::Callback(stage))) => {
                policy.apply_retained(runtime, issuer, target, now, magnitude, caster, stage)
            }
            _ => policy.apply(runtime, issuer, target, now, magnitude),
        };
        #[cfg(test)]
        if result.is_none()
            && matches!(&self.source_draw,Some(SourceDraw::Callback(stage)) if stage.parent().entry().creature.key == "oteryn:creature.foam_stalker")
        {
            eprintln!(
                "SRX3_RETAINED_FOAM_COMPONENT refused exact_live_lookup={} registered={:?} raw={magnitude}",
                runtime.borrow_exact_actor_lookup().contains(issuer),
                runtime.source_creature_is_registered(issuer)
            );
        }
        result
    }
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
pub(in crate::gameplay_transport) type ReferenceParties =
    BTreeMap<[u8; 16], Vec<(MovementLocalPosition, u32)>>;
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
// through execution. Each pulse uses facts for its own source, target and current owner stamp.
#[allow(clippy::too_many_arguments)]
pub(super) async fn execute_proposals(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    proposals: &[ProfileAbilityProposal],
    parties: &ReferenceParties,
    now: u64,
    charms: &crate::content::charm_source::CanonicalCharmCatalogue,
    native_health_names: &crate::gameplay_transport::source_names::NativeHealthNamesData,
) {
    for proposal in proposals {
        if proposal.list != crate::ai_think::profile_schedule::ScheduleList::Attack {
            continue;
        }
        let Some(parent_source) = source_for(runtime, native, proposal) else {
            if let Some(source) = composed::source_for(runtime, native, proposal) {
                composed::execute(
                    runtime,
                    states,
                    attack,
                    room,
                    native,
                    map,
                    &source,
                    proposal,
                    parties,
                    now,
                    charms,
                    native_health_names,
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
        let phosphorus_curse = source.ability.key == "oteryn:ability.spell.phosphorus_curse";
        // Keep the event's exact source qualification even though the current cycle
        // also supports ordinary conditions and composed monster attacks.
        if phosphorus_curse
            && !native.creature_profiles().records.iter().any(|record| {
                oteryn_gameplay_encounters::bosses::summer_2026::is_phosphorus_curse(record, source)
            })
        {
            continue;
        }
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
            let execute = if phosphorus_curse {
                crate::creature_damage_spell::DamageSpellOwner::execute_current_target_condition
            } else {
                crate::creature_damage_spell::DamageSpellOwner::execute_current
            };
            let result = execute(
                &mut spells,
                runtime,
                stamp,
                &mut super::super::attack::charm_combat::source_vitals::CharmSourceVitals {
                    states,
                    attack,
                    catalogue: charms,
                    root: GameplayDecisionRoot::from_bytes(native.source_digest()),
                },
                &parent_source,
                proposal,
                &mut reader,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
            );
            let activity_time =
                spells.retained_activity_time(proposal.issuer, proposal.list, proposal.entry_index);
            states.monster_spells = spells;
            if let Ok(outcome) = result {
                if let Some(activity_time) = activity_time
                    && let Ok(snapshot) = runtime.companion_snapshot(proposal.issuer)
                {
                    for (target, hit) in &outcome.targets {
                        if let Ok(hit) = hit
                            && let Some((_, session)) =
                                reader.players.iter().find(|(actor, _)| actor == target)
                        {
                            states.prey_activity(runtime, *target, *session, super::super::prey::PreyActivity {
                                creature: &snapshot.state.policy.definition_key,
                                kind: if hit.damage.is_some_and(|damage| damage.applied > 0) {
                                    oteryn_gameplay_progression::prey::difficulty::PreyActivityKind::ReceivedDamage
                                } else {
                                    oteryn_gameplay_progression::prey::difficulty::PreyActivityKind::Attacked
                                },
                                now: activity_time.get(),
                            });
                        }
                    }
                }
                record_hits(
                    runtime,
                    attack,
                    Some(proposal.issuer),
                    &reader.players,
                    outcome,
                    now,
                );
            }
        }
    }
}

pub(in crate::gameplay_transport) fn record_native_player_hit(
    runtime: &ChannelRuntimeV1,
    attack: &mut ChannelAttackStates,
    source: Option<ExactActorRef>,
    target: ExactActorRef,
    session: GameSessionId,
    hit: &crate::player_lethal::PlayerDamageReceipt,
    now: u64,
) {
    if hit.applied == 0 {
        return;
    }
    if let Some(source) = source {
        attack.kills().boss_damage_received(
            runtime,
            source,
            target,
            session,
            u64::from(hit.applied),
            hit.vitals_revision,
        );
    }
    attack.record_hit_taken(
        runtime,
        target,
        session,
        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now),
    );
}

fn native_source_profiles<'a>(
    native: &NativeGameplayState,
    record: &'a crate::content::native_gameplay::CreatureProfileRecord,
    source_overlay: bool,
) -> Option<std::borrow::Cow<'a, [crate::content::ProjectV2AuthoringProfile]>> {
    let profiles = &record.attack_definitions.as_ref()?.profiles;
    let soul_war = source_overlay
        .then(|| native.soul_war_source_attacks(&record.profile.target))
        .flatten();
    let omrafir = native.omrafir_source_attacks(&record.profile.target);
    let Some(behavior) = soul_war
        .as_ref()
        .map(|source| source.behavior())
        .or_else(|| omrafir.as_ref().map(|source| source.behavior()))
    else {
        return Some(std::borrow::Cow::Borrowed(profiles));
    };
    // Project only the sealed complete source schedule. Original Creature,
    // Ability, Effect, Formula and record bodies remain byte-for-byte native.
    // Their actual restored entry index therefore reaches their existing
    // constructor, RNG and replay owner without rewriting the proposal.
    let reference = &record.behavior.as_ref()?.target;
    let mut positions = profiles
        .iter()
        .enumerate()
        .filter(|(_, profile)| profile.target == *reference);
    let (index, profile) = positions.next()?;
    if positions.next().is_some()
        || !matches!(
            profile.data,
            crate::content::ProjectV2AuthoringProfileData::Behavior(_)
        )
    {
        return None;
    }
    let mut projected = profiles.clone();
    projected[index].data =
        crate::content::ProjectV2AuthoringProfileData::Behavior(behavior.clone());
    Some(std::borrow::Cow::Owned(projected))
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
    let profiles = native_source_profiles(
        native,
        record,
        matches!(
            runtime.binding().runtime_scope(),
            crate::foundation::RuntimeScopeRefV1::Instance { .. }
        ),
    )?;
    SpellSource::from_native(
        &record.profile.target,
        proposal.entry_index,
        &definitions.records,
        &profiles,
        native.source_digest(),
    )
    .ok()
}

pub(super) fn record_hits(
    runtime: &ChannelRuntimeV1,
    attack: &mut ChannelAttackStates,
    source: Option<ExactActorRef>,
    players: &[(ExactActorRef, GameSessionId)],
    outcome: crate::creature_damage_spell::SpellOutcome,
    now: u64,
) {
    for (target, hit) in outcome.targets {
        if let Ok(hit) = hit
            && let Some(damage) = hit.damage.filter(|damage| damage.applied > 0)
            && let Some((_, session)) = players.iter().find(|(actor, _)| *actor == target)
        {
            record_native_player_hit(runtime, attack, source, target, *session, &damage, now);
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
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn prepare_composed_reader<'native>(
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
    callback: Option<&oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast::CallbackStagePlan>,
) -> Option<CurrentTarget<'native>> {
    let retained = callback.is_some_and(|plan| {
        matches!((plan.retained_caster(),plan.native_stage()),(Some(caster),Some(stage))
            if caster.matches_stage(runtime,proposal.issuer,stage))
    });
    if !runtime.matches_live_creature_identity(proposal.issuer, source.creature_key().as_bytes())
        && !retained
    {
        return None;
    }
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
    let anchor = match callback {
        Some(plan) => plan.anchor_position(runtime).ok()?,
        None => None,
    };
    let physical_facing = from.facing();
    let from = anchor.unwrap_or(from.position());
    let to = anchor.unwrap_or(to.position());
    let scheduled_facing = || {
        states
            .monster_melee
            .creature_ai
            .state(proposal.issuer)
            .map(|state| match state.attack_facing() {
                crate::movement::CardinalStep::North => Facing::North,
                crate::movement::CardinalStep::East
                | crate::movement::CardinalStep::NorthEast
                | crate::movement::CardinalStep::SouthEast => Facing::East,
                crate::movement::CardinalStep::South => Facing::South,
                crate::movement::CardinalStep::West
                | crate::movement::CardinalStep::NorthWest
                | crate::movement::CardinalStep::SouthWest => Facing::West,
            })
    };
    let canonical_facing = physical_facing.map(|facing| match facing {
        crate::foundation::MovementFacing::North => Facing::North,
        crate::foundation::MovementFacing::East => Facing::East,
        crate::foundation::MovementFacing::South => Facing::South,
        crate::foundation::MovementFacing::West => Facing::West,
    });
    let facing = if retained {
        // The actual AI scheduler may have forgotten the dead actor. Its
        // registered canonical pose remains authoritative; create no new think.
        canonical_facing.or_else(scheduled_facing)?
    } else {
        let scheduled = scheduled_facing()?;
        canonical_facing.unwrap_or(scheduled)
    };
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
    let mut source_damage = Vec::new();
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
        let Some(state) = states.get(runtime, *target, session) else {
            continue;
        };
        if callback
            .is_some_and(|plan| !plan.permits_player_vocation(state.character_facts().vocation))
        {
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
        let party_lowest = source
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
            });
        let Some((current, policy)) = current_player_combat_facts(
            runtime,
            states,
            attack,
            room,
            proposal.issuer,
            *target,
            session,
            now,
            source.element(),
            !invisible || behavior.targeting.sense_invisible,
            party_lowest,
        ) else {
            continue;
        };
        facts.push(current);
        source_damage.push((*target, policy));
        players.push((*target, session));
    }
    // Preserve the spell owner's bound after selecting the actual area, not the Channel roster.
    if (players.is_empty() && callback.is_none() && !source.allows_empty_players())
        || players.len() > 64
    {
        return None;
    }
    Some(CurrentTarget {
        native,
        retained_caster: callback.and_then(|plan| plan.retained_caster()).cloned(),
        source_draw: callback
            .and_then(|plan| plan.native_stage())
            .cloned()
            .map(SourceDraw::Callback)
            .or_else(|| match &source {
                SourceShape::Composed(source) => source
                    .omrafir_beam_stage()
                    .cloned()
                    .map(SourceDraw::Omrafir),
                _ => None,
            }),
        stamp,
        players,
        issuer: proposal.issuer,
        element: source.element().map(str::to_owned),
        facing,
        facts,
        tiles,
        source_damage,
    })
}

/// Shared current facts for a normal Creature spell or an explicit native event.
#[allow(
    clippy::too_many_arguments,
    reason = "actual source/target, current owners, selection and reference-party facts remain independent"
)]
fn current_player_combat_facts(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    issuer: ExactActorRef,
    target: ExactActorRef,
    session: GameSessionId,
    now: u64,
    element: Option<&str>,
    visible: bool,
    party_lowest: Option<u32>,
) -> Option<(
    SpellCombatFacts,
    crate::gameplay_transport::soul_war_callbacks::QualifiedThirdTaintPolicy,
)> {
    if states.get(runtime, target, session)?.vitals().health == 0 {
        return None;
    }
    current_registered_player_combat_facts(
        runtime,
        states,
        attack,
        room,
        issuer,
        target,
        session,
        now,
        element,
        visible,
        party_lowest,
    )
}

/// Direct source Combat also reaches a registered HP0 target before its removal.
/// This reads current identity and protection; it does not authorize a new hit.
#[allow(
    clippy::too_many_arguments,
    reason = "actual source/target, current owners, selection and reference-party facts remain independent"
)]
fn current_registered_player_combat_facts(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    issuer: ExactActorRef,
    target: ExactActorRef,
    session: GameSessionId,
    now: u64,
    element: Option<&str>,
    visible: bool,
    party_lowest: Option<u32>,
) -> Option<(
    SpellCombatFacts,
    crate::gameplay_transport::soul_war_callbacks::QualifiedThirdTaintPolicy,
)> {
    let from = runtime.read_actor_position(issuer).ok()?.position();
    let to = runtime.read_actor_position(target).ok()?.position();
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
    let issuer_tile = tile(from).ok()?;
    let target_tile = tile(to).ok()?;
    let state = states.get(runtime, target, session)?;
    let protected = attack.creature_target_protection(
        runtime,
        target,
        session,
        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now),
    )?;
    use crate::content::ReferenceResistanceKind as Kind;
    let kind = match element {
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
        // Undefined has no source-specific or absorbpercentall protection.
        Some("undefined") | None => None,
        _ => return None,
    };
    let facts = SpellCombatFacts {
        attack: AttackFacts {
            issuer,
            target,
            session,
            revision: state.revision(),
            visible,
            issuer_pz: issuer_tile.flags().protection_zone,
            target_pz: target_tile.flags().protection_zone,
            issuer_protected: false,
            target_protected: protected,
            defense: 0,
            armor: state.equipped_armor(),
            player_level: state.character_facts().level,
            reference_level_party_lowest: party_lowest,
        },
        multiplier_ppm: 1_000_000,
        equipment_protection: kind.and_then(|kind| state.equipped_element_protection(kind)),
        immune: false,
        condition_policy: Some(CurrentConditionPolicy {
            immunities: Vec::new(),
            base_speed: u16::try_from(state.owned_base_speed()).ok()?,
        }),
    };
    Some((
        facts,
        crate::gameplay_transport::soul_war_callbacks::source_third_taint_policy(
            states, runtime, issuer, target, now,
        )?,
    ))
}

/// The actual native event selects explicit spectator IDs, not AI visibility or sight.
#[allow(
    clippy::too_many_arguments,
    reason = "source event facts retain the existing runtime, target protection and physical map owners"
)]
pub(in crate::gameplay_transport) async fn prepare_mushroom_reader<'native>(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &'native NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    source: &crate::creature_damage_spell::native_event::MushroomThinkSource,
    now: u64,
    stamp: RuntimeWorkStamp,
) -> Option<CurrentTarget<'native>> {
    if !source.current(runtime) {
        return None;
    }
    let issuer = source.actor();
    let from = runtime.read_actor_position(issuer).ok()?.position();
    let mut players = Vec::new();
    let mut facts = Vec::new();
    let mut source_damage = Vec::new();
    let mut tiles = BTreeMap::new();
    for (target, _, session) in runtime.positioned_melee_census().ok()? {
        let Some(session) = session else {
            continue;
        };
        let to = runtime.read_actor_position(target).ok()?.position();
        if to.floor != from.floor
            || i64::from(to.x).abs_diff(i64::from(from.x)) > 3
            || i64::from(to.y).abs_diff(i64::from(from.y)) > 3
            || states.is_dead(target)
        {
            continue;
        }
        if states.has_pending_spell_commit(target, session)
            || runtime.assert_actor_spell_unreserved(target).is_err()
        {
            return None;
        }
        map.read(runtime, to).await?;
        let (current, policy) = current_player_combat_facts(
            runtime,
            states,
            attack,
            room,
            issuer,
            target,
            session,
            now,
            Some("life_drain"),
            true,
            None,
        )?;
        tiles.insert((to.x, to.y, to.floor), Some(true));
        players.push((target, session));
        facts.push(current);
        source_damage.push((target, policy));
    }
    if players.len() > 64 || !map.validate_transaction(runtime).await {
        return None;
    }
    Some(CurrentTarget {
        native,
        source_draw: None,
        retained_caster: None,
        stamp,
        players,
        issuer,
        element: Some("life_drain".into()),
        facing: Facing::North,
        facts,
        tiles,
        source_damage,
    })
}

/// Reads the callback's actual AI target. The caller validates this same map
/// transaction after source preparation and before the synchronous HP commit.
#[allow(
    clippy::too_many_arguments,
    reason = "the source callback retains its actual boss, target, room and physical map owner"
)]
pub(in crate::gameplay_transport) async fn prepare_rotten_massive_reader<'native>(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &'native NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    source: &crate::creature_damage_spell::native_event::rotten_massive::RottenMassiveSource,
    target: ExactActorRef,
    session: GameSessionId,
    now: u64,
    stamp: RuntimeWorkStamp,
) -> Option<CurrentTarget<'native>> {
    use crate::player_lethal::PlayerLethalVitals;
    let (health, _, _) = states.source_player_health_including_dead(runtime, target, session)?;
    if !source.current(runtime)
        || (health > 0
            && (states.has_pending_spell_commit(target, session)
                || runtime.assert_actor_spell_unreserved(target).is_err()))
    {
        return None;
    }
    let issuer = source.actor();
    let from = runtime.read_actor_position(issuer).ok()?.position();
    let to = runtime.read_actor_position(target).ok()?.position();
    map.read(runtime, from).await?;
    map.read(runtime, to).await?;
    // Agony uses these actual protection/identity facts, but skips elemental
    // equipment blocking in its source-qualified damage preparation. HP0 still
    // needs the same canCombat facts for the source visual and continuation.
    let fact_element = if source.kind().element() == "agony" {
        "undefined"
    } else {
        "energy"
    };
    let (current, policy) = current_registered_player_combat_facts(
        runtime,
        states,
        attack,
        room,
        issuer,
        target,
        session,
        now,
        Some(fact_element),
        true,
        None,
    )?;
    Some(CurrentTarget {
        native,
        source_draw: None,
        retained_caster: None,
        stamp,
        players: vec![(target, session)],
        issuer,
        element: Some(source.kind().element().into()),
        facing: Facing::North,
        facts: vec![current],
        tiles: BTreeMap::from([((to.x, to.y, to.floor), Some(true))]),
        source_damage: vec![(target, policy)],
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

#[allow(
    clippy::too_many_arguments,
    reason = "same current physical read with an owner-sealed source stage"
)]
pub(super) async fn prepare_callback_reader<'native>(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &'native NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    plan: &oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast::CallbackStagePlan,
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
        SourceShape::Spell(plan.source()),
        plan.proposal(),
        parties,
        now,
        stamp,
        false,
        None,
        Some(plan),
    )
    .await
}

#[allow(
    clippy::too_many_arguments,
    reason = "existing ordinary spell timer owner shares one physical scope read"
)]
pub(super) async fn drain_due(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    parties: &ReferenceParties,
    now: u64,
    charms: &crate::content::charm_source::CanonicalCharmCatalogue,
) {
    timers::drain(
        runtime, states, attack, room, native, map, parties, now, charms,
    )
    .await;
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
    #[test]
    fn srx3_source_overlay_preserves_original_damage_body_at_restored_index()
    -> Result<(), Box<dyn std::error::Error>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), String> {
                let world = crate::content::native_gameplay_test_world_id()
                    .map_err(|error| format!("{error:?}"))?;
                let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../content/spells.manifest.json");
                let native = crate::content::native_gameplay::retained_callback_native_fixture_at(
                    world, &manifest,
                );
                let mut checked = 0;
                for slug in [
                    "goshnar_s_cruelty",
                    "goshnar_s_megalomania_green",
                    "goshnar_s_megalomania_purple",
                    "goshnar_s_megalomania_blue",
                    "omrafir",
                ] {
                    let key = format!("oteryn:creature.{slug}");
                    let record = native
                        .creature_profiles()
                        .records
                        .iter()
                        .find(|record| record.profile.target.key == key)
                        .ok_or("actual source creature")?;
                    let restored_behavior = native
                        .soul_war_source_attacks(&record.profile.target)
                        .map(|source| source.behavior().clone())
                        .or_else(|| {
                            native
                                .omrafir_source_attacks(&record.profile.target)
                                .map(|source| source.behavior().clone())
                        })
                        .ok_or("actual sealed source overlay")?;
                    let definitions = record
                        .attack_definitions
                        .as_ref()
                        .ok_or("actual native closure")?;
                    let crate::content::ProjectV2AuthoringProfileData::Behavior(original) =
                        &record.behavior.as_ref().ok_or("actual behavior")?.data
                    else {
                        return Err("behavior family".into());
                    };
                    let profiles = native_source_profiles(&native, record, true)
                        .ok_or("current source constructor profiles")?;
                    let mut found = false;
                    for (before_index, entry) in original.attacks.iter().enumerate() {
                        let Some(source_index) = restored_behavior
                            .attacks
                            .iter()
                            .position(|restored| restored == entry)
                        else {
                            return Err("source slot disappeared".into());
                        };
                        // Cruelty inserts at the END: its five original slots
                        // remain in place. Green/Purple/Blue have real shifted
                        // originals and exercise the displaced-index regression.
                        if source_index == before_index && slug != "goshnar_s_cruelty" {
                            continue;
                        }
                        if let Ok(before) = SpellSource::from_native(
                            &record.profile.target,
                            before_index,
                            &definitions.records,
                            &definitions.profiles,
                            native.source_digest(),
                        ) {
                            let after = SpellSource::from_native(
                                &record.profile.target,
                                source_index,
                                &definitions.records,
                                &profiles,
                                native.source_digest(),
                            )
                            .map_err(|error| {
                                format!("displaced original {slug} {source_index}: {error:?}")
                            })?;
                            assert_eq!(
                                after.ability, before.ability,
                                "restored source index must retain original executable ability"
                            );
                            assert_eq!(
                                after.entry_index, source_index,
                                "retain actual source index for RNG, occurrence and replay"
                            );
                            assert_eq!(
                                (after.minimum, after.maximum, after.element, after.area),
                                (before.minimum, before.maximum, before.element, before.area)
                            );
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return Err(format!(
                            "no retained original damage spell after restored {slug} slot"
                        ));
                    }
                    checked += 1;
                }
                assert_eq!(checked, 5);
                Ok(())
            })?
            .join()
            .map_err(|_| "source constructor test thread panicked")??;
        Ok(())
    }
}
