use super::*;
use crate::content::*;

const V4: ReferenceArtifactProfile = ReferenceArtifactProfile::TypedItemV4;

fn identity() -> Result<TypedDefinitionRef, ContentError> {
    Ok(TypedDefinitionRef::new(
        DefinitionFamily::Item,
        ProductionKey::new("oteryn:item.schema.boundary-witness")?,
        DefinitionRevisionRef::new("definition-r1")?,
    ))
}

fn source_definition(item: ReferenceItemDefinition) -> Result<ReferenceDefinition, ContentError> {
    Ok(ReferenceDefinition {
        definition: identity()?,
        kind: ReferenceDefinitionKind::Item(item),
        client_projection: ClientProjectionClass::ClientSafe,
    })
}

fn worst_definitions(
    item: ReferenceItemDefinition,
) -> Result<Vec<ReferenceDefinition>, ContentError> {
    let identity_only = ReferenceItemDefinition {
        physical_class: ReferenceItemPhysicalClass::Unknown,
        materializable: false,
        stack_class: ReferenceItemStackClass::Unknown,
        legal_destinations: Vec::new(),
        semantics: Default::default(),
    };
    let mut definitions = (0..REFERENCE_ITEM_REGISTRY_SIZE - 1)
        .map(|ordinal| {
            Ok(ReferenceDefinition {
                definition: TypedDefinitionRef::new(
                    DefinitionFamily::Item,
                    ProductionKey::new(&format!("oteryn:item.schema.boundary-{ordinal:05}"))?,
                    DefinitionRevisionRef::new("definition-r1")?,
                ),
                kind: ReferenceDefinitionKind::Item(identity_only.clone()),
                client_projection: ClientProjectionClass::ClientSafe,
            })
        })
        .collect::<Result<Vec<_>, ContentError>>()?;
    definitions.push(source_definition(item)?);
    Ok(definitions)
}

fn target() -> Result<ReferenceItemTarget, ContentError> {
    ReferenceItemTarget::new("oteryn:item.schema.boundary-witness", "definition-r1")
}

fn hex_bytes(value: &str) -> Result<Vec<u8>, ContentError> {
    fn nibble(value: u8) -> Result<u8, ContentError> {
        match value {
            b'0'..=b'9' => Ok(value - b'0'),
            b'a'..=b'f' => Ok(value - b'a' + 10),
            _ => Err(ContentError::InvalidArtifact("invalid fixture hex")),
        }
    }

    let (pairs, remainder) = value.as_bytes().as_chunks::<2>();
    if !remainder.is_empty() {
        return Err(ContentError::InvalidArtifact("odd fixture hex length"));
    }
    pairs
        .iter()
        .map(|pair| Ok((nibble(pair[0])? << 4) | nibble(pair[1])?))
        .collect()
}

fn oracle_weapon(distance: bool) -> Result<ReferenceItemDefinition, ContentError> {
    use ReferenceItemField::{Known, NotApplicable};
    let elements = (1..=5)
        .map(|wire| {
            Ok(ReferenceElementalAttack {
                element: ReferenceWeaponElement::from_wire(wire)?,
                points: Known(ReferenceSignedPoints(0)),
            })
        })
        .collect::<Result<Vec<_>, ContentError>>()?;
    Ok(ReferenceItemDefinition {
        physical_class: ReferenceItemPhysicalClass::Physical,
        materializable: true,
        stack_class: ReferenceItemStackClass::StackCapable,
        legal_destinations: vec![ReferenceItemDestination::CharacterInventory],
        semantics: ReferenceItemSemantics {
            weapon: Known(ReferenceItemWeapon {
                weapon_type: Known(if distance {
                    ReferenceWeaponType::Distance
                } else {
                    ReferenceWeaponType::Sword
                }),
                attack: Known(ReferenceSignedPoints(if distance { 35 } else { 42 })),
                defense: Known(ReferenceSignedPoints(0)),
                extra_defense: Known(ReferenceSignedPoints(0)),
                range: Known(ReferenceCells(if distance { 7 } else { 1 })),
                hit_chance: Known(ReferenceRationalPercent::new(
                    if distance { 9 } else { 0 },
                    if distance { 10 } else { 1 },
                )?),
                max_hit_chance: Known(ReferenceRationalPercent::new(
                    if distance { 1 } else { 0 },
                    1,
                )?),
                ammunition: if distance {
                    Known(ReferenceAmmoType::Arrow)
                } else {
                    NotApplicable
                },
                elemental: Known(elements),
            }),
            ..Default::default()
        },
    })
}

