//! Crystal-compatible swimming and trap step hooks on the production paced-movement path.
//!
//! Donor values are from Crystal `data/scripts/movements/swimming.lua` lines 1-3, 20-37 and
//! 46-53, and `data/scripts/movements/trap.lua` lines 1-8, 23-44 and 59-69 at `201189f8`.
use crate::content::ReferenceResistanceKind;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::map::boot::BundleMap;
use crate::map::overlay::TilePos;
use crate::map::view::{EntryFacts, MapFacts, placement_key};
use oteryn_gameplay_domain::domain::appearance::{AppearanceSelection, Colours};
use oteryn_gameplay_domain::domain::condition::{ConditionType, DotElement, StatusKind};
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

pub(in crate::gameplay_transport) const SWIMMING_LOOK_TYPE: u32 = 267;
pub(in crate::gameplay_transport) const SWIMMING_GROUND_IDS: [u16; 12] = [
    629, 630, 631, 632, 633, 634, 4809, 4810, 4811, 4812, 4813, 4814,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gameplay_transport) struct Trap {
    pub closed: u16,
    pub minimum_damage: u32,
    pub maximum_damage: u32,
    pub element: ReferenceResistanceKind,
    pub optional_pvp_player_immune: bool,
}

pub(in crate::gameplay_transport) fn trap(id: u16) -> Option<Trap> {
    Some(match id {
        2145 => Trap {
            closed: 2146,
            minimum_damage: 50,
            maximum_damage: 100,
            element: ReferenceResistanceKind::Physical,
            optional_pvp_player_immune: false,
        },
        2147 => Trap {
            closed: 2148,
            minimum_damage: 50,
            maximum_damage: 100,
            element: ReferenceResistanceKind::Physical,
            optional_pvp_player_immune: false,
        },
        3482 => Trap {
            closed: 3481,
            minimum_damage: 15,
            maximum_damage: 30,
            element: ReferenceResistanceKind::Physical,
            optional_pvp_player_immune: true,
        },
        3944 => Trap {
            closed: 3945,
            minimum_damage: 15,
            maximum_damage: 30,
            element: ReferenceResistanceKind::Earth,
            optional_pvp_player_immune: false,
        },
        44529 => Trap {
            closed: 44530,
            minimum_damage: 50,
            maximum_damage: 100,
            element: ReferenceResistanceKind::Physical,
            optional_pvp_player_immune: false,
        },
        _ => return None,
    })
}

fn tile_position(position: MovementLocalPosition) -> Option<TilePos> {
    Some(TilePos {
        x: position.x.try_into().ok()?,
        y: position.y.try_into().ok()?,
        floor: i8::try_from(position.floor).ok()?.checked_neg()?,
    })
}

struct TileEntries {
    protection_zone: bool,
    entries: Vec<(u64, EntryFacts)>,
}

fn tile_entries(map: &BundleMap, position: MovementLocalPosition) -> Option<TileEntries> {
    let pos = tile_position(position)?;
    let tile = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
    let mut ordinal = 0_u8;
    let mut appearances = Vec::new();
    for (&id, &depth) in tile.ids().iter().zip(tile.depths()) {
        if depth != 0 {
            continue;
        }
        if let Some(entry) = map.facts.base_entry(pos, ordinal, id) {
            let key = placement_key(pos, ordinal)?;
            if !map.facts.base_hidden(key) {
                appearances.push((key, entry));
            }
        }
        ordinal = ordinal.checked_add(1)?;
    }
    Some(TileEntries {
        protection_zone: tile.protection_zone(),
        entries: appearances,
    })
}

fn transform(map: &BundleMap, key: u64, appearance: u16) -> bool {
    let Some(replacement) = map.facts.quest_callback_replacement(key, appearance) else {
        return false;
    };
    map.facts.apply_quest_callback(
        key,
        map.facts.object_revision(key),
        replacement,
        &[],
        &[],
        false,
    )
}

