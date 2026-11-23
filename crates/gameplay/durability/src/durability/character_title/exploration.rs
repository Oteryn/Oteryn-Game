//! SRV6 exploration title consumer for WORLD-SRC's existing canonical progress owner.
//! Crystal 201189f discovery_system.lua:527-538 counts completed main areas,
//! not individual POIs or subareas. Unconverted legacy progress cannot grant this title.
use crate::domain::CharacterId;
use crate::durability::DurabilityError;
use crate::durability::character_cyclopedia::DiscoveryRoster;

pub(super) struct Source {
    roster: DiscoveryRoster,
}
impl Source {
    /// The transport supplies the completion roster from the admitted bundle.
    pub(super) fn new(roster: DiscoveryRoster) -> Option<Self> {
        (!roster.is_empty() && roster.len() <= oteryn_world_bundle::subarea::MAX_AREAS)
            .then_some(Self { roster })
    }

    fn percent(&self, progress: &serde_json::Value) -> u8 {
        crate::durability::character_cyclopedia::canonical_discovery_percent(&self.roster, progress)
    }

    pub(super) async fn read(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        character: CharacterId,
    ) -> Result<u8, DurabilityError> {
        // The Character primary key bounds this read to one existing owner projection.
        let row: Option<(Vec<u8>, String)>=sqlx::query_as("SELECT catalogue_digest,progress::text FROM game_character_cyclopedia WHERE character_id=encode($1,'hex')::uuid")
            .bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        let Some((catalogue, text)) = row else {
            return Ok(0);
        };
        if !crate::domain::cyclopedia::BasicArea::reads_content_digest(&catalogue)
            || text.len() > 262_144
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        // Match WORLD-SRC storage::load_all: completed canonical ids survive geometry
        // revisions and resets; only unfinished exploration is discarded by that owner.
        let progress: serde_json::Value =
            serde_json::from_str(&text).map_err(|_| DurabilityError::InvalidStoredState)?;
        Ok(self.percent(&progress))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srv6_exploration_counts_complete_main_areas_not_pois_or_legacy() {
        let source = Source {
            roster: [(1, 11), (1, 12), (2, 21), (3, 31), (4, 41)].into(),
        };
        assert_eq!(source.percent(&serde_json::json!({"discovered":true})), 0);
        assert_eq!(source.percent(&serde_json::json!({"subareas":{"11":{"discovered":true},"21":{"discovered":true},"999":{"discovered":true}}})),25);
        assert_eq!(source.percent(&serde_json::json!({"subareas":{"11":{"discovered":true},"12":{"discovered":true},"21":{"discovered":true}}})),50);
        assert_eq!(source.percent(&serde_json::json!({"subareas":{"11":{"discovered":true},"12":{"discovered":true},"21":{"discovered":true},"31":{"discovered":true},"41":{"discovered":true}}})),100);
    }
}
