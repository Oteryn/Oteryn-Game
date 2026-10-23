//! Read timed values and paired forms only from the admitted native generation.
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{
    ReferenceItemField, ReferenceItemTarget, ReferenceTemporalMode, ReferenceTransformKind,
};
use crate::domain::timed_item_host::resolve_timed_definition;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_timed_state::{TimedDefinitionFacts, TimedExpiryTarget};
fn typed(target: &ReferenceItemTarget) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: target.key.clone(),
        revision_ref: target.revision.clone(),
    }
}
fn transform(
    native: &NativeGameplayState,
    definition: &TypedDefinitionRef,
    kind: ReferenceTransformKind,
) -> Option<TypedDefinitionRef> {
    let policy = native.item_policy(&definition.production_key, &definition.revision_ref)?;
    let ReferenceItemField::Known(transforms) = &policy.record().semantics.use_transform else {
        return None;
    };
    transforms.targets.iter().find_map(|entry| {
        if entry.kind != kind {
            return None;
        }
        match &entry.target {
            ReferenceItemField::Known(target) => Some(typed(target)),
            _ => None,
        }
    })
}

/// Inactive equip forms take their full values from the admitted paired active definition.
pub(super) fn facts(
    native: &NativeGameplayState,
    definition: &TypedDefinitionRef,
) -> Option<(TimedDefinitionFacts, Option<TimedExpiryTarget>)> {
    let policy = native.item_policy(&definition.production_key, &definition.revision_ref)?;
    if matches!(&policy.record().semantics.temporal, ReferenceItemField::Known(temporal)
        if temporal.consumption_mode == ReferenceItemField::Known(ReferenceTemporalMode::DurableAbsoluteDeadline))
    {
        return None;
    }
    let reference =
        ReferenceItemTarget::new(&definition.production_key, &definition.revision_ref).ok()?;
    let resolved = resolve_timed_definition(&reference, &policy.record().semantics).ok()?;
    let resolved = match resolved {
        Some(resolved) => resolved,
        None => {
            let active = transform(native, definition, ReferenceTransformKind::Equip)?;
            let policy = native.item_policy(&active.production_key, &active.revision_ref)?;
            resolve_timed_definition(
                &ReferenceItemTarget::new(&active.production_key, &active.revision_ref).ok()?,
                &policy.record().semantics,
            )
            .ok()??
        }
    };
    let target = resolved.decay_target.map(|target| {
        let definition = typed(&target);
        let policy = native.item_policy(&definition.production_key, &definition.revision_ref)?;
        let full = resolve_timed_definition(&target, &policy.record().semantics)
            .ok()?
            .map(|t| t.full);
        Some(TimedExpiryTarget {
            definition,
            full,
            lit_continuous: false,
        })
    });
    let target = match target {
        None => None,
        Some(Some(target)) => Some(target),
        Some(None) => return None,
    };
    Some((
        TimedDefinitionFacts {
            definition: definition.clone(),
            full: resolved.full,
            lit_continuous: false,
        },
        target,
    ))
}

pub(super) fn supported(active: Option<&crate::content::ActiveGeneration>) -> bool {
    let Some(native) = active.and_then(|active| active.native_gameplay()) else {
        return false;
    };
    let Some(index) = native.item_index() else {
        return false;
    };
    ["oteryn:item.tibia.i3081", "oteryn:item.tibia.i6529"]
        .iter()
        .all(|key| {
            let Some(revision) = index.revision(key) else {
                return false;
            };
            facts(
                native,
                &TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: (*key).into(),
                    revision_ref: revision.into(),
                },
            )
            .is_some()
        })
}
