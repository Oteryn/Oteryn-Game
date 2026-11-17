//! Exact immutable child-stage descriptions; runtime authority stays with owners.
use super::{NativeCallbackError, QualifiedNativeCallback};
use crate::content::{
    EffectFamilyDocument, ProjectReferenceRecord, ProjectV2AbilityArea, ProjectV2AbilityDetails,
    ProjectV2AbilityEffect, ProjectV2AbilityKind, ProjectV2AbilitySchedule,
    ProjectV2AuthoringProfileData as Data, ProjectV2DefinitionRef as Ref,
    ProjectV2FormulaAuthoring, ProjectV2Magnitude,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::OnceLock};
const SOURCE: &str =
    include_str!("../../../../../../apps/game-server/src/source_callback_cast_fixture.json");
const SOURCE_HASH: &str = "d92247a424de7d3c97566a789ec211f326d49e4cfeccaa87ff246e7882cb6c9b";
#[derive(serde::Deserialize)]
struct Witness {
    cases: BTreeMap<String, Case>,
    profiles: BTreeMap<String, Data>,
    records: Vec<ProjectReferenceRecord>,
}
#[derive(serde::Deserialize)]
struct Case {
    caster: String,
    index: usize,
    phase: String,
    encounter: String,
    casts: Vec<Stage>,
    entry: serde_json::Value,
}
#[derive(serde::Deserialize)]
struct Stage {
    delay_ms: Delay,
    action: serde_json::Value,
}
#[derive(serde::Deserialize)]
struct Delay {
    min: u64,
    max: u64,
}
fn source() -> Result<&'static Witness, NativeCallbackError> {
    static W: OnceLock<Result<Witness, NativeCallbackError>> = OnceLock::new();
    W.get_or_init(|| {
        let hash: String = Sha256::digest(SOURCE.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if hash != SOURCE_HASH {
            return Err(NativeCallbackError::InvalidSource);
        }
        serde_json::from_str(SOURCE).map_err(|_| NativeCallbackError::InvalidSource)
    })
    .as_ref()
    .map_err(|e| *e)
}
#[derive(Debug, Clone)]
struct ParsedStage {
    minimum: u32,
    maximum: u32,
    element: String,
    area: Option<ProjectV2AbilityArea>,
    direction: bool,
    presentation: Option<Box<ProjectV2AbilityDetails>>,
    armor: bool,
}
fn parse_stage(w: &Witness, c: &Case, stage: &Stage) -> Result<ParsedStage, NativeCallbackError> {
    if stage.delay_ms.min != stage.delay_ms.max
        || stage.delay_ms.max > 10000
        || stage
            .action
            .get("at")
            .and_then(|a| a.get("kind"))
            .and_then(|v| v.as_str())
            != Some("subject_position")
    {
        return Err(NativeCallbackError::InvalidSource);
    }
    let (minimum, maximum, element, area, direction, presentation, armor) = if let Some(child) =
        stage.action.get("ability")
    {
        let child: Ref = serde_json::from_value(child.clone())
            .map_err(|_| NativeCallbackError::InvalidSource)?;
        let Some(Data::Ability(a)) = w.profiles.get(&child.key) else {
            return Err(NativeCallbackError::InvalidSource);
        };
        let details = a
            .details
            .as_deref()
            .ok_or(NativeCallbackError::InvalidSource)?;
        if details.kind != ProjectV2AbilityKind::Spell
            || details.needs_target
            || details.encounter.is_some()
            || details.chain.is_some()
            || details.windup.is_some()
            || !details.variants.is_empty()
        {
            return Err(NativeCallbackError::InvalidSource);
        }
        let [ProjectV2AbilityEffect::Executable(effect)] = details.effects.as_slice() else {
            return Err(NativeCallbackError::InvalidSource);
        };
        let Some(ProjectReferenceRecord::Ability{effects,..})=w.records.iter().find(|r|matches!(r,ProjectReferenceRecord::Ability{identity,..}if identity.key==child.key&&identity.revision==child.revision))else{return Err(NativeCallbackError::InvalidSource)};
        if !effects
            .iter()
            .any(|e| e.key == effect.key && e.revision == effect.revision)
        {
            return Err(NativeCallbackError::InvalidSource);
        }
        let Some(ProjectReferenceRecord::Effect{effect_family:EffectFamilyDocument::Damage,formula,..})=w.records.iter().find(|r|matches!(r,ProjectReferenceRecord::Effect{identity,..}if identity.key==effect.key&&identity.revision==effect.revision))else{return Err(NativeCallbackError::InvalidSource)};
        let Some(Data::Effect(ep)) = w.profiles.get(&effect.key) else {
            return Err(NativeCallbackError::InvalidSource);
        };
        let Some(Data::Formula(ProjectV2FormulaAuthoring::Range { minimum, maximum })) =
            w.profiles.get(&formula.key)
        else {
            return Err(NativeCallbackError::InvalidSource);
        };
        if ep.affects.is_some()
            || ep
                .mitigated_by
                .contains(&crate::content::ProjectV2Mitigation::Shield)
        {
            return Err(NativeCallbackError::InvalidSource);
        }
        (
            *minimum,
            *maximum,
            ep.damage_type.clone(),
            details.area.clone(),
            details.needs_direction,
            None,
            ep.mitigated_by
                .contains(&crate::content::ProjectV2Mitigation::Armor),
        )
    } else {
        let Some(Data::Encounter(e)) = w.profiles.get(&c.encounter) else {
            return Err(NativeCallbackError::InvalidSource);
        };
        let details = e
            .details
            .as_ref()
            .ok_or(NativeCallbackError::InvalidSource)?;
        let ability = stage
            .action
            .get("encounter_ability")
            .and_then(|v| v.as_str())
            .ok_or(NativeCallbackError::InvalidSource)?;
        let a = details
            .abilities
            .iter()
            .find(|a| a.key == ability)
            .ok_or(NativeCallbackError::InvalidSource)?;
        if !a.affects.players
            || !a.affects.creatures.is_empty()
            || a.area.shape != crate::content::ProjectV2NearShape::Circle
        {
            return Err(NativeCallbackError::InvalidSource);
        }
        (
            a.damage.min,
            a.damage.max,
            a.damage.damage_type.clone(),
            Some(source_callback_area(&c.caster).ok_or(NativeCallbackError::InvalidSource)?),
            false,
            None,
            false,
        )
    };
    if minimum > maximum
        || !matches!(
            element.as_str(),
            "physical" | "energy" | "ice" | "fire" | "earth" | "death" | "holy"
        )
    {
        return Err(NativeCallbackError::InvalidSource);
    }
    let minimum = u32::try_from(minimum).map_err(|_| NativeCallbackError::InvalidSource)?;
    let maximum = u32::try_from(maximum).map_err(|_| NativeCallbackError::InvalidSource)?;

    Ok(ParsedStage {
        minimum,
        maximum,
        element,
        area,
        direction,
        presentation,
        armor,
    })
}
/// Sealed source DATA from an actual qualified parent; no synthetic profile.
#[derive(Debug, Clone)]
pub struct QualifiedNativeCallbackStage {
    parent: QualifiedNativeCallback,
    index: usize,
    delay_ms: u64,
    schedule: ProjectV2AbilitySchedule,
    parsed: ParsedStage,
    fingerprint: [u8; 32],
}
impl QualifiedNativeCallbackStage {
    pub fn parent(&self) -> &QualifiedNativeCallback {
        &self.parent
    }
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn delay_ms(&self) -> u64 {
        self.delay_ms
    }
    pub fn parent_schedule(&self) -> &ProjectV2AbilitySchedule {
        &self.schedule
    }
    pub fn range_tiles(&self) -> u16 {
        self.schedule.range_tiles.unwrap_or(0)
    }
    pub fn magnitude(&self) -> Option<ProjectV2Magnitude> {
        self.schedule.magnitude
    }
    pub fn minimum(&self) -> u32 {
        self.parsed.minimum
    }
    pub fn maximum(&self) -> u32 {
        self.parsed.maximum
    }
    pub fn element(&self) -> &str {
        &self.parsed.element
    }
    pub fn area(&self) -> Option<&ProjectV2AbilityArea> {
        self.parsed.area.as_ref()
    }
    pub fn needs_direction(&self) -> bool {
        self.parsed.direction
    }
    pub fn armor(&self) -> bool {
        self.parsed.armor
    }
    pub fn presentation(&self) -> Option<&ProjectV2AbilityDetails> {
        self.parsed.presentation.as_deref()
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn source_body_sha256(&self) -> &'static str {
        SOURCE_HASH
    }
}
impl QualifiedNativeCallback {
    pub fn stage(
        &self,
        index: usize,
    ) -> Result<Option<QualifiedNativeCallbackStage>, NativeCallbackError> {
        let w = source()?;
        let Some(c) = w.cases.get(&self.entry().ability.key) else {
            return Ok(None);
        };
        if c.caster != self.entry().creature.key
            || c.index != self.entry().index
            || c.phase != "attacks"
            || self.entry().phase != "attacks"
            || c.encounter != self.entry().encounter.key
            || w.profiles.get(&c.encounter) != Some(&self.encounter_profile()?.data)
        {
            return Err(NativeCallbackError::InvalidSource);
        }
        let schedule: ProjectV2AbilitySchedule = serde_json::from_value(c.entry.clone())
            .map_err(|_| NativeCallbackError::InvalidSource)?;
        if schedule.ability != self.entry().ability {
            return Err(NativeCallbackError::InvalidSource);
        }
        let Some(stage) = c.casts.get(index) else {
            return Ok(None);
        };
        let parsed = parse_stage(w, c, stage)?;
        let fingerprint = Sha256::digest(
            serde_json::to_vec(&(
                SOURCE_HASH,
                &self.entry().creature,
                &self.entry().ability,
                index,
                &stage.action,
                self.content(),
            ))
            .map_err(|_| NativeCallbackError::InvalidSource)?,
        )
        .into();
        Ok(Some(QualifiedNativeCallbackStage {
            parent: self.clone(),
            index,
            delay_ms: stage.delay_ms.min,
            schedule,
            parsed,
            fingerprint,
        }))
    }
}

