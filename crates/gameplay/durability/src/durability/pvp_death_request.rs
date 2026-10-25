//! Classified Open-PvP input. The runtime classifier is a separate, gated mechanic.
use super::character_progression::CharacterProgressionError;
use crate::domain::CharacterId;
use oteryn_gameplay_combat_rules::pvp::skull::MAX_CONTRIBUTORS;
pub use oteryn_gameplay_combat_rules::pvp::skull::PvpContributorKind;

/// Supplied only after Open-PvP classification. Empty means the existing PvE path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PvpDeathContributor {
    pub character_id: CharacterId,
    pub kind: PvpContributorKind,
    pub unjustified: bool,
}

pub(super) fn bind_contributors(
    semantic: &mut Vec<u8>,
    victim: CharacterId,
    contributors: &[PvpDeathContributor],
) -> Result<(), CharacterProgressionError> {
    if contributors.len() > MAX_CONTRIBUTORS
        || contributors.iter().any(|c| c.character_id == victim)
        || !contributors
            .windows(2)
            .all(|pair| pair[0].character_id.as_bytes() < pair[1].character_id.as_bytes())
    {
        return Err(CharacterProgressionError::InvalidInput);
    }
    // Preserve every pre-PvP command binding byte for byte.
    if !contributors.is_empty() {
        semantic.extend_from_slice(b"oteryn:open-pvp-consequences:v1");
        semantic.push(
            u8::try_from(contributors.len())
                .map_err(|_| CharacterProgressionError::InvalidInput)?,
        );
        for contributor in contributors {
            semantic.extend_from_slice(contributor.character_id.as_bytes());
            semantic.push(match contributor.kind {
                PvpContributorKind::Damage => 1,
                PvpContributorKind::Assist => 2,
            });
            semantic.push(u8::from(contributor.unjustified));
        }
    }
    Ok(())
}