fn group_keys() -> Result<Vec<ReferenceItemGroupKey>, ContentError> {
    let prefix = "oteryn:equipment-group.";
    (0..2)
        .map(|index| {
            let suffix = char::from(b'a' + index);
            ReferenceItemGroupKey::new(&format!(
                "{prefix}{}{suffix}",
                "a".repeat(MAX_KEY_BYTES - prefix.len() - 1)
            ))
        })
        .collect()
}

fn modifier_parameter(
    kind: ReferenceSkillModifierKind,
) -> Result<ReferenceModifierParameter, ContentError> {
    Ok(match modifier_parameter_shape(kind) {
        1 => ReferenceModifierParameter::Boolean(false),
        2 => ReferenceModifierParameter::RationalPercent(ReferenceRationalPercent::new(0, 1)?),
        3 => ReferenceModifierParameter::Milliseconds(ReferenceMilliseconds(0)),
        4 => ReferenceModifierParameter::Cells(ReferenceCells(0)),
        5 => ReferenceModifierParameter::Element(ReferenceModifierElement::Death),
        6 => ReferenceModifierParameter::SignedPoints(ReferenceSignedPoints(0)),
        _ => unreachable!(),
    })
}

fn worst_item() -> Result<ReferenceItemDefinition, ContentError> {
    use ReferenceItemField::{Known, Unknown};
    let target = target()?;
    let capabilities = std::array::from_fn(|index| Known(index % 2 == 0));
    let elements = (1..=5)
        .map(|wire| {
            Ok(ReferenceElementalAttack {
                element: ReferenceWeaponElement::from_wire(wire)?,
                points: Known(ReferenceSignedPoints(0)),
            })
        })
        .collect::<Result<Vec<_>, ContentError>>()?;
    let resistances = (1..=12)
        .map(|wire| {
            Ok(ReferenceResistance {
                kind: ReferenceResistanceKind::from_wire(wire)?,
                percent: Known(ReferenceRationalPercent::new(0, 1)?),
            })
        })
        .collect::<Result<Vec<_>, ContentError>>()?;
    let modifiers = (1..=37)
        .map(|wire| {
            let kind = ReferenceSkillModifierKind::from_wire(wire)?;
            Ok(ReferenceModifierBinding {
                kind,
                target_domain: Known(wire),
                evaluation_phase: Known(wire),
                priority: Known(0),
                parameter: Known(modifier_parameter(kind)?),
            })
        })
        .collect::<Result<Vec<_>, ContentError>>()?;
    let allowances = (1..=20)
        .map(|wire| {
            Ok(ReferenceImbuementAllowance {
                family: ReferenceImbuementFamily::from_wire(wire)?,
                tier: ReferenceImbuementTier::Three,
            })
        })
        .collect::<Result<Vec<_>, ContentError>>()?;
    let excluded = (1..=20)
        .map(ReferenceImbuementFamily::from_wire)
        .collect::<Result<Vec<_>, ContentError>>()?;
    let patterns = (1..=2)
        .map(|pattern_id| {
            let primary_slot = ReferenceEquipmentSlot::from_wire(pattern_id)?;
            let additional_reserved_slots = (1..=10)
                .map(ReferenceEquipmentSlot::from_wire)
                .collect::<Result<Vec<_>, ContentError>>()?
                .into_iter()
                .filter(|slot| *slot != primary_slot)
                .collect();
            Ok(ReferenceEquipmentPattern {
                pattern_id,
                primary_slot: Known(primary_slot),
                additional_reserved_slots: Known(additional_reserved_slots),
                mutually_exclusive_groups: Known(group_keys()?),
                vocations: Known(
                    (1..=5)
                        .map(ReferenceBaseVocation::from_wire)
                        .collect::<Result<Vec<_>, _>>()?,
                ),
                level: Known(0),
                compatibility_rule: Unknown,
            })
        })
        .collect::<Result<Vec<_>, ContentError>>()?;
    let transforms = (1..=10)
        .map(|wire| {
            Ok(ReferenceTransformTarget {
                kind: ReferenceTransformKind::from_wire(wire)?,
                target: Known(target.clone()),
            })
        })
        .collect::<Result<Vec<_>, ContentError>>()?;
    Ok(ReferenceItemDefinition {
        physical_class: ReferenceItemPhysicalClass::Physical,
        materializable: true,
        stack_class: ReferenceItemStackClass::StackCapable,
        legal_destinations: vec![ReferenceItemDestination::CharacterInventory],
        semantics: ReferenceItemSemantics {
            presentation: Known(ReferenceItemPresentation {
                name: Known("N".repeat(REFERENCE_ITEM_MAX_NAME_BYTES)),
                description: Known("D".repeat(REFERENCE_ITEM_MAX_DESCRIPTION_BYTES)),
            }),
            classification: Known(ReferenceItemClassification {
                item_type: Known(ReferenceItemType::Bed),
                capabilities: Known(capabilities),
            }),
            physical: Known(ReferenceItemPhysical {
                weight: Known(0),
                movable: Known(false),
                pickupable: Known(true),
            }),
            stack: Known(ReferenceItemStack {
                stackable: Known(true),
                stack_max: Known(100),
            }),
            equipment: Known(ReferenceItemEquipment {
                patterns: Known(patterns),
            }),
            weapon: Known(ReferenceItemWeapon {
                weapon_type: Known(ReferenceWeaponType::Ammunition),
                attack: Known(ReferenceSignedPoints(0)),
                defense: Known(ReferenceSignedPoints(0)),
                extra_defense: Known(ReferenceSignedPoints(0)),
                range: Known(ReferenceCells(0)),
                hit_chance: Known(ReferenceRationalPercent::new(0, 1)?),
                max_hit_chance: Known(ReferenceRationalPercent::new(0, 1)?),
                ammunition: Known(ReferenceAmmoType::Arrow),
                elemental: Known(elements),
            }),
            protection: Known(ReferenceItemProtection {
                armor: Known(ReferenceSignedPoints(0)),
                resistances: Known(resistances),
            }),
            skill_modifiers: Known(ReferenceItemSkillModifiers {
                modifiers: Known(modifiers),
            }),
            charges: Known(ReferenceItemCharges { count: Known(0) }),
            temporal: Known(ReferenceItemTemporal {
                consumption_mode: Known(ReferenceTemporalMode::DurableAbsoluteDeadline),
                duration: Known(ReferenceMilliseconds(0)),
                stop_duration: Known(false),
                decay_target: Known(target.clone()),
            }),
            container: Known(ReferenceItemContainer { capacity: Known(0) }),
            imbuement: Known(ReferenceItemImbuement {
                slot_count: Known(3),
                allowed_family_tiers: Known(allowances),
                excluded_families: Known(excluded),
            }),
            use_transform: Known(ReferenceItemUseTransform {
                targets: transforms,
            }),
            trade_restrictions: Known(ReferenceItemTradeRestrictions {
                tradeable: Known(false),
                marketable: Known(false),
                vocations: Known(
                    (1..=5)
                        .map(ReferenceBaseVocation::from_wire)
                        .collect::<Result<Vec<_>, _>>()?,
                ),
                account_binding_policy: Unknown,
                character_binding_policy: Unknown,
            }),
            fluid: Known(ReferenceItemFluid {
                fluid_type: Known(ReferenceFluidType::Beer),
            }),
            readable_writeable: Known(ReferenceItemReadableWriteable {
                readable: Known(true),
                writeable: Known(true),
                distance_read: Known(true),
                max_text_length: Known(0),
                write_once_target: Known(target),
            }),
            use_requirements: Unknown,
            consumption: Unknown,
            bed: Unknown,
        },
    })
}

// Include topic files in this module to preserve test paths and private fixture access.
include!("reference_artifact_tests_v4.rs");
include!("reference_artifact_tests_v5.rs");
include!("reference_artifact_tests_profiles.rs");
include!("reference_artifact_tests_v6.rs");
include!("reference_artifact_tests_beds.rs");
include!("reference_artifact_tests_bed_sets.rs");