/// Literal source area DATA, not a numeric Circle approximation or a grant.
/// Canary47dfd51/Crystal201189f outburst_explode.lua:16-30,
/// eruption_of_destruction_explosion.lua:8-22, gaz'haragoth_death.lua:8-22,
/// tenebris_ultimate.lua:8-24, plagirath_bog.lua:6-20.
pub fn source_callback_area(creature: &str) -> Option<ProjectV2AbilityArea> {
    let rows: &[&str] = match creature {
        "oteryn:creature.charging_outburst"
        | "oteryn:creature.eruption_of_destruction"
        | "oteryn:creature.gaz_haragoth" => &[
            "......x......",
            "....xxxxx....",
            "...xxxxxxx...",
            "..xxxxxxxxx..",
            ".xxxxxxxxxxx.",
            ".xxxxxxxxxxx.",
            "xxxxxxCxxxxxx",
            ".xxxxxxxxxxx.",
            ".xxxxxxxxxxx.",
            "..xxxxxxxxx..",
            "...xxxxxxx...",
            "....xxxxx....",
            "......x......",
        ],
        "oteryn:creature.lady_tenebris" => &[
            ".......x.......",
            "......xxx......",
            ".....xxxxx.....",
            "....xxxxxxx....",
            "...xxxxxxxxx...",
            "..xxxxxxxxxxx..",
            ".xxxxxxxxxxxxx.",
            "xxxxxxxCxxxxxxx",
            ".xxxxxxxxxxxxx.",
            "..xxxxxxxxxxx..",
            "...xxxxxxxxx...",
            "....xxxxxxx....",
            ".....xxxxx.....",
            "......xxx......",
            ".......x.......",
        ],
        "oteryn:creature.plagirath" => &[
            ".............",
            ".............",
            "......x......",
            ".....xxx.....",
            "....xxxxx....",
            "...xxxxxxx...",
            "..xxxxCxxxx..",
            "...xxxxxxx...",
            "....xxxxx....",
            ".....xxx.....",
            "......x......",
            ".............",
            ".............",
        ],
        _ => return None,
    };
    Some(ProjectV2AbilityArea::Matrix {
        north: rows.iter().map(|row| (*row).to_owned()).collect(),
        diagonal: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_opaque_callback_stage_keeps_exact_gaz_damage_and_plagirath_matrix()
    -> Result<(), Box<dyn std::error::Error>> {
        let w = source().map_err(|e| format!("{e:?}"))?;
        let gaz = w
            .cases
            .get("oteryn:ability.spell.gaz_haragoth_death")
            .ok_or("actual Gaz callback")?;
        let spec = parse_stage(w, gaz, &gaz.casts[0]).map_err(|e| format!("{e:?}"))?;
        assert_eq!((spec.minimum, spec.maximum), (30000, 30000));
        assert_eq!(spec.element, "energy");
        let plag = w
            .cases
            .get("oteryn:ability.spell.plagirath_bog")
            .ok_or("actual Plagirath callback")?;
        let spec = parse_stage(w, plag, &plag.casts[0]).map_err(|e| format!("{e:?}"))?;
        let Some(ProjectV2AbilityArea::Matrix { north, .. }) = spec.area else {
            return Err("actual source matrix".into());
        };
        assert_eq!(
            north
                .iter()
                .map(|r| r.bytes().filter(|b| matches!(*b, b'x' | b'C')).count())
                .sum::<usize>(),
            41
        );
        Ok(())
    }
    #[test]
    fn srx3_opaque_callback_stage_compiles_all_nine_existing_source_cases()
    -> Result<(), Box<dyn std::error::Error>> {
        let w = source().map_err(|e| format!("{e:?}"))?;
        assert_eq!(w.cases.len(), 9);
        for c in w.cases.values() {
            for stage in &c.casts {
                parse_stage(w, c, stage).map_err(|e| format!("{}:{e:?}", c.caster))?;
            }
        }
        Ok(())
    }
}
