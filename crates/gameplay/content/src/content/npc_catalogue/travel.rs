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
    fn paid_travel_routes_preserve_source_corrections_and_qualified_global_exceptions()
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
        assert_eq!(corrections.len(), 196);
        let keys = corrections
            .iter()
            .map(|row| &row.key)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(keys.len(), 196);
        let sources = source["rows"].as_array().ok_or("source rows")?;
        let evidence = evidence["records"].as_array().ok_or("evidence rows")?;
        assert_eq!(sources.len() + 1, corrections.len());
        assert_eq!(evidence.len() + 1, corrections.len());
        for ((row, source), evidence) in corrections
            .iter()
            .filter(|row| !row.key.ends_with("captain_seagull/yalahar"))
            .zip(sources)
            .zip(evidence)
        {
            assert_eq!(serde_json::json!(row.key), source["key"]);
            assert_eq!(serde_json::json!(row.keyword), source["keyword"]);
            assert_eq!(serde_json::json!(row.price), source["selected_fare"]);
            let seagull_free_route = matches!(
                row.key.as_str(),
                "oteryn:service.travel.captain_seagull/edron"
                    | "oteryn:service.travel.captain_seagull/gray_island"
            );
            if seagull_free_route {
                assert!(!row.premium);
            } else {
                assert_eq!(serde_json::json!(row.premium), source["selected_premium"]);
            }
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
                assert!(
                    row.held || row.key == "oteryn:service.travel.kendra/thais",
                    "{}",
                    row.key
                );
            }
        }
        Ok(())
    }

    // Crystal 201189f8 data-global/npc/captain_seagull.lua:64-69 and kendra.lua:70, read as
    // reference data (Canary 47dfd51f identical). Crystal sails every Seagull route without a
    // premium gate and gates Yalahar on quest storage. Oteryn qualifies those same rules.
    #[test]
    fn crystal_seagull_and_kendra_fares_and_arrivals() -> Result<(), Box<dyn std::error::Error>> {
        let corrections: Vec<super::Correction> =
            serde_json::from_str(include_str!("travel_routes.json"))?;
        let find = |key: &str| {
            corrections
                .iter()
                .find(|row| row.key == format!("oteryn:service.travel.{key}"))
        };
        let mut premium_diffs = Vec::new();
        for (key, price, crystal_premium, oteryn_premium, x, y, floor) in [
            (
                "captain_seagull/thais",
                130,
                false,
                false,
                32_310,
                32_210,
                6,
            ),
            (
                "captain_seagull/carlin",
                80,
                false,
                false,
                32_387,
                31_820,
                6,
            ),
            (
                "captain_seagull/gray_island",
                150,
                false,
                false,
                33_196,
                31_984,
                7,
            ),
            ("captain_seagull/edron", 70, false, false, 33_175, 31_764, 6),
            (
                "captain_seagull/venore",
                90,
                false,
                false,
                32_954,
                32_022,
                6,
            ),
            ("kendra/thais", 180, true, true, 32_259, 32_262, 7),
        ] {
            let row = find(key).ok_or(key)?;
            assert_eq!(
                (row.price, row.premium, row.x, row.y, row.floor, row.postman),
                (price, oteryn_premium, x, y, floor, true),
                "{key}"
            );
            if row.premium != crystal_premium {
                premium_diffs.push(key);
            }
        }
        assert!(premium_diffs.is_empty());
        let yalahar = find("captain_seagull/yalahar").ok_or("yalahar")?;
        assert_eq!(
            (
                yalahar.price,
                yalahar.premium,
                yalahar.x,
                yalahar.y,
                yalahar.floor
            ),
            (160, false, 32_816, 31_272, 6)
        );
        assert!(!find("kendra/thais").ok_or("kendra")?.held);
        Ok(())
    }
}
