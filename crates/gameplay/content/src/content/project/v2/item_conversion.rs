//! CRAFT-1 authored conversions. Validation admits data only; no execution or durable writes.

mod digging;

use super::*;
use crate::content::{ReferenceItemField, ReferenceItemSemantics, ReferenceItemTarget};
use crate::domain::timed_item_host::TimedDefinitionSource;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ItemConversion {
    pub input_item: ProjectV2DefinitionRef,
    pub target: ProjectV2ConversionTarget,
    pub operands: Vec<ProjectV2ConversionOperand>,
    pub outputs: ProjectV2ConversionOutputs,
    pub tool: ProjectV2ConversionTool,
    pub capacity_check: bool,
    pub result_message: ProjectV2ConversionMessages,
    pub source: ProjectV2ConversionSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skill: Option<ProjectV2ConversionSkill>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chance_curve: Option<ProjectV2ConversionCurve>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tile_cooldown_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revert_ms: Option<u32>,
    /// A volatile C5 overlay change, never a durable map-target operand effect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opened_target: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_effect: Option<ProjectV2ConversionTargetEffect>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectV2ConversionTarget {
    None,
    Carried,
    MapItem { definition: ProjectV2DefinitionRef },
    MapObject { definition: ProjectV2DefinitionRef },
    TileKind { definition: ProjectV2DefinitionRef },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ConversionOperand {
    pub role: ProjectV2ConversionRole,
    pub item_id: ProjectV2DefinitionRef,
    pub count: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_type: Option<u16>,
    pub effect: ProjectV2ConversionEffect,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consume_on: Option<ProjectV2ConversionConsumeOn>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_item_id: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<ProjectV2ConversionLifecycle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_sub_type: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProjectV2ConversionRole {
    Used,
    Target,
    Ingredient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProjectV2ConversionEffect {
    Consume,
    TransformTo,
    Keep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProjectV2ConversionConsumeOn {
    Always,
    Success,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProjectV2ConversionLifecycle {
    PreserveInstance,
    ReplaceInstance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2ConversionTool {
    None,
    Rod,
    Shovel,
    Pick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2ConversionSkill {
    Fishing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ConversionOutput {
    pub item_id: ProjectV2DefinitionRef,
    pub count: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ConversionAlternative {
    pub weight: u32,
    /// Exactly one output or the empty alternative.
    pub entries: Vec<ProjectV2ConversionOutput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectV2ConversionOutputs {
    Fixed {
        entries: Vec<ProjectV2ConversionOutput>,
    },
    Weighted {
        alternatives: Vec<ProjectV2ConversionAlternative>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "effect",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum ProjectV2ConversionTargetEffect {
    Retire,
    /// C4b always preserves the corpse instance and its contents.
    TransformTo {
        output_item_id: ProjectV2DefinitionRef,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ConversionCurve {
    pub base: u32,
    pub per_skill: u32,
    pub skill_offset: u16,
    pub minimum: u32,
    pub maximum: u32,
    pub roll_scale: u16,
    pub roll_max: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ConversionMessages {
    pub success: String,
    pub no_room: String,
    pub too_heavy: String,
    pub no_fish: String,
    pub not_owner: String,
    pub empty: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ConversionSource {
    pub repository: String,
    pub revision: String,
    pub path: String,
    pub sha256: String,
    pub lines: u32,
    pub line_start: u32,
    pub line_end: u32,
    pub evidence: ProjectV2EvidenceClass,
    pub differences: Vec<String>,
}

impl ProjectV2ItemConversion {
    pub(super) fn references(&self) -> Vec<(ProjectV2Family, &ProjectV2DefinitionRef)> {
        let mut refs = vec![(ProjectV2Family::Item, &self.input_item)];
        match &self.target {
            ProjectV2ConversionTarget::None | ProjectV2ConversionTarget::Carried => {}
            ProjectV2ConversionTarget::MapItem { definition } => {
                refs.push((ProjectV2Family::Item, definition))
            }
            ProjectV2ConversionTarget::MapObject { definition } => {
                refs.push((definition.family, definition))
            }
            ProjectV2ConversionTarget::TileKind { definition } => {
                refs.push((definition.family, definition))
            }
        }
        for operand in &self.operands {
            refs.push((ProjectV2Family::Item, &operand.item_id));
            if let Some(output) = &operand.output_item_id {
                refs.push((ProjectV2Family::Item, output));
            }
        }
        for entry in self.output_entries() {
            refs.push((ProjectV2Family::Item, &entry.item_id));
        }
        if let Some(output) = &self.opened_target {
            refs.push((ProjectV2Family::Item, output));
        }
        if let Some(ProjectV2ConversionTargetEffect::TransformTo { output_item_id }) =
            &self.target_effect
        {
            refs.push((ProjectV2Family::Item, output_item_id));
        }
        refs
    }

    fn output_entries(&self) -> Vec<&ProjectV2ConversionOutput> {
        match &self.outputs {
            ProjectV2ConversionOutputs::Fixed { entries } => entries.iter().collect(),
            ProjectV2ConversionOutputs::Weighted { alternatives } => {
                alternatives.iter().flat_map(|a| &a.entries).collect()
            }
        }
    }

    pub(super) fn validate_shape(&self, limits: ProjectEvidenceLimits) -> Result<(), ProjectError> {
        use ProjectV2ConversionEffect::{Consume, Keep, TransformTo};
        use ProjectV2ConversionRole::{Ingredient, Target, Used};
        let used: Vec<_> = self.operands.iter().filter(|o| o.role == Used).collect();
        let targets = self.operands.iter().filter(|o| o.role == Target).count();
        let ingredients: Vec<_> = self
            .operands
            .iter()
            .filter(|o| o.role == Ingredient)
            .collect();
        if self.operands.len() > 4
            || used.len() != 1
            || targets > 1
            || ingredients.len() > 2
            || (targets == 1) != matches!(self.target, ProjectV2ConversionTarget::Carried)
            || used[0].item_id != self.input_item
            || !self.capacity_check
        {
            return Err(ProjectError::InvalidProject(
                "item_conversion operand roles or capacity policy",
            ));
        }
        if let ProjectV2ConversionTarget::TileKind { definition } = &self.target
            && !matches!(
                definition.family,
                ProjectV2Family::Item | ProjectV2Family::Terrain
            )
        {
            return Err(ProjectError::InvalidProject(
                "item_conversion tile target family",
            ));
        }
        if let ProjectV2ConversionTarget::MapObject { definition } = &self.target
            && !matches!(
                definition.family,
                ProjectV2Family::WorldObject | ProjectV2Family::LocalObject
            )
        {
            return Err(ProjectError::InvalidProject(
                "item_conversion map object target family",
            ));
        }
        let mut ingredient_ids = BTreeSet::new();
        for operand in &self.operands {
            let transform = operand.effect == TransformTo;
            if !(1..=100).contains(&operand.count)
                || (operand.role == Ingredient) != operand.consume_on.is_some()
                || (operand.role == Ingredient && operand.effect != Consume)
                || transform != operand.output_item_id.is_some()
                || transform != operand.lifecycle.is_some()
                || (transform && operand.sub_type.is_some()) != operand.output_sub_type.is_some()
                || (operand.role == Ingredient
                    && !ingredient_ids.insert((&operand.item_id, operand.sub_type)))
            {
                return Err(ProjectError::InvalidProject(
                    "item_conversion operand lifecycle or quantity",
                ));
            }
        }
        match &self.outputs {
            ProjectV2ConversionOutputs::Fixed { entries } if entries.len() > 3 => {
                return Err(ProjectError::InvalidProject(
                    "item_conversion fixed output ceiling",
                ));
            }
            ProjectV2ConversionOutputs::Weighted { alternatives }
                if !(2..=24).contains(&alternatives.len())
                    || alternatives
                        .iter()
                        .any(|a| a.weight == 0 || a.entries.len() > 1)
                    || alternatives
                        .iter()
                        .try_fold(0_u32, |total, a| total.checked_add(a.weight))
                        .is_none() =>
            {
                return Err(ProjectError::InvalidProject(
                    "item_conversion weighted output ceiling or weights",
                ));
            }
            _ => {}
        }
        if self
            .output_entries()
            .iter()
            .any(|o| !(1..=100).contains(&o.count))
        {
            return Err(ProjectError::InvalidProject(
                "item_conversion output quantity",
            ));
        }
        let fishing = self.skill.is_some();
        if fishing != self.chance_curve.is_some()
            || fishing != self.tile_cooldown_ms.is_some()
            || self
                .tile_cooldown_ms
                .is_some_and(|ms| !(1..=3600000).contains(&ms))
        {
            return Err(ProjectError::InvalidProject(
                "item_conversion fishing fields",
            ));
        }
        if fishing
            && (self.tool != ProjectV2ConversionTool::Rod
                || used[0].effect != Keep
                || !matches!(self.target, ProjectV2ConversionTarget::TileKind { .. })
                || targets != 0
                || ingredients.len() != 1
                || ingredients[0].count != 1
                || ingredients[0].consume_on != Some(ProjectV2ConversionConsumeOn::Success))
        {
            return Err(ProjectError::InvalidProject(
                "item_conversion fishing operands",
            ));
        }
        if let Some(curve) = &self.chance_curve
            && (
                curve.base,
                curve.per_skill,
                curve.skill_offset,
                curve.minimum,
                curve.maximum,
                curve.roll_scale,
                curve.roll_max,
            ) != (10000, 597, 10, 10000, 50000, 1000, 100)
        {
            return Err(ProjectError::InvalidProject(
                "item_conversion fishing curve",
            ));
        }
        let digging = matches!(
            self.tool,
            ProjectV2ConversionTool::Shovel | ProjectV2ConversionTool::Pick
        );
        if digging != self.revert_ms.is_some()
            || digging != self.opened_target.is_some()
            || self
                .revert_ms
                .is_some_and(|ms| !(1000..=3600000).contains(&ms))
            || (digging
                && (fishing
                    || self.operands.len() != 1
                    || used[0].effect != Keep
                    || !matches!(
                        self.target,
                        ProjectV2ConversionTarget::MapItem { .. }
                            | ProjectV2ConversionTarget::MapObject { .. }
                    )
                    || !matches!(&self.outputs, ProjectV2ConversionOutputs::Fixed { entries } if entries.is_empty())
                    || self.target_effect.is_some()))
        {
            return Err(ProjectError::InvalidProject(
                "item_conversion digging fields",
            ));
        }
        if self.target_effect.is_some()
            && (fishing
                || digging
                || self.operands.len() != 1
                || used[0].effect != Keep
                || !matches!(self.target, ProjectV2ConversionTarget::MapItem { .. }))
        {
            return Err(ProjectError::InvalidProject(
                "item_conversion corpse target shape",
            ));
        }
        for text in [
            &self.result_message.success,
            &self.result_message.no_room,
            &self.result_message.too_heavy,
            &self.result_message.no_fish,
            &self.result_message.not_owner,
            &self.result_message.empty,
        ] {
            if !text.is_empty() {
                validate_v2_source_text("item_conversion result message", text, limits)?;
            }
        }
        Ok(())
    }
}

/// Reuse the reference Item semantics where admitted. Source-only Items stay source-only:
/// the snapshot supplies shape evidence without promoting materializability or runtime behavior.
pub(super) fn validate_item_conversions(
    state: &ProjectV2State,
    records: &[ProjectReferenceRecord],
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    if !state
        .declarations
        .iter()
        .any(|d| matches!(d, ProjectV2Declaration::ItemConversion { .. }))
    {
        return Ok(());
    }
    let catalogue: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../../content/craft/recipes.json"
    ))
    .map_err(|e| ProjectError::InvalidJson(e.to_string()))?;
    let snapshot: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../../content/craft/sources.json"
    ))
    .map_err(|e| ProjectError::InvalidJson(e.to_string()))?;
    for declaration in &state.declarations {
        let ProjectV2Declaration::ItemConversion {
            identity, recipe, ..
        } = declaration
        else {
            continue;
        };
        let expected_source = catalogue["records"]
            .as_array()
            .and_then(|rows| {
                rows.iter().find(|r| {
                    r["identity"]["key"].as_str() == Some(identity.key.as_str())
                        && r["identity"]["revision"].as_str() == Some(identity.revision.as_str())
                })
            })
            .map(|r| &r["recipe"]["source"]);
        let authored_source = serde_json::to_value(&recipe.source)
            .map_err(|e| ProjectError::InvalidJson(e.to_string()))?;
        if expected_source != Some(&authored_source) {
            return Err(ProjectError::InvalidProject(
                "item_conversion source substituted for row",
            ));
        }
        let source = &recipe.source;
        let pinned = snapshot["sources"].as_object().and_then(|sources| {
            sources.values().find(|s| {
                s["path"].as_str() == Some(source.path.as_str())
                    && s["revision"].as_str() == Some(source.revision.as_str())
                    && s["repository"].as_str() == Some(source.repository.as_str())
                    && s["sha256"].as_str() == Some(source.sha256.as_str())
                    && s["lines"].as_u64() == Some(u64::from(source.lines))
            })
        });
        if pinned.is_none()
            || source.line_start == 0
            || source.line_start > source.line_end
            || source.line_end > source.lines
            || source.evidence != ProjectV2EvidenceClass::OtsHypothesisOnly
        {
            return Err(ProjectError::InvalidProject(
                "item_conversion source provenance mismatch",
            ));
        }
        for difference in &source.differences {
            validate_v2_source_text("item_conversion source difference", difference, limits)?;
        }
        let quantities = recipe
            .operands
            .iter()
            .flat_map(|o| {
                std::iter::once((&o.item_id, o.count))
                    .chain(o.output_item_id.as_ref().map(|r| (r, o.count)))
            })
            .chain(
                recipe
                    .output_entries()
                    .into_iter()
                    .map(|o| (&o.item_id, o.count)),
            );
        for (reference, count) in quantities {
            validate_quantity(reference, count, records, &snapshot)?;
        }
        for operand in &recipe.operands {
            let semantics = records.iter().find_map(|r| match r {
                ProjectReferenceRecord::Item {
                    identity,
                    semantics,
                    ..
                } if identity.key == operand.item_id.key => Some(semantics),
                _ => None,
            });
            if semantics
                .map(|s| timed(s, &operand.item_id))
                .transpose()?
                .unwrap_or(false)
                && operand.effect != ProjectV2ConversionEffect::Keep
            {
                return Err(ProjectError::InvalidProject(
                    "item_conversion timed operand must KEEP",
                ));
            }
            let id = operand
                .item_id
                .key
                .strip_prefix("oteryn:item.tibia.i")
                .unwrap_or("");
            // C1 also adopts the baking source's holy-water flask, whose appearance
            // lacks liquidcontainer even though the source explicitly matches its subtype.
            let fluid = snapshot["item_facts"][id]["fluid_container"].as_bool() == Some(true)
                || snapshot["facts"]["baking"]["tables"]["liquidContainers"]
                    .as_array()
                    .is_some_and(|ids| {
                        ids.iter().any(|value| {
                            value.as_u64().map(|value| value.to_string()).as_deref() == Some(id)
                        })
                    });
            if fluid != operand.sub_type.is_some() {
                return Err(ProjectError::InvalidProject(
                    "item_conversion fluid subtype required only for fluid operands",
                ));
            }
        }
        digging::validate(recipe)?;
        // The accepted C4b exception is source-qualified; ordinary map targets remain read-only.
        if recipe.target_effect.is_some() {
            let ProjectV2ConversionTarget::MapItem { definition } = &recipe.target else {
                return Err(ProjectError::InvalidProject(
                    "item_conversion target effect requires map item",
                ));
            };
            if !snapshot["corpse_targets"]
                .as_array()
                .is_some_and(|ids| ids.iter().any(|v| v.as_str() == Some(&definition.key)))
            {
                return Err(ProjectError::InvalidProject(
                    "item_conversion target effect requires C4b corpse",
                ));
            }
        }
    }
    Ok(())
}

// Reuse the existing TIMED-ITEM-0B adapter, including its conflict rejection.
fn timed(
    semantics: &ReferenceItemSemantics,
    reference: &ProjectV2DefinitionRef,
) -> Result<bool, ProjectError> {
    let target = ReferenceItemTarget::new(&reference.key, &reference.revision)?;
    semantics
        .resolve_timed_definition(&target)
        .map(|definition| definition.is_some())
        .map_err(|_| ProjectError::InvalidProject("item_conversion invalid timed definition"))
}

fn validate_quantity(
    reference: &ProjectV2DefinitionRef,
    count: u16,
    records: &[ProjectReferenceRecord],
    snapshot: &serde_json::Value,
) -> Result<(), ProjectError> {
    let id = reference
        .key
        .strip_prefix("oteryn:item.tibia.i")
        .unwrap_or("");
    let mut maximum = snapshot["item_facts"][id]["max_stack"].as_u64();
    let mut found = false;
    for record in records {
        if let ProjectReferenceRecord::Item {
            identity,
            stack_class,
            semantics,
            ..
        } = record
            && identity.key == reference.key
            && identity.revision == reference.revision
        {
            found = true;
            if matches!(
                semantics.stack,
                ReferenceItemField::Conflict | ReferenceItemField::NotApplicable
            ) {
                return Err(ProjectError::InvalidProject(
                    "item_conversion item stack evidence conflict",
                ));
            }
            let admitted = match stack_class {
                ItemStackDocument::NonStackable => Some(1),
                ItemStackDocument::StackCapable => match &semantics.stack {
                    ReferenceItemField::Known(stack) => match stack.stack_max {
                        ReferenceItemField::Known(max) => Some(u64::from(max)),
                        ReferenceItemField::Unknown => None,
                        ReferenceItemField::NotApplicable | ReferenceItemField::Conflict => {
                            return Err(ProjectError::InvalidProject(
                                "item_conversion item stack maximum conflict",
                            ));
                        }
                    },
                    ReferenceItemField::Unknown
                    | ReferenceItemField::NotApplicable
                    | ReferenceItemField::Conflict => None,
                },
                ItemStackDocument::Unknown => None,
            };
            if let Some(admitted) = admitted {
                maximum = Some(maximum.map_or(admitted, |source| source.min(admitted)));
            }
            if timed(semantics, reference)? && (count != 1 || maximum != Some(1)) {
                return Err(ProjectError::InvalidProject(
                    "item_conversion timed output must be one non-stackable instance",
                ));
            }
        }
    }
    if !found || !(1..=100).contains(&count) || maximum.is_none_or(|max| u64::from(count) > max) {
        return Err(ProjectError::InvalidProject(
            "item_conversion quantity exceeds item max_stack",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "item_conversion_tests.rs"]
mod tests;