impl crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_> {
    /// Sample the fenced World policy before taking the movement runtime/state guards.
    pub(in crate::gameplay_transport) async fn special_movement_policy(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<bool> {
        self.bundle_map()?;
        use crate::durability::{DurabilityError, spell_item_transaction as item_tx};
        use crate::foundation::{CommandId, CommandRef};
        let pin = {
            let runtime = self.runtime.lock().await;
            runtime.player_control_facts(actor, session).ok()?;
            runtime.content_pin().server_artifact_digest()
        };
        let fence = self.current_quest_fence(session).await.ok()??;
        let command = CommandRef::new(session, CommandId::new(self.owner_now().get().max(1)).ok()?);
        let pass = self.root.try_issue_semantic_pass().ok()?;
        let mut context = self;
        pass.run_with_context(&mut context, move |holder, deadline, owner| {
            Box::pin(async move {
                let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                let authority = item_tx::assert_spell_item_authority_in_transaction(
                    &mut tx,
                    permit,
                    owner.root,
                    owner.character,
                    owner.holder,
                    &fence.item_fence(),
                    command,
                    pin,
                )
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
                let policy =
                    crate::durability::spell_field_policy::read_world_field_policy_in_transaction(
                        &mut tx,
                        owner.root,
                        authority.runtime_scope(),
                        authority.scope_generation(),
                    )
                    .await?
                    .ok_or(DurabilityError::Unavailable)?;
                let optional = policy.bind(&tx).mode()
                    == crate::durability::spell_field_policy::FieldWorldType::NoPvp;
                tx.rollback().await?;
                Ok(optional)
            })
        })
        .await
        .ok()
    }
}

fn swimming_definition() -> Option<ConditionDefinition> {
    ConditionDefinition::new_permanent_outfit("environment.swimming", 1, SWIMMING_LOOK_TYPE)?
        .with_appearance(AppearanceSelection {
            outfit_key: "environment.swimming".to_owned(),
            colours: Colours::default(),
            addons: 0,
            mount_key: None,
        })
}

impl ChannelSpellStates {
    #[allow(
        clippy::too_many_arguments,
        reason = "one movement owner turn binds both positions and actor authority"
    )]
    pub(in crate::gameplay_transport) fn stage_special_movement_step(
        &mut self,
        runtime: &ChannelRuntimeV1,
        map: &BundleMap,
        actor: ExactActorRef,
        session: GameSessionId,
        now: u64,
        origin: Option<MovementLocalPosition>,
        destination: MovementLocalPosition,
        optional_pvp: Option<bool>,
    ) -> bool {
        let from_swimming = origin
            .and_then(|at| tile_entries(map, at))
            .is_some_and(|tile| {
                tile.entries
                    .iter()
                    .any(|(_, entry)| SWIMMING_GROUND_IDS.contains(&entry.appearance_id))
            });
        let Some(TileEntries {
            protection_zone,
            entries: ids,
        }) = tile_entries(map, destination)
        else {
            return false;
        };
        let to_swimming = ids
            .iter()
            .any(|(_, entry)| SWIMMING_GROUND_IDS.contains(&entry.appearance_id));
        let Some(current) = self.get(runtime, actor, session).cloned() else {
            return false;
        };
        let mut next = current.clone();
        if from_swimming {
            next.conditions.remove_type(ConditionType::Outfit);
        }
        if to_swimming {
            // Crystal removes harmful conditions before installing the swimming outfit.
            for condition in [
                ConditionType::DamageOverTime(DotElement::Poison),
                ConditionType::DamageOverTime(DotElement::Fire),
                ConditionType::DamageOverTime(DotElement::Energy),
                ConditionType::DamageOverTime(DotElement::Bleeding),
                ConditionType::DamageOverTime(DotElement::Drown),
                ConditionType::DamageOverTime(DotElement::Freezing),
                ConditionType::DamageOverTime(DotElement::Dazzled),
                ConditionType::DamageOverTime(DotElement::Cursed),
                ConditionType::Paralysis,
                ConditionType::Status(StatusKind::Drunk),
                ConditionType::Outfit,
            ] {
                next.conditions.remove_type(condition);
            }
            if let Some(definition) = swimming_definition() {
                let root = GameplayDecisionRoot::from_bytes(
                    runtime.content_pin().server_artifact_digest(),
                );
                let facts = ApplicationFacts {
                    now,
                    base_speed: 1,
                    mana_shield_capacity: 0,
                    target_reentry_protected: false,
                    source_reentry_protected: false,
                    target_is_player: true,
                    decision_root: &root,
                    occurrence: DecisionOccurrenceId::from_bytes([0x53; 16]),
                };
                let _ = next.conditions.apply(
                    &definition,
                    Some("environment:swimming".to_owned()),
                    ConditionSourceKind::Field,
                    &[],
                    &facts,
                );
            }
        }
        let mut changed = next != current;
        if changed {
            let Some(revision) = current.revision().checked_add(1) else {
                return false;
            };
            next.source_set_batch_revision(revision);
            if !self.commit(runtime, actor, session, next) {
                return false;
            }
        }
        // trap.lua:59-69 reopens these four closed spike types on every step-out.
        if let Some(TileEntries {
            protection_zone: false,
            entries,
        }) = origin.and_then(|at| tile_entries(map, at))
        {
            for (key, entry) in entries {
                if matches!(entry.appearance_id, 2146 | 2148 | 3945 | 44530) {
                    transform(map, key, entry.appearance_id - 1);
                }
            }
        }
        let enabled =
            !protection_zone && ids.iter().filter(|(_, e)| e.appearance_id == 3482).count() <= 1;
        if enabled {
            for (key, entry) in &ids {
                let Some(rule) = trap(entry.appearance_id) else {
                    continue;
                };
                // An unavailable policy read cannot grant permission to harm with a jaw trap.
                if rule.optional_pvp_player_immune && optional_pvp != Some(false) {
                    continue;
                }
                // trap.lua:35 transforms before combat (:44), including a blocked hit.
                if !transform(map, *key, rule.closed) {
                    continue;
                }
                changed = true;
                let occurrence = Sha256::new()
                    .chain_update(actor.placement_identity())
                    .chain_update(session.as_bytes())
                    .chain_update(now.to_be_bytes())
                    .chain_update(key.to_be_bytes())
                    .chain_update(map.facts.object_revision(*key).to_be_bytes())
                    .finalize();
                let mut bytes = [0; 16];
                bytes.copy_from_slice(&occurrence[..16]);
                let root = GameplayDecisionRoot::from_bytes(
                    runtime.content_pin().server_artifact_digest(),
                );
                let Ok(draw) = deterministic_decision_u64(
                    &root,
                    DecisionOccurrenceId::from_bytes(bytes),
                    "movement.trap.damage",
                    0,
                ) else {
                    continue;
                };
                let damage = crate::spell::uniform_draw(
                    draw,
                    i64::from(rule.minimum_damage),
                    i64::from(rule.maximum_damage),
                );
                let damage = self
                    .get(runtime, actor, session)
                    .and_then(|state| state.equipped_element_protection(rule.element))
                    .and_then(|protection| {
                        oteryn_gameplay_items::equipment_effects::apply_protection(
                            damage, protection,
                        )
                    })
                    .unwrap_or(damage);
                let Ok(damage) = u32::try_from(damage) else {
                    continue;
                };
                let _ = self.apply_creature_damage_native(
                    runtime,
                    actor,
                    session,
                    damage,
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
                );
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_gameplay_domain::domain::condition::ConditionValues;

    #[test]
    fn every_crystal_trap_has_transform_damage_element_and_optional_pvp_rule() {
        assert_eq!(
            trap(2145).map(|r| (r.closed, r.minimum_damage, r.maximum_damage)),
            Some((2146, 50, 100))
        );
        assert_eq!(trap(2147).map(|r| r.closed), Some(2148));
        assert!(trap(3482).is_some_and(|r| r.optional_pvp_player_immune));
        assert!(
            trap(3944)
                .is_some_and(|r| r.element == ReferenceResistanceKind::Earth && r.closed == 3945)
        );
        assert_eq!(trap(44529).map(|r| r.closed), Some(44530));
    }

    #[test]
    fn swimming_uses_crystals_permanent_look_267() {
        assert_eq!(
            swimming_definition().and_then(|d| match d.values() {
                ConditionValues::Outfit { look_type, .. } => Some(look_type),
                _ => None,
            }),
            Some(267)
        );
    }
}
