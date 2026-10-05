//! Source appearance data qualification using the actual current native artifact owners.
//! This lowers a source-bound inline operation; it grants no cast/schedule/target authority.
//! Existing source profile consumers must independently validate the whole scheduled body,
//! scope fence, target/session, source actor generation, current tile policy and replay.
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{ProjectV2Family, ProjectV2InlineEffect, ProjectV2InlineEffectOperation};
use crate::foundation::ChannelRuntimeV1;
use crate::foundation::{ConditionDefinition, ConditionValues};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AppearanceContentError {
    ContentChanged,
    UnsupportedShape,
    MissingQualifiedMember,
    InvalidDefinition,
}

pub(crate) fn lower_current_appearance(
    runtime: &ChannelRuntimeV1,
    content: &NativeGameplayState,
    effect: &ProjectV2InlineEffect,
) -> Result<ConditionDefinition, AppearanceContentError> {
    if content.source_digest() != runtime.content_pin().server_artifact_digest() {
        return Err(AppearanceContentError::ContentChanged);
    }
    let ProjectV2InlineEffectOperation::AppearanceTransform {
        duration_ms,
        creature,
        item,
    } = &effect.operation
    else {
        return Err(AppearanceContentError::UnsupportedShape);
    };
    let duration_ms = u32::try_from(*duration_ms)
        .ok()
        .filter(|v| *v > 0)
        .ok_or(AppearanceContentError::InvalidDefinition)?;
    match (creature, item) {
        (Some(creature), None) if creature.family == ProjectV2Family::Creature => {
            let registry = content
                .spell_appearances()
                .ok_or(AppearanceContentError::MissingQualifiedMember)?;
            if registry.source_digest() != content.source_digest() {
                return Err(AppearanceContentError::ContentChanged);
            }
            lower_creature_member(registry, creature, &effect.key, duration_ms)
        }
        (None, Some(item)) if item.family == ProjectV2Family::Item => {
            // Resolve appearance-only canonical membership, never a durable Item grant.
            let registry = content
                .spell_appearances()
                .ok_or(AppearanceContentError::MissingQualifiedMember)?;
            if registry.source_digest() != content.source_digest() {
                return Err(AppearanceContentError::ContentChanged);
            }
            let member = registry
                .for_item(&item.key, &item.revision)
                .ok_or(AppearanceContentError::MissingQualifiedMember)?;
            ConditionDefinition::new(&effect.key, 1, ConditionValues::ItemOutfit { duration_ms })
                .and_then(|definition| {
                    definition.with_item_appearance(
                        &member.key,
                        &member.revision,
                        content.source_digest(),
                    )
                })
                .ok_or(AppearanceContentError::InvalidDefinition)
        }
        _ => Err(AppearanceContentError::UnsupportedShape),
    }
}

fn lower_creature_member(
    registry: &crate::content::native_spell_appearances::CompiledSpellAppearances,
    creature: &crate::content::ProjectV2DefinitionRef,
    effect_key: &str,
    duration_ms: u32,
) -> Result<ConditionDefinition, AppearanceContentError> {
    if creature.family != ProjectV2Family::Creature {
        return Err(AppearanceContentError::UnsupportedShape);
    }
    let member = registry
        .for_creature(&creature.key, &creature.revision)
        .ok_or(AppearanceContentError::MissingQualifiedMember)?;
    ConditionDefinition::new(
        effect_key,
        1,
        ConditionValues::Outfit {
            duration_ms,
            look_type: member.look_type(),
        },
    )
    .and_then(|definition| definition.with_appearance(member.selection()))
    .ok_or(AppearanceContentError::InvalidDefinition)
}
#[cfg(test)]
#[path = "creature_appearance_content_tests.rs"]
mod tests;
