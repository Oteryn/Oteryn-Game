//! WRITE-1: current Native Content to the bounded rewrite leaf policy.
//! A qualified profile supplies immutable expected facts, never current custody
//! or activation authority. The durability writer independently checks those.

use crate::content::native_gameplay::{
    NativeGameplayState, NativeItemAdmission, NativeItemDestination, NativeItemStackClass,
    NativeProductionItemDefinition,
};
use crate::content::{ReferenceItemField, ReferenceItemSemantics};
use crate::domain::item_definition::TypedDefinitionRef;
use crate::durability::item_text::{ItemTextWriteError, MailTextPolicy};

type Result<T> = std::result::Result<T, ItemTextWriteError>;

pub trait NativeGameplayMailTextPolicyExt {
    fn mail_text_policy(&self, definition: &TypedDefinitionRef) -> Result<MailTextPolicy>;
}

impl NativeGameplayMailTextPolicyExt for NativeGameplayState {
    /// Sole producer of MailTextPolicy: lookup is in this decoded generation;
    /// the exact definition and this generation's digest travel together.
    fn mail_text_policy(&self, definition: &TypedDefinitionRef) -> Result<MailTextPolicy> {
        let policy = self
            .item_policy(&definition.production_key, &definition.revision_ref)
            .ok_or(ItemTextWriteError::NotWritable)?;
        let record = policy.record();
        let maximum = qualified_profile(
            &record.production_definition,
            &record.admission,
            &record.semantics,
            definition,
        )?;
        Ok(MailTextPolicy::from_qualified_content(
            definition.clone(),
            policy.source_digest(),
            maximum,
        ))
    }
}

