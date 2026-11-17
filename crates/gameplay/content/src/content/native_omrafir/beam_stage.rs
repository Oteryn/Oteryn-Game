//! Immutable source data derived only from the exact native Omrafir overlay.
use super::*;
use sha2::{Digest, Sha256};
#[derive(Debug, Clone)]
pub struct QualifiedOmrafirBeamStage {
    content: [u8; 32],
    creature: ProjectV2DefinitionRef,
    schedule: super::super::ProjectV2AbilitySchedule,
    details: super::super::ProjectV2AbilityDetails,
    direction: u8,
    minimum: u32,
    maximum: u32,
    delay_ms: u32,
    fingerprint: [u8; 32],
}
impl QualifiedOmrafirBeamStage {
    pub fn content(&self) -> [u8; 32] {
        self.content
    }
    pub fn creature(&self) -> &ProjectV2DefinitionRef {
        &self.creature
    }
    pub fn ability(&self) -> &ProjectV2DefinitionRef {
        &self.schedule.ability
    }
    pub fn entry_index(&self) -> usize {
        2
    }
    pub fn parent_schedule(&self) -> &super::super::ProjectV2AbilitySchedule {
        &self.schedule
    }
    pub fn details(&self) -> &super::super::ProjectV2AbilityDetails {
        &self.details
    }
    pub fn source_direction(&self) -> u8 {
        self.direction
    }
    pub fn minimum(&self) -> u32 {
        self.minimum
    }
    pub fn maximum(&self) -> u32 {
        self.maximum
    }
    pub fn delay_ms(&self) -> u32 {
        self.delay_ms
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}
impl QualifiedOmrafirSourceAttacks {
    /// The Lua callback selects one literal matrix from its CURRENT direction at
    /// firing time. It does not rotate that chosen matrix a second time.
    pub fn beam_stage(&self, direction: u8) -> Option<QualifiedOmrafirBeamStage> {
        let source = source_witness()?;
        let schedule = self.behavior.attacks.get(2)?;
        if self.kind(&schedule.ability) != Some(OmrafirSourceCallbackKind::Beam) {
            return None;
        }
        let mut details = self
            .abilities
            .get(&schedule.ability)?
            .details
            .as_deref()?
            .clone();
        let north = source.beam_matrices.get(usize::from(direction))?.clone();
        details.area = Some(super::super::ProjectV2AbilityArea::Matrix {
            north,
            diagonal: Vec::new(),
        });
        details.needs_target = false;
        details.needs_direction = false;
        details.range_tiles = 0;
        details.impact_cue = Some("canary.appearance:effect/firearea".into());
        let fingerprint: [u8; 32] = Sha256::new()
            .chain_update(b"oteryn:source-omrafir-beam-stage:v1")
            .chain_update(self.content)
            .chain_update(source.source_definitions_sha256.as_bytes())
            .chain_update(serde_json::to_vec(&(schedule, &details, direction)).ok()?)
            .finalize()
            .into();
        Some(QualifiedOmrafirBeamStage {
            content: self.content,
            creature: source.native.profile.target.clone(),
            schedule: schedule.clone(),
            details,
            direction,
            minimum: source.beam_damage.minimum,
            maximum: source.beam_damage.maximum,
            delay_ms: source.beam_delay_ms,
            fingerprint,
        })
    }
}
