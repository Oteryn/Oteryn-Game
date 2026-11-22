//! SRV6 exploration title consumer for WORLD-SRC's existing canonical progress owner.
//! Crystal 201189f discovery_system.lua:527-538 counts completed main areas,
//! not individual POIs or subareas. Unconverted legacy progress cannot grant this title.
use crate::domain::CharacterId;
use crate::durability::DurabilityError;
use crate::durability::character_cyclopedia::DiscoveryAreas;

pub(super) struct Source(DiscoveryAreas);

impl Source {
    /// The transport supplies the current admitted bundle's table.
    pub(super) fn new(table: &oteryn_world_bundle::subarea::Table) -> Option<Self> {
        DiscoveryAreas::from_table(table).map(Self)
    }

    fn percent(&self, progress: &serde_json::Value) -> u8 {
        self.0.completion(|id| {
            progress
                .get("subareas")
                .and_then(|subareas| subareas.get(id.to_string()))
                .and_then(|state| state.get("discovered"))
                .and_then(serde_json::Value::as_bool)
                == Some(true)
        }).1
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
    fn srv6_exploration_counts_complete_main_areas_not_pois_or_legacy() -> Result<(), &'static str> {
        use oteryn_world_bundle::subarea::{Area, Table};
        let table = Table {
            areas: [(1, None), (2, None), (3, None), (4, None), (11, Some(1)),
                     (12, Some(1)), (21, Some(2)), (31, Some(3)), (41, Some(4))]
                .into_iter()
                .map(|(id, parent)| Area {
                    id,
                    key: format!("oteryn:content.area.fixture{id}"),
                    name: format!("Area {id}"),
                    parent,
                    source_record_sha256: "a".repeat(64),
                    town_id: None,
                    hometown: false,
                    temple: None,
                }).collect(),
            ..Table::default()
        };
        let source = Source::new(&table).ok_or("invalid exploration source")?;
        assert_eq!(source.percent(&serde_json::json!({"discovered":true})), 0);
        assert_eq!(source.percent(&serde_json::json!({"subareas":{"11":{"discovered":true},"21":{"discovered":true},"999":{"discovered":true}}})),25);
        assert_eq!(source.percent(&serde_json::json!({"subareas":{"11":{"discovered":true},"12":{"discovered":true},"21":{"discovered":true}}})),50);
        assert_eq!(source.percent(&serde_json::json!({"subareas":{"11":{"discovered":true},"12":{"discovered":true},"21":{"discovered":true},"31":{"discovered":true},"41":{"discovered":true}}})),100);
        Ok(())
    }
}
