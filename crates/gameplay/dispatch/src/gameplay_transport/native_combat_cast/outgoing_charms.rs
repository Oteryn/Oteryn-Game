//! One retained outgoing-charm decision beside the original numerical cast.
use super::super::super::attack::charm_combat;
use super::*;
use crate::foundation::runtime_actor_spell_types::{
    MAX_EFFECTS, OwnerCombatChange, OwnerCombatEffect,
};
use crate::spell::delayed_execution::SpellCharmDamage;
use oteryn_gameplay_entities::carrier::runtime_actor_spell::SpellCharmChild;
use oteryn_gameplay_progression::charm_effects::{
    CharmAttackerFacts, CharmDamageKind, CharmEffectOutcome, CharmElement, CharmHitSource,
    CharmHookEvent,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
use sha2::{Digest, Sha256};

#[derive(Debug)]
pub(super) struct PreparedOutgoingCharms {
    pub(super) batch: OwnerCombatBatch,
    fatal_holds: Vec<(ExactActorRef, u64)>,
    pub(super) damage: Vec<SpellCharmDamage>,
    pub(super) children:
        Option<oteryn_gameplay_entities::carrier::runtime_actor_spell::QualifiedSpellChildren>,
}

pub(super) fn hit_occurrence(
    batch: &OwnerCombatBatch,
    target: ExactActorRef,
    ordinal: u16,
) -> DecisionOccurrenceId {
    let hash = Sha256::new()
        .chain_update(b"oteryn:spell-charm-hit:v1")
        .chain_update(batch.caster.placement_identity())
        .chain_update(batch.command.game_session_id().as_bytes())
        .chain_update(batch.command.command_id().get().to_be_bytes())
        .chain_update(target.placement_identity())
        .chain_update(ordinal.to_be_bytes())
        .finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hash[..16]);
    DecisionOccurrenceId::from_bytes(bytes)
}

impl PreparedNativeCombatCast {
    pub(super) fn prepare_outgoing_charms(
        &mut self,
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        content: &NativeGameplayState,
        catalogue: &crate::content::charm_source::CanonicalCharmCatalogue,
        talisman: &crate::durability::task_board::BountyTalismanSnapshot,
    ) -> Result<(), Error> {
        self.outgoing_charms = PreparedOutgoingCharms::prepare(
            runtime,
            states,
            content,
            catalogue,
            talisman,
            &self.batch,
            &self.before,
            &self.creatures,
            &self.tile_facts,
        )?;
        Ok(())
    }
}