/// Validate actual decoded fields. This helper produces no writable policy;
/// only the NativeGameplayState adapter above supplies the bound generation.
fn qualified_profile(
    definition: &NativeProductionItemDefinition,
    admission: &NativeItemAdmission,
    semantics: &ReferenceItemSemantics,
    requested: &TypedDefinitionRef,
) -> Result<u32> {
    let ReferenceItemField::Known(text) = &semantics.readable_writeable else {
        return Err(ItemTextWriteError::NotWritable);
    };
    let ReferenceItemField::Known(maximum @ 1..=4_000) = text.max_text_length else {
        return Err(ItemTextWriteError::NotWritable);
    };
    // MAIL keeps its accepted limits even if a mismatched content profile is supplied.
    let mail_limit_matches = match requested.production_key.as_str() {
        "oteryn:item.tibia.i3505" => maximum == 1_999,
        "oteryn:item.tibia.i3507" => maximum == 79,
        "oteryn:item.tibia.i3506" => false, // stamped letter
        _ => true,
    };
    if definition.family != "Item"
        || definition.family != requested.family
        || definition.production_key != requested.production_key
        || definition.revision_ref != requested.revision_ref
        || !mail_limit_matches
        || !admission.materializable
        || admission.stack_class != NativeItemStackClass::NonStackable
        || !admission
            .legal_destinations
            .contains(&NativeItemDestination::CharacterInventory)
        || !matches!(
            &semantics.readable_writeable,
            ReferenceItemField::Known(text)
                if text.readable == ReferenceItemField::Known(true)
                    && text.writeable == ReferenceItemField::Known(true)
                    && text.max_text_length == ReferenceItemField::Known(maximum)
                    && matches!(text.write_once_target, ReferenceItemField::NotApplicable)
        )
    {
        return Err(ItemTextWriteError::NotWritable);
    }
    Ok(maximum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::{ReferenceItemReadableWriteable, ReferenceItemTarget};

    // Explicit unit inputs for the decoded-field validator. These do not create
    // a NativeGameplayState, MailTextPolicy or current/admitted Content fact.
    fn fields(
        key: &str,
        maximum: u32,
    ) -> (
        NativeProductionItemDefinition,
        NativeItemAdmission,
        ReferenceItemSemantics,
        TypedDefinitionRef,
    ) {
        (
            NativeProductionItemDefinition {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: "definition-r1".into(),
            },
            NativeItemAdmission {
                materializable: true,
                stack_class: NativeItemStackClass::NonStackable,
                legal_destinations: vec![NativeItemDestination::CharacterInventory],
            },
            ReferenceItemSemantics {
                readable_writeable: ReferenceItemField::Known(ReferenceItemReadableWriteable {
                    readable: ReferenceItemField::Known(true),
                    writeable: ReferenceItemField::Known(true),
                    distance_read: ReferenceItemField::Unknown,
                    max_text_length: ReferenceItemField::Known(maximum),
                    write_once_target: ReferenceItemField::NotApplicable,
                }),
                ..ReferenceItemSemantics::default()
            },
            TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: "definition-r1".into(),
            },
        )
    }

    #[test]
    fn text_profile_keeps_letter_and_label_limits() {
        for (key, maximum) in [
            ("oteryn:item.tibia.i3505", 1_999),
            ("oteryn:item.tibia.i3507", 79),
        ] {
            let (definition, admission, semantics, requested) = fields(key, maximum);
            assert!(matches!(
                qualified_profile(&definition, &admission, &semantics, &requested),
                Ok(actual) if actual == maximum
            ));
            for different in [0, maximum - 1, maximum + 1] {
                let (_, _, changed, _) = fields(key, different);
                assert!(qualified_profile(&definition, &admission, &changed, &requested).is_err());
            }
        }
    }

    #[test]
    fn text_profile_requires_exact_definition_and_inventory_admission() {
        let (definition, admission, semantics, requested) =
            fields("oteryn:item.tibia.i3505", 1_999);
        for changed in [
            TypedDefinitionRef {
                family: "Creature".into(),
                ..requested.clone()
            },
            TypedDefinitionRef {
                production_key: "oteryn:item.tibia.i3507".into(),
                ..requested.clone()
            },
            TypedDefinitionRef {
                revision_ref: "definition-r2".into(),
                ..requested.clone()
            },
        ] {
            assert!(qualified_profile(&definition, &admission, &semantics, &changed).is_err());
        }
        for destinations in [vec![], vec![NativeItemDestination::Ground]] {
            let changed = NativeItemAdmission {
                legal_destinations: destinations,
                ..admission.clone()
            };
            assert!(qualified_profile(&definition, &changed, &semantics, &requested).is_err());
        }
    }

    #[test]
    fn text_profile_refuses_unadmitted_mail_definitions() {
        let (definition, admission, semantics, requested) =
            fields("oteryn:item.tibia.i3505", 1_999);
        let changed = NativeItemAdmission {
            materializable: false,
            ..admission.clone()
        };
        assert!(qualified_profile(&definition, &changed, &semantics, &requested).is_err());
        let (other, _, _, other_requested) = fields("oteryn:item.tibia.i3506", 1_999);
        assert!(qualified_profile(&other, &admission, &semantics, &other_requested).is_err());
        let changed = NativeProductionItemDefinition {
            family: "Creature".into(),
            ..definition.clone()
        };
        assert!(qualified_profile(&changed, &admission, &semantics, &requested).is_err());
    }

    #[test]
    fn text_profile_refuses_unknown_conflicting_and_write_once_facts()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let (definition, admission, semantics, requested) =
            fields("oteryn:item.tibia.i3505", 1_999);
        let ReferenceItemField::Known(text) = &semantics.readable_writeable else {
            return Err("explicit decoded-field fixture missing readable/writeable facts".into());
        };
        for field in [
            ReferenceItemField::Unknown,
            ReferenceItemField::NotApplicable,
            ReferenceItemField::Conflict,
        ] {
            let changed = ReferenceItemSemantics {
                readable_writeable: field,
                ..semantics.clone()
            };
            assert!(qualified_profile(&definition, &admission, &changed, &requested).is_err());
        }
        for field in [
            ReferenceItemField::Unknown,
            ReferenceItemField::NotApplicable,
            ReferenceItemField::Conflict,
            ReferenceItemField::Known(false),
        ] {
            for writeable in [false, true] {
                let mut changed_text = text.clone();
                if writeable {
                    changed_text.writeable = field.clone();
                } else {
                    changed_text.readable = field.clone();
                }
                let changed = ReferenceItemSemantics {
                    readable_writeable: ReferenceItemField::Known(changed_text),
                    ..semantics.clone()
                };
                assert!(qualified_profile(&definition, &admission, &changed, &requested).is_err());
            }
        }
        for field in [
            ReferenceItemField::Unknown,
            ReferenceItemField::NotApplicable,
            ReferenceItemField::Conflict,
        ] {
            let mut changed_text = text.clone();
            changed_text.max_text_length = field;
            let changed = ReferenceItemSemantics {
                readable_writeable: ReferenceItemField::Known(changed_text),
                ..semantics.clone()
            };
            assert!(qualified_profile(&definition, &admission, &changed, &requested).is_err());
        }
        for field in [
            ReferenceItemField::Unknown,
            ReferenceItemField::Conflict,
            ReferenceItemField::Known(ReferenceItemTarget {
                key: "oteryn:item.tibia.i3506".into(),
                revision: "definition-r1".into(),
            }),
        ] {
            let mut changed_text = text.clone();
            changed_text.write_once_target = field;
            let changed = ReferenceItemSemantics {
                readable_writeable: ReferenceItemField::Known(changed_text),
                ..semantics.clone()
            };
            assert!(qualified_profile(&definition, &admission, &changed, &requested).is_err());
        }
        Ok(())
    }
    #[test]
    fn write_profile_uses_content_limits_and_refuses_stacks() {
        for maximum in [1, 511, 1023, 1500, 3997, 4000] {
            let (definition, mut admission, semantics, requested) =
                fields("oteryn:item.tibia.i2815", maximum);
            assert!(matches!(
                qualified_profile(&definition, &admission, &semantics, &requested),
                Ok(actual) if actual == maximum
            ));
            admission.stack_class = NativeItemStackClass::StackCapable;
            assert!(qualified_profile(&definition, &admission, &semantics, &requested).is_err());
        }
        for maximum in [0, 4001, u32::MAX] {
            let (definition, admission, semantics, requested) =
                fields("oteryn:item.tibia.i2815", maximum);
            assert!(qualified_profile(&definition, &admission, &semantics, &requested).is_err());
        }
    }

    #[test]
    fn write_profile_current_production_content_admits_only_qualified_rewrites()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        use crate::content::native_gameplay::test_support::activated_with_loot_tables;
        let world = crate::foundation::WorldId::decode(&crate::bestiary_postgres_harness::id(42))?;
        let (controller, _) = activated_with_loot_tables(world, None);
        let native = controller
            .active()
            .and_then(|active| active.native_gameplay())
            .ok_or("production gameplay activation missing")?;
        // Scroll is the existing materializable rewrite; MAIL E09 admits the letter and the
        // parcel label (Canary items.xml 3505, 3507) at their accepted limits. Do not widen
        // mint permission for the content-only books, the stamped letter or blackboards.
        let requested = TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2815".into(),
            revision_ref: "definition-r1".into(),
        };
        native.mail_text_policy(&requested)?;
        for id in [3505, 3507] {
            native.mail_text_policy(&TypedDefinitionRef {
                production_key: format!("oteryn:item.tibia.i{id}"),
                ..requested.clone()
            })?;
        }
        for id in [
            173, 177, 2598, 2605, 2813, 2814, 2816, 2817, 2818, 2819, 2824, 2834, 2837, 2839, 3506,
            6386, 6533, 6538, 7879, 8747, 27932, 27933, 27934,
        ] {
            let other = TypedDefinitionRef {
                production_key: format!("oteryn:item.tibia.i{id}"),
                ..requested.clone()
            };
            assert!(matches!(
                native.mail_text_policy(&other),
                Err(ItemTextWriteError::NotWritable)
            ));
        }
        for other in [
            TypedDefinitionRef {
                revision_ref: "definition-r2".into(),
                ..requested.clone()
            },
            TypedDefinitionRef {
                family: "Document".into(),
                ..requested
            },
        ] {
            assert!(matches!(
                native.mail_text_policy(&other),
                Err(ItemTextWriteError::NotWritable)
            ));
        }
        Ok(())
    }
}
