//! Paid travel corrections and qualified additions to admitted captain services.
//! `travel_routes.json` projects source-correction-check and admission-evidence in
//! docs/reference/content-prep/travel-content-1, plus native S10S/S10T captain additions.
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
    pub yalahar_town_gate: Option<String>,
    pub quest_gate: Option<String>,
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
    #[serde(default)]
    yalahar_town_gate: Option<String>,
    #[serde(default)]
    native_addition: bool,
    #[serde(default)]
    quest_gate: Option<String>,
}

pub fn resolve(
    model: &NpcServiceModel,
    npc_key: &str,
    services: &[ProjectV2DefinitionRef],
) -> Vec<TravelRoute> {
    static CORRECTIONS: OnceLock<Vec<Correction>> = OnceLock::new();
    let corrections = CORRECTIONS.get_or_init(|| {
        serde_json::from_str(include_str!("travel_routes.json")).unwrap_or_default()
    });
    let native_service = format!(
        "oteryn:service.travel.{}",
        npc_key.trim_start_matches("oteryn:npc.")
    );
    corrections
        .iter()
        .filter(|correction| {
            services
                .iter()
                .any(|service| correction.key.starts_with(&format!("{}/", service.key)))
                || (correction.native_addition
                    && correction.key.starts_with(&format!("{native_service}/")))
        })
        .filter_map(|correction| {
            let (service, key) = correction.key.rsplit_once('/')?;
            let template = services
                .iter()
                .find(|reference| reference.key == service)
                .and_then(|reference| model.services().get(reference))
                .and_then(|entry| {
                    entry
                        .routes
                        .iter()
                        .find(|route| route.key == key)
                        .or_else(|| entry.routes.first().filter(|_| correction.native_addition))
                });
            if template.is_none() && !correction.native_addition {
                return None;
            }
            let mut route = template.cloned().unwrap_or(ProjectV2TravelRoute {
                key: key.to_owned(),
                destination: crate::content::ProjectV2TravelDestination {
                    coordinate_frame: "global-target-2026-09-27".to_owned(),
                    x: correction.x,
                    y: correction.y,
                    floor: correction.floor,
                },
                price: correction.price,
                premium: correction.premium,
                min_level: None,
            });
            route.key = key.to_owned();
            route.price = correction.price;
            route.premium = correction.premium;
            route.min_level = None;
            route.destination.x = correction.x;
            route.destination.y = correction.y;
            route.destination.floor = correction.floor;
            Some(TravelRoute {
                service: service.to_owned(),
                route,
                keyword: correction.keyword.clone(),
                postman: correction.postman,
                carpet: correction.carpet,
                held: correction.held,
                yalahar_gate: correction.yalahar_gate,
                yalahar_town_gate: correction.yalahar_town_gate.clone(),
                quest_gate: correction.quest_gate.clone(),
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
        assert_eq!(corrections.len(), 227);
        let keys = corrections
            .iter()
            .map(|row| &row.key)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(keys.len(), 227);
        let sources = source["rows"].as_array().ok_or("source rows")?;
        let evidence = evidence["records"].as_array().ok_or("evidence rows")?;
        let corrected = corrections
            .iter()
            .filter(|row| !row.native_addition)
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
        let routes = super::resolve(&model, "oteryn:npc.captain_seagull", &seagull.services);
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

    // Crystal 201189f8: captain_bluebear.lua:70,83,88; captain_fearless.lua:79;
    // captain_greyhound.lua:69; captain_max.lua:77-78; captain_sinbeard.lua:101;
    // captain_seahorse.lua:60,74; charles.lua:120-129; jack_fate.lua:81-82;
    // karith.lua:96-200,247-325.
    #[test]
    fn crystal_remaining_captain_routes_resolve_through_native_overlay()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalogue = crate::content::NpcDataCatalogue::load(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content"),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )?;
        let model = catalogue.service_model_for_project()?;
        for (npc, route, price, destination) in [
            ("captain_bluebear", "yalahar", 200, (32_816, 31_272, 6)),
            ("captain_fearless", "yalahar", 185, (32_816, 31_272, 6)),
            ("captain_greyhound", "yalahar", 185, (32_816, 31_272, 6)),
            ("captain_max", "calassa", 200, (31_911, 32_710, 6)),
            ("captain_sinbeard", "yalahar", 230, (32_816, 31_272, 6)),
            ("captain_seahorse", "oramond", 200, (33_479, 31_985, 7)),
            ("charles", "liberty_bay", 50, (32_285, 32_892, 6)),
            ("jack_fate", "goroma", 0, (32_161, 32_558, 6)),
            ("karith", "port_hope", 260, (32_527, 32_784, 6)),
            ("captain_waverider", "peg leg", 50, (32_346, 32_625, 7)),
            ("captain_waverider", "passage", 200, (32_131, 32_913, 7)),
            ("captain_jack_rat", "safe", 0, (33_839, 31_222, 5)),
        ] {
            let key = format!("oteryn:npc.{npc}");
            let entry = model
                .npcs()
                .iter()
                .find(|(reference, _)| reference.key == key)
                .map(|(_, npc)| npc)
                .ok_or(key)?;
            let found = super::resolve(&model, &format!("oteryn:npc.{npc}"), &entry.services)
                .into_iter()
                .find(|candidate| candidate.keyword == route)
                .ok_or(route)?;
            assert_eq!(found.route.price, price);
            assert_eq!(
                found.route.premium,
                !matches!(npc, "captain_jack_rat" | "captain_waverider")
            );
            assert_eq!(
                (
                    found.route.destination.x,
                    found.route.destination.y,
                    found.route.destination.floor
                ),
                destination
            );
        }
        Ok(())
    }
    #[test]
    fn remaining_captain_routes_preserve_unlocks_and_custom_fares()
    -> Result<(), Box<dyn std::error::Error>> {
        let rows: Vec<super::Correction> =
            serde_json::from_str(include_str!("travel_routes.json"))?;
        for (key, track) in [
            (
                "captain_max/calassa",
                "oteryn:quest-progress/quest/u7_6/explorer_society/calassa_quest",
            ),
            (
                "jack_fate/goroma",
                "oteryn:quest-progress/quest/u7_8/the_shattered_isles/access_to_goroma",
            ),
            (
                "captain_waverider/meriana",
                "oteryn:quest-progress/quest/u7_8/the_shattered_isles/access_to_meriana",
            ),
            (
                "captain_jack_rat/safe",
                "oteryn:quest-progress/quest/u12_60/apirates_tail/tentugly_killed",
            ),
        ] {
            let row = rows
                .iter()
                .find(|row| row.key == format!("oteryn:service.travel.{key}"))
                .ok_or(key)?;
            assert_eq!(row.quest_gate.as_deref(), Some(track));
            assert!(!row.held);
        }
        for row in rows
            .iter()
            .filter(|row| row.key.starts_with("oteryn:service.travel.karith/"))
        {
            assert!(!row.postman);
            assert!(row.yalahar_town_gate.is_some());
        }
        // These donor routes cannot be safely offered with the currently admitted owners.
        for key in [
            "captain_max/yalahar",
            "captain_bluebear/targuna",
            "captain_bluebear/carlin",
            "captain_seahorse/venore",
        ] {
            assert!(
                rows.iter()
                    .find(|row| row.key == format!("oteryn:service.travel.{key}"))
                    .ok_or(key)?
                    .held
            );
        }
        Ok(())
    }
}