impl PreparedOutgoingCharms {
    #[allow(
        clippy::too_many_arguments,
        reason = "the same direct and due physical owner inputs"
    )]
    fn prepare(
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        content: &NativeGameplayState,
        catalogue: &crate::content::charm_source::CanonicalCharmCatalogue,
        talisman: &crate::durability::task_board::BountyTalismanSnapshot,
        batch: &OwnerCombatBatch,
        before: &PlayerSpellState,
        creatures: &[crate::foundation::CompanionSnapshot],
        tile_facts: &BTreeMap<TilePosition, TileFlags>,
    ) -> Result<Option<Self>, Error> {
        if states
            .charm_state(runtime, batch.caster, batch.command.game_session_id())
            .is_none()
            || !batch.effects.iter().any(|effect| {
                runtime.contains_live_creature(effect.target)
                    && matches!(
                        effect.change,
                        OwnerCombatChange::Damage { magnitude: 1.., .. }
                            | OwnerCombatChange::DamageWithHealing { damage: 1.., .. }
                    )
            })
        {
            return Ok(None);
        }
        let root = GameplayDecisionRoot::from_bytes(content.source_digest());
        let primary = runtime.stage_spell_batch(batch)?;
        let facts = before.character_facts();
        let vitals = before.vitals();
        let mut joined = PreparedOutgoingCharms {
            batch: batch.clone(),
            fatal_holds: Vec::new(),
            damage: Vec::new(),
            children: None,
        };
        let mut children = Vec::new();
        for receipt in primary.preview_effect_receipts() {
            let Some(health) = receipt
                .health
                .filter(|health| health.applied && health.health_before > health.health_after)
            else {
                continue;
            };
            let Ok(creature) = runtime.companion_snapshot(receipt.target) else {
                continue;
            };
            if creature.state.master.is_some() || creature.state.policy.is_familiar {
                continue;
            }
            let occurrence = hit_occurrence(batch, receipt.target, receipt.sub_ordinal);
            let outcomes = charm_combat::evaluate_scoped(
                states,
                runtime,
                catalogue,
                (batch.caster, batch.command.game_session_id()),
                (&creature.state.policy.definition_key, Some(receipt.target)),
                (&root, occurrence),
                CharmHookEvent::committed_hit(
                    &health,
                    CharmHitSource::CharacterAttack,
                    CharmAttackerFacts {
                        level: facts.level,
                        max_health: u64::from(vitals.max_health),
                        max_mana: u64::from(vitals.max_mana),
                    },
                    u64::try_from(creature.maximum_health).map_err(|_| Error::InvalidMagnitude)?,
                ),
            );
            for outcome in &outcomes {
                match charm_combat::effect(outcome) {
                    Some(CharmEffectOutcome::ProcDamage { damage, amount })
                        if health.health_after > 0 =>
                    {
                        joined.append_damage(
                            runtime,
                            &mut children,
                            receipt.target,
                            (outcome.charm_key.as_str(), damage, amount),
                            &root,
                            occurrence,
                            talisman,
                        )?;
                    }
                    Some(CharmEffectOutcome::ParalyseCreature {
                        duration_ms: 10_000,
                    }) if health.health_after > 0 => {
                        if children.len() >= MAX_EFFECTS {
                            return Err(Error::TooManyEffects);
                        }
                        children.push(SpellCharmChild::Paralysis {
                            target: receipt.target,
                            root: content.source_digest(),
                            occurrence,
                        });
                    }
                    Some(CharmEffectOutcome::PreventCreatureFlee { duration_ms })
                        if health.health_after > 0 =>
                    {
                        let until = batch
                            .now_us
                            .ok_or(Error::InvalidBatch)?
                            .get()
                            .checked_add(u64::from(duration_ms) * 1000)
                            .ok_or(Error::InvalidCondition)?;
                        let ai = &mut states.monster_melee.creature_ai;
                        if ai.state(receipt.target).is_none() {
                            let definition = crate::content::ProjectV2DefinitionRef {
                                family: crate::content::ProjectV2Family::Creature,
                                key: creature.state.policy.definition_key.clone(),
                                revision: creature.state.policy.definition_revision.clone(),
                            };
                            ai.admit_native_creature(
                                runtime,
                                content,
                                receipt.target,
                                &definition,
                                batch.now_us.ok_or(Error::InvalidBatch)?.get(),
                            )
                            .map_err(|_| Error::InvalidCondition)?;
                        }
                        joined.fatal_holds.push((receipt.target, until));
                    }
                    Some(CharmEffectOutcome::AreaDamageAroundKill { damage, amount })
                        if health.health_after == 0 =>
                    {
                        // Only primary-source deaths enter here. Generated leaves never
                        // evaluate another hook, including Carnage on an elemental kill.
                        for candidate in creatures {
                            let at = candidate.position;
                            let origin = creature.position;
                            let tile = TilePosition {
                                x: at.x,
                                y: at.y,
                                floor: at.floor,
                            };
                            let legal = tile_facts.get(&tile).is_some_and(|flags| {
                                flags.present
                                    && !flags.floor_change
                                    && !flags.solid
                                    && !flags.protection
                            });
                            if candidate.actor != receipt.target
                                && candidate.health > 0
                                && candidate.state.master.is_none()
                                && !candidate.state.policy.is_familiar
                                && crate::monster_summon::current_source_creature_role(
                                    runtime,
                                    candidate.actor,
                                )
                                .is_ok_and(|role| role.is_ordinary())
                                && candidate.state.policy.flags.attackable
                                && legal
                                && runtime.is_tibiadrome(candidate.actor)
                                    == runtime.is_tibiadrome(batch.caster)
                                && at.floor == origin.floor
                                && ((at.x == origin.x && at.y.abs_diff(origin.y) == 1)
                                    || (at.y == origin.y && at.x.abs_diff(origin.x) == 1))
                            {
                                let child_occurrence =
                                    hit_occurrence(batch, candidate.actor, receipt.sub_ordinal);
                                joined.append_damage(
                                    runtime,
                                    &mut children,
                                    candidate.actor,
                                    (outcome.charm_key.as_str(), damage, amount),
                                    &root,
                                    child_occurrence,
                                    talisman,
                                )?;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        joined.set_children(runtime, children)?;
        Ok(Some(joined))
    }
}

impl PreparedOutgoingCharms {
    fn set_children(
        &mut self,
        runtime: &ChannelRuntimeV1,
        children: Vec<SpellCharmChild>,
    ) -> Result<(), Error> {
        let mut binding: Value =
            serde_json::from_slice(&self.batch.binding).map_err(|_| Error::InvalidBatch)?;
        binding
            .as_object_mut()
            .ok_or(Error::InvalidBatch)?
            .remove("source_charm_children");
        self.batch.binding = serde_json::to_vec(&binding).map_err(|_| Error::InvalidBatch)?;
        self.children = if children.is_empty() {
            None
        } else {
            Some(oteryn_gameplay_entities::carrier::runtime_actor_spell::QualifiedSpellChildren::bind(runtime, &mut self.batch, children)?)
        };
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn append_damage(
        &mut self,
        runtime: &ChannelRuntimeV1,
        children: &mut Vec<SpellCharmChild>,
        target: ExactActorRef,
        proc: (&str, CharmDamageKind, u64),
        root: &GameplayDecisionRoot,
        occurrence: DecisionOccurrenceId,
        talisman: &crate::durability::task_board::BountyTalismanSnapshot,
    ) -> Result<(), Error> {
        let (key, damage, amount) = proc;
        // Ask the actual staged physical owner, including every prior child and
        // native HP callback. There is no parallel HP simulation or mutable runtime.
        self.set_children(runtime, children.clone())?;
        let preview = match &self.children {
            Some(proof) => runtime.stage_spell_batch_with_children(&self.batch, proof)?,
            None => runtime.stage_spell_batch(&self.batch)?,
        };
        if preview
            .preview_effect_receipts()
            .iter()
            .rev()
            .find(|effect| effect.target == target && effect.health.is_some())
            .and_then(|effect| effect.health)
            .is_some_and(|health| health.health_after == 0)
        {
            return Ok(());
        }
        let snapshot = runtime.companion_snapshot(target)?;
        let mut amount = u32::try_from(amount).map_err(|_| Error::InvalidMagnitude)?;
        if damage.reduced_by_armor {
            let draw = oteryn_simulation_determinism::deterministic_decision_u64(
                root,
                occurrence,
                "charm.spell-armor",
                0,
            )
            .map_err(|_| Error::InvalidMagnitude)?;
            amount = oteryn_gameplay_combat_rules::resolution::player_armor_mitigated(
                crate::combat_attack::AttackConstants::checked_in()
                    .ok_or(Error::InvalidMagnitude)?,
                amount,
                snapshot.state.policy.armor.ok_or(Error::InvalidMagnitude)?,
                draw,
            );
        }
        if !damage.ignores_resistances {
            let element = match damage.element {
                CharmElement::Physical => "physical",
                CharmElement::Fire => "fire",
                CharmElement::Earth => "earth",
                CharmElement::Energy => "energy",
                CharmElement::Ice => "ice",
                CharmElement::Holy => "holy",
                CharmElement::Death => "death",
            };
            amount = u32::try_from(
                oteryn_gameplay_combat_rules::resolution::creature_damage_response(
                    &snapshot.state.policy,
                    i64::from(amount),
                    element,
                )
                .ok_or(Error::InvalidMagnitude)?,
            )
            .map_err(|_| Error::InvalidMagnitude)?;
        }
        if talisman.elite(&snapshot.state.policy.definition_key) {
            amount /= 2;
        }
        if amount == 0 {
            return Ok(());
        }
        if children.len() >= MAX_EFFECTS {
            return Err(Error::TooManyEffects);
        }
        let sub_ordinal =
            u16::try_from(MAX_EFFECTS + children.len()).map_err(|_| Error::TooManyEffects)?;
        children.push(SpellCharmChild::Damage(OwnerCombatEffect {
            target,
            sub_ordinal,
            change: OwnerCombatChange::Damage {
                target_atom: runtime.creature_spell_target_atom(target)?,
                magnitude: i64::from(amount),
            },
        }));
        self.damage.push(SpellCharmDamage {
            target,
            sub_ordinal,
            element: damage.element,
            key: key.to_owned(),
        });
        Ok(())
    }

    pub(super) fn validate_fatal_holds(&self, states: &ChannelSpellStates) -> Result<(), Error> {
        if self
            .fatal_holds
            .iter()
            .any(|(actor, _)| states.monster_melee.creature_ai.state(*actor).is_none())
        {
            return Err(Error::SnapshotChanged);
        }
        Ok(())
    }

    pub(super) fn install_fatal_holds(&self, states: &mut ChannelSpellStates) {
        for (actor, until) in &self.fatal_holds {
            states
                .monster_melee
                .creature_ai
                .apply_fatal_hold(*actor, *until);
        }
    }
}

/// SQL-qualified cardinal candidate tiles, retained alongside the first due numbers.
#[allow(
    clippy::too_many_arguments,
    reason = "the existing fenced tile reader's inputs"
)]
pub(in crate::gameplay_transport) async fn due_candidates(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    batch: &OwnerCombatBatch,
) -> Result<Vec<(crate::foundation::CompanionSnapshot, bool)>, SpellCastDisposition> {
    let mut candidates = Vec::new();
    for (actor, _, session) in runtime
        .positioned_actor_census()
        .map_err(|_| SpellCastDisposition::Rejected)?
    {
        if session.is_some() {
            continue;
        }
        let snapshot = runtime
            .companion_snapshot(actor)
            .map_err(|_| SpellCastDisposition::Rejected)?;
        let at = snapshot.position;
        let cardinal = batch.effects.iter().any(|effect| {
            runtime
                .companion_snapshot(effect.target)
                .is_ok_and(|origin| {
                    let p = origin.position;
                    at.floor == p.floor
                        && ((at.x == p.x && at.y.abs_diff(p.y) == 1)
                            || (at.y == p.y && at.x.abs_diff(p.x) == 1))
                })
        });
        let legal = if cardinal {
            let flags = read_tile(
                tx,
                authority,
                room,
                runtime,
                objects,
                TilePosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                },
            )
            .await?;
            flags.present && !flags.floor_change && !flags.solid && !flags.protection
        } else {
            false
        };
        candidates.push((snapshot, legal));
    }
    Ok(candidates)
}

/// Called only after the real timer has sealed this exact due batch.
pub(in crate::gameplay_transport) fn prepare_due_children(
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    content: &NativeGameplayState,
    catalogue: &crate::content::charm_source::CanonicalCharmCatalogue,
    batch: &OwnerCombatBatch,
    inputs: &crate::spell::delayed_execution::DueSpellAugments,
) -> Result<crate::spell::delayed_execution::DueSpellChildren, Error> {
    if !runtime.qualified_deferred_batch(batch) {
        return Err(Error::InvalidBatch);
    }
    let before = states
        .get(runtime, batch.caster, batch.command.game_session_id())
        .ok_or(Error::SnapshotChanged)?
        .clone();
    let mut creatures = Vec::new();
    let mut tiles = BTreeMap::new();
    for (snapshot, legal) in &inputs.candidates {
        if runtime.companion_snapshot(snapshot.actor)? != *snapshot {
            return Err(Error::SnapshotChanged);
        }
        creatures.push(snapshot.clone());
        let at = snapshot.position;
        tiles.insert(
            TilePosition {
                x: at.x,
                y: at.y,
                floor: at.floor,
            },
            TileFlags {
                present: *legal,
                protection: false,
                solid: false,
                floor_change: false,
                projectile: false,
            },
        );
    }
    Ok(PreparedOutgoingCharms::prepare(
        runtime,
        states,
        content,
        catalogue,
        &inputs.talisman,
        batch,
        &before,
        &creatures,
        &tiles,
    )?
    .map_or_else(Default::default, |prepared| {
        crate::spell::delayed_execution::DueSpellChildren {
            proof: prepared.children,
            fatal_holds: prepared.fatal_holds,
            damage: prepared.damage,
        }
    }))
}
