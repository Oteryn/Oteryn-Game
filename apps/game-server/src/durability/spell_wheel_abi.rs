//! Source-qualified Wheel allocation gates. Caller artifact bytes and the active Content pin
//! are required; the embedded reference validates provenance and never activates a profile.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFile {
    path: String,
    sha256: String,
    git_blob: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Slot {
    slot: u8,
    capacity: u16,
    minimum_available_points: u32,
    colour: String,
    full_neighbours: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Profile {
    schema: String,
    source_revision: String,
    source_files: Vec<SourceFile>,
    points_per_level: u32,
    minimum_level: u32,
    stage_thresholds: [u32; 3],
    revelation_stats: [[i32; 2]; 3],
    slots: Vec<Slot>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompiledWheelProfile {
    source_digest: [u8; 32],
    profile: Profile,
}
impl CompiledWheelProfile {
    pub(crate) fn from_active_artifact(
        bytes: &[u8],
        source_digest: [u8; 32],
    ) -> Result<Self, &'static str> {
        if source_digest == [0; 32] || bytes.is_empty() || bytes.len() > 64 * 1024 {
            return Err("unqualified Wheel artifact");
        }
        let profile: Profile =
            serde_json::from_slice(bytes).map_err(|_| "invalid Wheel profile")?;
        let expected: Profile = serde_json::from_slice(include_bytes!(
            "../../../../tools/content-schema/native-gameplay/wheel-profile.json"
        ))
        .map_err(|_| "invalid Wheel source qualification")?;
        if profile != expected {
            return Err("Wheel source/projection mismatch");
        }
        Ok(Self {
            source_digest,
            profile,
        })
    }
    pub(crate) fn source_digest(&self) -> [u8; 32] {
        self.source_digest
    }
    pub(crate) fn bind_qualified_outer_artifact(
        &mut self,
        digest: [u8; 32],
    ) -> Result<(), &'static str> {
        if digest == [0; 32] {
            return Err("zero Wheel outer pin");
        }
        self.source_digest = digest;
        Ok(())
    }
    /// Unknown independently owned scroll/quest/mod points refuse before allocation. The source
    /// uint16 budget is bounded explicitly; this candidate never wraps a high-level subtraction.
    pub(crate) fn available_points(
        &self,
        level: u32,
        extra_points: Option<u16>,
        mods: Option<u8>,
    ) -> Result<u16, &'static str> {
        if level < self.profile.minimum_level {
            return Err("Wheel minimum level");
        }
        if mods != Some(0) {
            return Err("unqualified nonzero Wheel modifier budget");
        }
        let base = (level - 50)
            .checked_mul(self.profile.points_per_level)
            .ok_or("Wheel point overflow")?;
        let total = base
            .checked_add(u32::from(extra_points.ok_or("unknown Wheel extra points")?))
            .and_then(|v| v.checked_add(u32::from(mods?)))
            .ok_or("unknown/overflow Wheel modifier points")?;
        u16::try_from(total).map_err(|_| "Wheel source budget exceeds uint16")
    }
    /// Simulates the real source priority passes and six retries on immutable local data. Only a
    /// completely successful assignment is emitted; the Game owner commits its result atomically.
    pub(crate) fn plan_assignment(
        &self,
        current: [u16; 36],
        next: [u16; 36],
        budget: u16,
    ) -> Result<[u16; 36], &'static str> {
        for (i, slot) in self.profile.slots.iter().enumerate() {
            if next[i] > slot.capacity || current[i] > slot.capacity {
                return Err("Wheel slot capacity");
            }
        }
        if next.iter().map(|v| u32::from(*v)).sum::<u32>() > u32::from(budget) {
            return Err("Wheel allocation budget");
        }
        let mut output = current;
        let mut pending: Vec<usize> = (0..36).collect();
        pending.sort_by_key(|i| (self.profile.slots[*i].capacity, *i));
        for _ in 0..7 {
            let mut retries = Vec::new();
            for i in pending {
                let slot = &self.profile.slots[i];
                if next[i] > 0
                    && (u32::from(budget) < slot.minimum_available_points
                        || (!slot.full_neighbours.is_empty()
                            && !slot.full_neighbours.iter().any(|p| {
                                let parent = usize::from(*p) - 1;
                                output[parent] == self.profile.slots[parent].capacity
                            })))
                {
                    retries.push(i);
                    continue;
                }
                let prior = output[i];
                output[i] = 0;
                let used = output.iter().map(|v| u32::from(*v)).sum::<u32>();
                if used + u32::from(next[i]) > u32::from(budget) {
                    output[i] = prior;
                    retries.push(i);
                    continue;
                }
                output[i] = next[i];
            }
            if retries.is_empty() {
                return Ok(output);
            }
            pending = retries;
        }
        Err("Wheel disconnected/full-parent gate")
    }
    pub(crate) fn stages(
        &self,
        allocation: &[u16; 36],
        revelation_bonus: Option<[u16; 4]>,
        mods: Option<u8>,
    ) -> Result<BTreeMap<String, u8>, &'static str> {
        let bonus = revelation_bonus.ok_or("unknown Wheel gem revelation points")?;
        let mods = mods.ok_or("unknown Wheel maximum-grade modifiers")?;
        let mut out = BTreeMap::new();
        for (index, colour) in ["green", "red", "purple", "blue"].into_iter().enumerate() {
            let total = self
                .profile
                .slots
                .iter()
                .enumerate()
                .filter(|(_, s)| s.colour == colour)
                .map(|(i, _)| u32::from(allocation[i]))
                .sum::<u32>()
                + u32::from(bonus[index])
                + u32::from(mods);
            let stage = self
                .profile
                .stage_thresholds
                .iter()
                .filter(|t| total >= **t)
                .count() as u8;
            out.insert(colour.into(), stage);
        }
        Ok(out)
    }
    pub(crate) fn flat_stats(
        &self,
        stages: &BTreeMap<String, u8>,
    ) -> Result<(i32, i32), &'static str> {
        let mut out = (0_i32, 0_i32);
        for colour in ["green", "red", "purple", "blue"] {
            let stage = *stages.get(colour).ok_or("missing complete Wheel stage")?;
            if stage > 3 {
                return Err("Wheel stage bound");
            }
            if stage > 0 {
                let stats = self.profile.revelation_stats[usize::from(stage) - 1];
                out.0 += stats[0];
                out.1 += stats[1];
            }
        }
        Ok(out)
    }
    pub(crate) fn spell_stages(
        &self,
        vocation: &str,
        stages: &BTreeMap<String, u8>,
    ) -> Result<BTreeMap<String, u8>, &'static str> {
        let names = match vocation {
            "knight" | "elite_knight" => [
                ("Executioner's Throw", "red"),
                ("Avatar of Steel", "purple"),
                ("Combat Mastery", "blue"),
            ],
            "paladin" | "royal_paladin" => [
                ("Divine Grenade", "red"),
                ("Avatar of Light", "purple"),
                ("Divine Empowerment", "blue"),
            ],
            "druid" | "elder_druid" => [
                ("Blessing of the Grove", "red"),
                ("Avatar of Nature", "purple"),
                ("Twin Burst", "blue"),
            ],
            "sorcerer" | "master_sorcerer" => [
                ("Beam Mastery", "red"),
                ("Avatar of Storm", "purple"),
                ("Lord of Destruction", "blue"),
            ],
            "monk" | "exalted_monk" => [
                ("Spiritual Outburst", "red"),
                ("Avatar of Balance", "purple"),
                ("Ascetic", "blue"),
            ],
            _ => return Err("unsupported Wheel vocation"),
        };
        let mut out = BTreeMap::new();
        for (name, colour) in names {
            out.insert(
                name.into(),
                *stages.get(colour).ok_or("missing Wheel colour")?,
            );
        }
        if let Some(stage) = out.get("Twin Burst").copied() {
            out.insert("Ice Burst".into(), stage);
            out.insert("Terra Burst".into(), stage);
        }
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn profile() -> CompiledWheelProfile {
        CompiledWheelProfile::from_active_artifact(
            include_bytes!("../../../../tools/content-schema/native-gameplay/wheel-profile.json"),
            [1; 32],
        )
        .unwrap()
    }
    #[test]
    fn missing_bonus_and_source_capacity_do_not_gain_points() {
        let p = profile();
        assert!(p.available_points(100, None, Some(0)).is_err());
        assert!(p.available_points(50, Some(0), Some(0)).is_err());
        assert_eq!(p.available_points(100, Some(0), Some(0)).unwrap(), 50);
        let mut slots = [0; 36];
        slots[0] = 200;
        assert!(p.plan_assignment([0; 36], slots, 1000).is_err());
    }
    #[test]
    fn actual_full_parent_path_unlocks_source_threshold() {
        let p = profile();
        let mut slots = [0; 36];
        let roots: Vec<_> = p
            .profile
            .slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.capacity == 50)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(roots.len(), 4);
        slots[roots[0]] = 50;
        let accepted = p.plan_assignment([0; 36], slots, 50).unwrap();
        assert_eq!(accepted, slots);
        let stages = p.stages(&slots, Some([0; 4]), Some(0)).unwrap();
        assert_eq!(p.flat_stats(&stages).unwrap(), (0, 0));
        assert!(p.stages(&slots, None, Some(0)).is_err());
    }
    #[test]
    fn changed_source_bytes_and_zero_pin_refuse() {
        let raw =
            include_bytes!("../../../../tools/content-schema/native-gameplay/wheel-profile.json");
        assert!(CompiledWheelProfile::from_active_artifact(raw, [0; 32]).is_err());
        let mut value: serde_json::Value = serde_json::from_slice(raw).unwrap();
        value["points_per_level"] = 2.into();
        assert!(
            CompiledWheelProfile::from_active_artifact(
                &serde_json::to_vec(&value).unwrap(),
                [1; 32]
            )
            .is_err()
        );
    }
}
