//! Paid travel corrections and qualified additions to admitted captain services.
//! `travel_routes.json` projects source-correction-check and admission-evidence in
//! docs/reference/content-prep/travel-content-1, plus the native S10S Yalahar addition.
//! Unmapped custom/quest guards stay held.
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
    pub yalahar_gate: bool,
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
    #[serde(default)]
    yalahar_gate: bool,
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
            let prefix = format!("{}/", service.key);
            corrections.iter().filter_map(move |correction| {
                let key = correction.key.strip_prefix(&prefix)?;
                let mut route = entry
                    .routes
                    .iter()
                    .find(|route| route.key == key)
                    .or_else(|| {
                        // S10S adds the qualified native route without modifying generated
                        // legacy membership. It shares this captain's coordinate frame.
                        if correction.yalahar_gate {
                            entry.routes.first()
                        } else {
                            None
                        }
                    })?
                    .clone();
                route.key = key.to_owned();
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
                    yalahar_gate: correction.yalahar_gate,
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
        assert_eq!(corrections.len(), 196);
        let keys = corrections
            .iter()
            .map(|row| &row.key)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(keys.len(), 196);
        let sources = source["rows"].as_array().ok_or("source rows")?;
        let evidence = evidence["records"].as_array().ok_or("evidence rows")?;
        let corrected = corrections
            .iter()
            .filter(|row| row.key != "oteryn:service.travel.captain_seagull/yalahar")
            .collect::<Vec<_>>();
        assert_eq!(corrected.len(), 195);
        assert_eq!(sources.len(), 195);
        assert_eq!(evidence.len(), 195);
        for ((row, source), evidence) in corrected.into_iter().zip(sources).zip(evidence) {
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

    // Crystal 201189f8 data-global/npc/captain_seagull.lua:64-69 and kendra.lua:70, read as
    // reference data (Canary 47dfd51f identical). Crystal sails every Seagull route without a
    // premium gate and gates Yalahar on a quest storage; Oteryn keeps the Edron and Gray Island
    // premium flags from its admitted correction and adds the gated Yalahar route.
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
                true,
                33_196,
                31_984,
                7,
            ),
            ("captain_seagull/edron", 70, false, true, 33_175, 31_764, 6),
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
        assert_eq!(
            premium_diffs,
            ["captain_seagull/gray_island", "captain_seagull/edron"]
        );
        let yalahar = find("captain_seagull/yalahar").ok_or("captain_seagull/yalahar")?;
        assert_eq!(
            (
                yalahar.price,
                yalahar.premium,
                yalahar.x,
                yalahar.y,
                yalahar.floor,
                yalahar.postman,
                yalahar.held,
                yalahar.yalahar_gate
            ),
            (160, false, 32_816, 31_272, 6, true, false, true)
        );
        let catalogue = crate::content::NpcDataCatalogue::load(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content"),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )?;
        let model = catalogue.service_model_for_project()?;
        let seagull = model
            .npcs()
            .iter()
            .find(|(reference, _)| reference.key == "oteryn:npc.captain_seagull")
            .map(|(_, npc)| npc)
            .ok_or("Captain Seagull")?;
        let routes = super::resolve(&model, &seagull.services);
        let route = routes
            .iter()
            .find(|route| route.keyword == "yalahar")
            .ok_or("resolved Yalahar route")?;
        assert_eq!(route.route.price, 160);
        assert_eq!(
            (
                route.route.destination.x,
                route.route.destination.y,
                route.route.destination.floor
            ),
            (32_816, 31_272, 6)
        );
        assert!(route.yalahar_gate);
        assert!(!route.held);
        Ok(())
    }
}
