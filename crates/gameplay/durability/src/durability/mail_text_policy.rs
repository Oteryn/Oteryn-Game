//! D1113: actual Native Content to the closed MAIL-TEXT leaf policy.
//! A qualified profile supplies immutable expected facts, never current custody
//! or activation authority. The durability writer independently checks those.

use crate::content::native_gameplay::{
    NativeGameplayState, NativeItemAdmission, NativeItemDestination, NativeProductionItemDefinition,
};
use crate::content::{ReferenceItemField, ReferenceItemSemantics};
use crate::domain::item_definition::TypedDefinitionRef;
use crate::domain::mail::MailTextClass;
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
        let (class, maximum) = qualified_profile(
            &record.production_definition,
            &record.admission,
            &record.semantics,
            definition,
        )?;
        Ok(MailTextPolicy::from_qualified_content(
            definition.clone(),
            policy.source_digest(),
            maximum,
            class,
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
) -> Result<(MailTextClass, u32)> {
    let (class, maximum) = match requested.production_key.as_str() {
        "oteryn:item.tibia.i3505" => (MailTextClass::Letter, 1_999),
        "oteryn:item.tibia.i3507" => (MailTextClass::Label, 79),
        _ => return Err(ItemTextWriteError::NotWritable),
    };
    if definition.family != "Item"
        || definition.family != requested.family
        || definition.production_key != requested.production_key
        || definition.revision_ref != requested.revision_ref
        || !admission.materializable
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
    Ok((class, maximum))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::native_gameplay::NativeItemStackClass;
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
        for (key, maximum, class) in [
            ("oteryn:item.tibia.i3505", 1_999, MailTextClass::Letter),
            ("oteryn:item.tibia.i3507", 79, MailTextClass::Label),
        ] {
            let (definition, admission, semantics, requested) = fields(key, maximum);
            assert!(matches!(
                qualified_profile(&definition, &admission, &semantics, &requested),
                Ok(actual) if actual == (class, maximum)
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
}
