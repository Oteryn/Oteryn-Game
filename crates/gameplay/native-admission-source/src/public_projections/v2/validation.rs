//! Closed-schema validation for the four revision 3 character envelopes.
use super::*;
use std::collections::BTreeMap;

fn raw_object(raw: &[u8], maximum: usize) -> Result<(), SourceError> {
    if raw.is_empty() || raw.len() > maximum {
        return Err(SourceError::CapacityExceeded);
    }
    let mut depth = 0_u8;
    let mut quoted = false;
    for byte in raw {
        match byte {
            b'\\' => return Err(SourceError::InvalidInput),
            b'"' => quoted = !quoted,
            b'{' | b'[' if !quoted => {
                depth = depth.checked_add(1).ok_or(SourceError::InvalidInput)?;
                if depth > 5 {
                    return Err(SourceError::InvalidInput);
                }
            }
            b'}' | b']' if !quoted => {
                depth = depth.checked_sub(1).ok_or(SourceError::InvalidInput)?;
            }
            _ => continue,
        }
    }
    if quoted || depth != 0 || raw.first() != Some(&b'{') || raw.last() != Some(&b'}') {
        return Err(SourceError::InvalidInput);
    }
    Ok(())
}
#[derive(Deserialize)]
struct Envelope {
    contract_version: u8,
    operation: String,
    source_authority: String,
    character_id: String,
    projection_epoch: String,
    projection_revision: String,
    source_observed_at: String,
    #[serde(flatten)]
    body: BTreeMap<String, serde_json::Value>,
}
pub fn decode_character_snapshot(raw: &[u8]) -> Result<Snapshot, SourceError> {
    raw_object(
        raw,
        Operation::PublishCharacterAchievementsV2.request_bytes_max(),
    )?;
    let mut e: Envelope = serde_json::from_slice(raw).map_err(|_| SourceError::InvalidInput)?;
    if e.contract_version != 2 || e.body.len() != 1 {
        return Err(SourceError::InvalidInput);
    }
    let (field, value) = e.body.pop_first().ok_or(SourceError::InvalidInput)?;
    let invalid = |_| SourceError::InvalidInput;
    let body = match (e.operation.as_str(), field.as_str()) {
        ("PublishCharacterSkillsV2", "skills") => {
            Body::Skills(serde_json::from_value(value).map_err(invalid)?)
        }
        ("PublishCharacterAchievementsV2", "achievements") => {
            Body::Achievements(serde_json::from_value(value).map_err(invalid)?)
        }
        ("PublishCharacterEquipmentV2", "equipment") => {
            Body::Equipment(serde_json::from_value(value).map_err(invalid)?)
        }
        ("PublishCharacterOutfitV2", "outfit") => {
            Body::Outfit(serde_json::from_value(value).map_err(invalid)?)
        }
        _ => return Err(SourceError::InvalidInput),
    };
    let snapshot = Snapshot {
        subject: e.character_id,
        world_id: String::new(),
        projection_epoch: e
            .projection_epoch
            .parse()
            .map_err(|_| SourceError::InvalidInput)?,
        projection_revision: e
            .projection_revision
            .parse()
            .map_err(|_| SourceError::InvalidInput)?,
        source_observed_at: e
            .source_observed_at
            .parse()
            .map_err(|_| SourceError::InvalidInput)?,
        body,
    };
    if encode_wire(&e.source_authority, &snapshot)?.as_bytes() != raw {
        return Err(SourceError::InvalidInput);
    }
    Ok(snapshot)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckedWatermark {
    contract_version: u8,
    operation: String,
    source_authority: String,
    family: WatermarkFamily,
    projection_epoch: String,
    complete_through: String,
    observed_at: String,
}
pub fn decode_watermark(raw: &[u8]) -> Result<WatermarkFamily, SourceError> {
    raw_object(raw, 512)?;
    let w: CheckedWatermark = serde_json::from_slice(raw).map_err(|_| SourceError::InvalidInput)?;
    if w.contract_version != 2
        || w.operation != "PublishPublicProjectionWatermarkV2"
        || !source_authority_valid(&w.source_authority)
        || !number(&w.projection_epoch, false, u64::MAX)
        || !number(&w.complete_through, true, i64::MAX as u64)
        || !number(&w.observed_at, true, i64::MAX as u64)
        || w.complete_through.parse::<u64>().ok() > w.observed_at.parse::<u64>().ok()
        || serde_json::to_vec(&w).map_err(|_| SourceError::InvalidInput)? != raw
    {
        return Err(SourceError::InvalidInput);
    }
    Ok(w.family)
}
/// The mTLS boundary independently enforces each new family flag.
pub(crate) fn publication_supported(
    operation: Operation,
    body: &str,
    activation: Activation,
) -> bool {
    let family = match operation {
        Operation::PublishCharacterSkillsV2 => Some(Family::CharacterSkills),
        Operation::PublishCharacterAchievementsV2 => Some(Family::CharacterAchievements),
        Operation::PublishCharacterEquipmentV2 => Some(Family::CharacterEquipment),
        Operation::PublishCharacterOutfitV2 => Some(Family::CharacterOutfit),
        Operation::PublishPublicProjectionWatermarkV2 => {
            return match decode_watermark(body.as_bytes()) {
                Ok(WatermarkFamily::CharacterSkills) => activation.character_skills,
                Ok(WatermarkFamily::CharacterAchievements) => activation.character_achievements,
                Ok(WatermarkFamily::CharacterEquipment) => activation.character_equipment,
                Ok(WatermarkFamily::CharacterOutfit) => activation.character_outfit,
                Ok(
                    WatermarkFamily::CharacterDeaths
                    | WatermarkFamily::Guild
                    | WatermarkFamily::House
                    | WatermarkFamily::CharacterOnlineSessions
                    | WatermarkFamily::WorldDailyKills,
                ) => true,
                _ => false,
            };
        }
        _ => None,
    };
    family.is_none_or(|f| f.publishable(activation))
}
