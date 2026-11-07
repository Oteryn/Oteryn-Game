//! SRV6 exploration title consumer for WORLD-SRC's existing canonical progress owner.
//! Crystal 201189f discovery_system.lua:527-538 counts completed main areas,
//! not individual POIs or subareas. Unconverted legacy progress cannot grant this title.
use crate::domain::CharacterId;
use crate::durability::DurabilityError;
use std::collections::BTreeMap;

pub(super) struct Source {
    mains: BTreeMap<u32, Vec<u32>>,
}
impl Source {
    /// The transport supplies the current admitted bundle's table, never client data.
    pub(super) fn new(table: &oteryn_world_bundle::subarea::Table) -> Option<Self> {
        let mut mains: BTreeMap<_, Vec<_>> = table
            .areas
            .iter()
            .filter(|area| area.parent.is_none())
            .map(|area| (area.id, Vec::new()))
            .collect();
        if mains.is_empty() || table.areas.len() > oteryn_world_bundle::subarea::MAX_AREAS {
            return None;
        }
        for area in &table.areas {
            if let Some(parent) = area.parent {
                mains.get_mut(&parent)?.push(area.id);
            }
        }
        Some(Self { mains })
    }

    fn percent(&self, progress: &serde_json::Value) -> u8 {
        let Some(subareas) = progress
            .get("subareas")
            .and_then(serde_json::Value::as_object)
        else {
            return 0;
        };
        let done = self
            .mains
            .values()
            .filter(|ids| {
                !ids.is_empty()
                    && ids.iter().all(|id| {
                        subareas
                            .get(&id.to_string())
                            .and_then(|state| state.get("discovered"))
                            .and_then(serde_json::Value::as_bool)
                            == Some(true)
                    })
            })
            .count();
        u8::try_from(done * 100 / self.mains.len()).unwrap_or(0)
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
            mains: BTreeMap::from([
                (1, vec![11, 12]),
                (2, vec![21]),
                (3, vec![31]),
                (4, vec![41]),
            ]),
        };
        assert_eq!(source.percent(&serde_json::json!({"discovered":true})), 0);
        assert_eq!(source.percent(&serde_json::json!({"subareas":{"11":{"discovered":true},"21":{"discovered":true},"999":{"discovered":true}}})),25);
        assert_eq!(source.percent(&serde_json::json!({"subareas":{"11":{"discovered":true},"12":{"discovered":true},"21":{"discovered":true}}})),50);
        assert_eq!(source.percent(&serde_json::json!({"subareas":{"11":{"discovered":true},"12":{"discovered":true},"21":{"discovered":true},"31":{"discovered":true},"41":{"discovered":true}}})),100);
    }
}
