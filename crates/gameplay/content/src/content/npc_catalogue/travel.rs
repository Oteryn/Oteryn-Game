//! TRAVEL-CONTENT-1 corrections over the NPC's existing admitted route membership.
//! `travel_routes.json` projects source-correction-check and admission-evidence in
//! docs/reference/content-prep/travel-content-1. Unmapped custom/quest guards stay held.
use super::NpcServiceModel;
use crate::content::{ProjectV2DefinitionRef, ProjectV2TravelRoute};
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TravelRoute {
    pub service: String,
    pub route: ProjectV2TravelRoute,
    pub keyword: String,
    pub postman: bool,
    pub carpet: bool,
    pub held: bool,
}

#[derive(Deserialize)]
struct Correction {
    key: String,
    keyword: String,
    price: u64,
    premium: bool,
    x: i32,
    y: i32,
    floor: i16,
    postman: bool,
    carpet: bool,
    held: bool,
}

pub fn resolve(model: &NpcServiceModel, services: &[ProjectV2DefinitionRef]) -> Vec<TravelRoute> {
    static CORRECTIONS: OnceLock<Vec<Correction>> = OnceLock::new();
    let corrections = CORRECTIONS.get_or_init(|| {
        serde_json::from_str(include_str!("travel_routes.json")).unwrap_or_default()
    });
    services
        .iter()
        .filter_map(|service| model.services().get(service).map(|entry| (service, entry)))
        .flat_map(|(service, entry)| {
            entry.routes.iter().filter_map(|route| {
                let key = format!("{}/{}", service.key, route.key);
                let correction = corrections.iter().find(|row| row.key == key)?;
                let mut route = route.clone();
                route.price = correction.price;
                route.premium = correction.premium;
                route.min_level = None;
                route.destination.x = correction.x;
                route.destination.y = correction.y;
                route.destination.floor = correction.floor;
                Some(TravelRoute {
                    service: service.key.clone(),
                    route,
                    keyword: correction.keyword.clone(),
                    postman: correction.postman,
                    carpet: correction.carpet,
                    held: correction.held,
                })
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn paid_travel_routes_preserve_all_195_corrected_fares_premium_and_arrivals()
    -> Result<(), Box<dyn std::error::Error>> {
        let corrections: Vec<super::Correction> =
            serde_json::from_str(include_str!("travel_routes.json"))?;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/reference/content-prep/travel-content-1");
        let source: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("source-correction-check.json"))?)?;
        let evidence: serde_json::Value = serde_json::from_slice(&std::fs::read(
            root.join("admission-evidence/evidence-00000-00194.json"),
        )?)?;
        assert_eq!(corrections.len(), 195);
        let keys = corrections
            .iter()
            .map(|row| &row.key)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(keys.len(), 195);
        let sources = source["rows"].as_array().ok_or("source rows")?;
        let evidence = evidence["records"].as_array().ok_or("evidence rows")?;
        assert_eq!(sources.len(), corrections.len());
        assert_eq!(evidence.len(), corrections.len());
        for ((row, source), evidence) in corrections.iter().zip(sources).zip(evidence) {
            assert_eq!(serde_json::json!(row.key), source["key"]);
            assert_eq!(serde_json::json!(row.keyword), source["keyword"]);
            assert_eq!(serde_json::json!(row.price), source["selected_fare"]);
            assert_eq!(serde_json::json!(row.premium), source["selected_premium"]);
            assert_eq!(serde_json::json!(row.x), source["destination"]["x"]);
            assert_eq!(serde_json::json!(row.y), source["destination"]["y"]);
            assert_eq!(serde_json::json!(row.floor), source["destination"]["z"]);
            let facts = &evidence["facts"];
            assert_eq!(row.carpet, facts["transport"]["value"] == "carpet");
            if row.carpet {
                assert!(row.premium, "{}", row.key);
            }
            if facts["quest_requirements"]["value"] != serde_json::json!([])
                || !facts["travel_guard_conditions"]["value"].is_null()
            {
                assert!(row.held, "{}", row.key);
            }
        }
        Ok(())
    }
}
