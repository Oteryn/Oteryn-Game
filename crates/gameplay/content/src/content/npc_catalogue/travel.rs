//! Paid travel corrections and qualified additions to admitted captain services.
//! `travel_routes.json` projects source-correction-check and admission-evidence in
//! docs/reference/content-prep/travel-content-1, plus native S10S/S10T captain additions.
//! Crystal 201189f8 NPC scripts supply the Cormaya return, Nostalgia return,
//! Meluna ferry, Vengoth return, Ingol ferries and Oskayaat passage.
//! Unmapped custom/quest guards stay held.
use super::NpcServiceModel;
use crate::content::{
    ProjectV2DefinitionRef, ProjectV2TravelQuestRequirement, ProjectV2TravelRoute,
};
use serde::Deserialize;
use std::sync::OnceLock;

#[path = "travel_return.rs"]
mod return_policy;
pub use return_policy::TravelReturnRequirement;

/// The donor storage advanced by each settled Postman tour passage.
pub const POSTMAN_TOUR_TRACK: &str =
    "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission01";

/// A paid ticket's purchase time is a Quest-owned Unix-second track.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TravelTicketRequirement {
    pub track: String,
    pub valid_for_seconds: u32,
}

/// An exact donor transition whose Quest settlement is pending.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartialTravelQuestCallback {
    pub track: String,
    pub stage: i64,
    pub next_stage: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TravelRoute {
    pub service: String,
    pub route: ProjectV2TravelRoute,
    pub keyword: String,
    /// Use Crystal's MsgContains for the route key and confirmation.
    pub crystal_callback: bool,
    pub postman: bool,
    pub carpet: bool,
    pub held: bool,
    pub yalahar_gate: bool,
    pub yalahar_town_gate: Option<String>,
    pub quest_gate: Option<String>,
    /// Refuse travel unless this track is absent or exactly zero (for example an unpaid fine).
    pub quest_clear_gate: Option<String>,
    pub ticket: Option<TravelTicketRequirement>,
    pub return_origin: Option<TravelReturnRequirement>,
    pub world_changes: Vec<String>,
    pub partial_quest_callback: Option<PartialTravelQuestCallback>,
}

#[derive(Deserialize)]
struct Correction {
    key: String,
    keyword: String,
    #[serde(default)]
    aliases: Vec<String>,
    price: u64,
    premium: bool,
    x: i32,
    y: i32,
    floor: i16,
    premium_override: Option<bool>,
    #[serde(default)]
    use_authored_fare_and_premium: bool,
    #[serde(default)]
    crystal_callback: bool,
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
    quest_tracks: Vec<ProjectV2TravelQuestRequirement>,
    #[serde(default)]
    quest_gate: Option<String>,
    #[serde(default)]
    quest_clear_gate: Option<String>,
    #[serde(default)]
    ticket: Option<TravelTicketRequirement>,
    #[serde(default)]
    return_origin: Option<TravelReturnRequirement>,
    #[serde(default)]
    world_change: Option<String>,
    #[serde(default)]
    world_changes: Vec<String>,
    #[serde(default)]
    partial_quest_callback: Option<PartialTravelQuestCallback>,
}

/// Source-qualified helpers that supersede older held route corrections.
pub const CRYSTAL_HELPERS: &str =
    include_str!("../../../../../../content/services/travel/crystal-helpers.json");

/// Bind route corrections to the same revision as their production conversation book.
pub fn revision_material() -> &'static str {
    include_str!("travel_routes.json")
}

fn corrections() -> &'static [Correction] {
    static CORRECTIONS: OnceLock<Vec<Correction>> = OnceLock::new();
    CORRECTIONS.get_or_init(|| {
        let Ok(mut historical) = serde_json::from_str::<Vec<Correction>>(revision_material())
        else {
            return Vec::new();
        };
        let Ok(helpers) = serde_json::from_str::<Vec<Correction>>(CRYSTAL_HELPERS) else {
            return historical;
        };
        historical.retain(|row| !helpers.iter().any(|helper| helper.key == row.key));
        historical.extend(helpers);
        historical
    })
}

impl TravelRoute {
    pub fn is_transfer_platform(&self) -> bool {
        self.service == "oteryn:service.travel.sharon" && self.route.key == "departure_platform"
    }

    /// The before and after Postman stages for the exact route settled by paid travel.
    pub fn postman_tour_passage(&self) -> Option<(i64, i64)> {
        match (self.service.as_str(), self.route.key.as_str()) {
            ("oteryn:service.travel.captain_bluebear", "carlin") => Some((1, 2)),
            ("oteryn:service.travel.uzon", "edron") => Some((2, 3)),
            ("oteryn:service.travel.captain_seahorse", "venore") => Some((3, 4)),
            ("oteryn:service.travel.brodrosch", "cormaya") => Some((4, 5)),
            _ => None,
        }
    }

    /// Existing routes retain both their spoken destination and their route key.
    pub fn matches_keyword(&self, text: &str) -> bool {
        text == self.keyword
            || text == self.route.key.replace('_', " ")
            || corrections().iter().any(|row| {
                !row.aliases.is_empty()
                    && row.key.split_once('/')
                        == Some((self.service.as_str(), self.route.key.as_str()))
                    && row.aliases.iter().any(|alias| alias == text)
            })
    }
}

/// The complete service and route key persisted by the paid travel owner.
pub fn is_carpet_route(key: &str) -> bool {
    corrections()
        .iter()
        .any(|route| route.key == key && route.carpet && !route.held)
}

pub fn resolve(
    model: &NpcServiceModel,
    npc_key: &str,
    services: &[ProjectV2DefinitionRef],
) -> Vec<TravelRoute> {
    let corrections = corrections();
    let native_service = format!(
        "oteryn:service.travel.{}",
        npc_key.trim_start_matches("oteryn:npc.")
    );
    let corrected = corrections
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
            let (x, y, floor) = (correction.x, correction.y, correction.floor);
            let (price, premium) = template
                .filter(|_| correction.use_authored_fare_and_premium)
                .map_or((correction.price, correction.premium), |route| {
                    (route.price, route.premium)
                });
            let premium = correction.premium_override.unwrap_or(premium);
            let mut route = template.cloned().unwrap_or(ProjectV2TravelRoute {
                key: key.to_owned(),
                destination: crate::content::ProjectV2TravelDestination {
                    coordinate_frame: "global-target-2026-09-27".to_owned(),
                    x,
                    y,
                    floor,
                },
                price,
                premium,
                min_level: None,
                required_quest_tracks: Vec::new(),
            });
            route.key = key.to_owned();
            route.price = price;
            route.premium = premium;
            route.min_level = None;
            route.destination.x = x;
            route.destination.y = y;
            route.destination.floor = floor;
            route
                .required_quest_tracks
                .extend(correction.quest_tracks.iter().cloned());
            Some(TravelRoute {
                service: service.to_owned(),
                route,
                keyword: correction.keyword.clone(),
                crystal_callback: correction.crystal_callback,
                postman: correction.postman,
                carpet: correction.carpet,
                held: correction.held,
                yalahar_gate: correction.yalahar_gate,
                yalahar_town_gate: correction.yalahar_town_gate.clone(),
                quest_gate: correction.quest_gate.clone(),
                quest_clear_gate: correction.quest_clear_gate.clone(),
                ticket: correction.ticket.clone(),
                return_origin: correction.return_origin.clone(),
                world_changes: correction
                    .world_change
                    .iter()
                    .chain(&correction.world_changes)
                    .cloned()
                    .collect(),
                partial_quest_callback: correction.partial_quest_callback.clone(),
            })
        });
    let authored = services
        .iter()
        .filter_map(|reference| {
            model
                .services()
                .get(reference)
                .map(|entry| (reference, entry))
        })
        .flat_map(|(reference, entry)| {
            entry.routes.iter().filter_map(move |route| {
                if route.required_quest_tracks.is_empty() {
                    return None;
                }
                let key = format!("{}/{}", reference.key, route.key);
                if corrections.iter().any(|correction| correction.key == key) {
                    return None;
                }
                Some(TravelRoute {
                    service: reference.key.clone(),
                    route: route.clone(),
                    keyword: route.key.replace('_', " "),
                    crystal_callback: false,
                    postman: false,
                    carpet: false,
                    held: false,
                    yalahar_gate: false,
                    yalahar_town_gate: None,
                    quest_gate: None,
                    quest_clear_gate: None,
                    ticket: None,
                    return_origin: None,
                    world_changes: Vec::new(),
                    partial_quest_callback: None,
                })
            })
        });
    corrected.chain(authored).collect()
}

#[cfg(test)]
mod tests {
    use super::revision_material;

    #[test]
    fn partial_postman_travel_callbacks_preserve_exact_donor_transitions()
    -> Result<(), Box<dyn std::error::Error>> {
        let evidence: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../docs/agents/evidence/quests-2-20261102/the_postman_missions_quest.json"
        ))?;
        let model = production_travel_model()?;
        let mut callbacks = 0;
        for row in evidence["rows"].as_array().ok_or("Postman evidence rows")? {
            let source = &row["source_row"];
            let Some((npc, destination)) = source["trigger"]
                .as_str()
                .and_then(|trigger| trigger.strip_prefix("NPC"))
                .and_then(|trigger| trigger.strip_suffix(",yes"))
                .and_then(|trigger| trigger.split_once(':'))
                .filter(|_| {
                    ["condition", "effect"].iter().all(|key| {
                        source[*key]
                            .as_str()
                            .is_some_and(|v| v.starts_with("Mission01="))
                    })
                })
            else {
                continue;
            };
            let stage = |key: &str| -> Result<i64, Box<dyn std::error::Error>> {
                Ok(source[key]
                    .as_str()
                    .and_then(|text| text.split_once(';'))
                    .and_then(|(condition, _)| condition.strip_prefix("Mission01="))
                    .ok_or("Postman stage")?
                    .parse()?)
            };
            let route = production_route(&model, npc, destination)?.ok_or("Postman route")?;
            let (before, after) = (stage("condition")?, stage("effect")?);
            assert_eq!(
                route.postman_tour_passage(),
                Some((before, after)),
                "{npc}/{destination}"
            );
            let legacy_callback = route
                .partial_quest_callback
                .as_ref()
                .map(|callback| (callback.track.as_str(), callback.stage, callback.next_stage));
            assert_eq!(
                legacy_callback,
                (npc == "captain_bluebear").then_some((super::POSTMAN_TOUR_TRACK, before, after)),
                "{npc}/{destination} legacy receipt metadata"
            );
            let fare = source["condition"]
                .as_str()
                .and_then(|condition| condition.split_once(";travel koszt"))
                .and_then(|(_, fare)| fare.split_once(' '))
                .ok_or("Postman donor fare")?
                .0
                .parse::<u64>()?;
            let cell = &route.route.destination;
            let landing = format!(
                "Mission01={after};podróż do{},{},{}",
                cell.x, cell.y, cell.floor
            );
            assert_eq!(source["effect"], landing, "{npc}/{destination}");
            assert_eq!(route.route.price, fare, "{npc}/{destination}");
            assert!(
                route.postman && !route.held && !route.route.premium,
                "{npc}/{destination}"
            );
            callbacks += 1;
        }
        assert_eq!(callbacks, 4);
        Ok(())
    }

    #[test]
    fn placed_postman_callback_routes_admit_ordinary_passengers()
    -> Result<(), Box<dyn std::error::Error>> {
        let model = production_travel_model()?;
        let mut blocked = Vec::new();
        for (npc, destination) in [
            ("captain_bluebear", "carlin"),
            ("uzon", "edron"),
            ("captain_seahorse", "venore"),
        ] {
            let route = production_route(&model, npc, destination)?.ok_or("production route")?;
            if route.held || !route.route.required_quest_tracks.is_empty() {
                blocked.push((npc, destination));
            }
        }
        assert!(
            blocked.is_empty(),
            "ordinary passengers blocked: {blocked:?}"
        );
        Ok(())
    }

    #[test]
    fn partial_junkar_routes_are_held_and_preserve_world_change_requirements()
    -> Result<(), Box<dyn std::error::Error>> {
        let model = production_travel_model()?;
        let mut missing = Vec::new();
        let mut released = Vec::new();
        for (npc, destinations) in [
            ("junkar_mines", ["kazordoon", "robsons_isle", "thais"]),
            ("junkar_robsons", ["kazordoon", "mines", "thais"]),
            ("junkar_thais", ["kazordoon", "mines", "robsons_isle"]),
        ] {
            for destination in destinations {
                let route =
                    production_route(&model, npc, destination)?.ok_or("production route")?;
                let expected: &[_] = if (npc, destination) == ("junkar_thais", "kazordoon") {
                    &["awash", "steamship"]
                } else {
                    &["awash"]
                };
                if !route.held {
                    released.push((npc, destination));
                }
                if route.world_changes != expected {
                    missing.push((npc, destination, route.world_changes));
                }
            }
        }
        assert!(missing.is_empty(), "world change missing: {missing:?}");
        assert!(
            released.is_empty(),
            "PARTIAL Junkar routes released without their world-change owners: {released:?}"
        );
        Ok(())
    }

    fn production_travel_model() -> Result<super::NpcServiceModel, Box<dyn std::error::Error>> {
        let catalogue = crate::content::NpcDataCatalogue::load(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content"),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )?;
        Ok(catalogue.service_model_for_project()?)
    }

    fn production_route(
        model: &super::NpcServiceModel,
        npc: &str,
        destination: &str,
    ) -> Result<Option<super::TravelRoute>, Box<dyn std::error::Error>> {
        let key = format!("oteryn:npc.{npc}");
        let (_, entry) = model
            .npcs()
            .iter()
            .find(|(reference, _)| reference.key == key)
            .ok_or_else(|| format!("missing NPC {key}"))?;
        Ok(super::resolve(model, &key, &entry.services)
            .into_iter()
            .find(|route| route.route.key == destination))
    }

    #[test]
    fn m38_return_origin_correction_is_optional_and_preserves_exact_policy()
    -> Result<(), Box<dyn std::error::Error>> {
        let rows: Vec<serde_json::Value> = serde_json::from_str(revision_material())?;
        let mut row = rows.into_iter().next().ok_or("travel correction")?;
        let legacy: super::Correction = serde_json::from_value(row.clone())?;
        assert!(legacy.return_origin.is_none());
        row["return_origin"] = serde_json::json!({
            "inbound_routes": ["oteryn:service.travel.port_a/travora", "oteryn:service.travel.port_b/travora"],
            "return_from": "oteryn:service.travel.port_b/travora"
        });
        let correction: super::Correction = serde_json::from_value(row.clone())?;
        let policy = correction.return_origin.ok_or("return policy")?;
        assert!(policy.is_valid());
        assert_eq!(policy.inbound_routes.len(), 2);
        assert!(policy.permits_latest_completed(Some("oteryn:service.travel.port_b/travora")));
        row["return_origin"]["fallback"] = serde_json::json!("any_port");
        assert!(serde_json::from_value::<super::Correction>(row).is_err());
        Ok(())
    }

    #[test]
    fn m38_travel_ticket_requirement_is_optional_and_keeps_elapsed_seconds()
    -> Result<(), Box<dyn std::error::Error>> {
        let rows: Vec<serde_json::Value> = serde_json::from_str(revision_material())?;
        let mut row = rows.into_iter().next().ok_or("travel correction")?;
        let legacy: super::Correction = serde_json::from_value(row.clone())?;
        assert!(legacy.ticket.is_none());
        row["ticket"] = serde_json::json!({"track":"oteryn:quest-progress/travora/ticket_purchased", "valid_for_seconds":86400});
        let guarded: super::Correction = serde_json::from_value(row.clone())?;
        assert_eq!(
            guarded.ticket,
            Some(super::TravelTicketRequirement {
                track: "oteryn:quest-progress/travora/ticket_purchased".into(),
                valid_for_seconds: 86400,
            })
        );
        row["ticket"]["client_clock"] = 1.into();
        assert!(serde_json::from_value::<super::Correction>(row).is_err());
        Ok(())
    }

    #[test]
    fn m38_travel_clear_gate_is_optional_and_preserved_by_the_catalogue_parser()
    -> Result<(), Box<dyn std::error::Error>> {
        let rows: Vec<serde_json::Value> = serde_json::from_str(revision_material())?;
        let mut row = rows.into_iter().next().ok_or("travel correction")?;
        let legacy: super::Correction = serde_json::from_value(row.clone())?;
        assert!(legacy.quest_clear_gate.is_none());
        row["quest_clear_gate"] = "oteryn:quest-progress/monastery/fine".into();
        let guarded: super::Correction = serde_json::from_value(row)?;
        assert_eq!(
            guarded.quest_clear_gate.as_deref(),
            Some("oteryn:quest-progress/monastery/fine")
        );
        Ok(())
    }

    #[test]
    fn destination_apostrophes_survive_native_route_resolution()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalogue = crate::content::NpcDataCatalogue::load(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content"),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )?;
        let model = catalogue.service_model_for_project()?;
        for (npc, slug, keyword, held) in [
            ("captain_bluebear", "ab_dendriel", "ab'dendriel", false),
            ("captain_fearless", "ab_dendriel", "ab'dendriel", false),
            ("captain_greyhound", "ab_dendriel", "ab'dendriel", false),
            ("captain_seahorse", "ab_dendriel", "ab'dendriel", false),
            ("junkar_mines", "robsons_isle", "robson's isle", true),
            ("junkar_thais", "robsons_isle", "robson's isle", true),
        ] {
            let key = format!("oteryn:npc.{npc}");
            let entry = model
                .npcs()
                .iter()
                .find(|(reference, _)| reference.key == key)
                .map(|(_, entry)| entry)
                .ok_or("NPC")?;
            let routes = super::resolve(&model, &key, &entry.services);
            let route = routes
                .iter()
                .find(|route| route.route.key == slug)
                .ok_or("route")?;
            assert_eq!(route.keyword, keyword, "{npc}/{slug}");
            assert_eq!(route.held, held, "{npc}/{slug}");
        }
        Ok(())
    }

    #[test]
    fn paid_travel_routes_preserve_all_195_corrected_fares_premium_and_arrivals()
    -> Result<(), Box<dyn std::error::Error>> {
        let corrections: Vec<super::Correction> = serde_json::from_str(revision_material())?;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/reference/content-prep/travel-content-1");
        let source: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("source-correction-check.json"))?)?;
        let evidence: serde_json::Value = serde_json::from_slice(&std::fs::read(
            root.join("admission-evidence/evidence-00000-00194.json"),
        )?)?;
        let keys = corrections
            .iter()
            .map(|row| &row.key)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(keys.len(), corrections.len());
        let dalbrect = corrections
            .iter()
            .find(|row| row.key == "oteryn:service.travel.dalbrect/isle_of_kings")
            .ok_or("source-qualified Dalbrect route")?;
        assert!(dalbrect.native_addition && !dalbrect.held && !dalbrect.premium);
        assert_eq!((&*dalbrect.keyword, dalbrect.price), ("passage", 10));
        assert_eq!((dalbrect.x, dalbrect.y, dalbrect.floor), (32190, 31957, 6));
        assert_eq!(
            dalbrect.quest_gate.as_deref(),
            Some("oteryn:quest-progress/quest/u7_24/the_white_raven_monastery/quest_log")
        );
        let eclipse = corrections
            .iter()
            .find(|row| row.key == "oteryn:service.travel.uzon/eclipse")
            .ok_or("source-qualified Eclipse route")?;
        assert!(eclipse.native_addition && !eclipse.held && !eclipse.premium);
        assert_eq!(
            (eclipse.x, eclipse.y, eclipse.floor, eclipse.price),
            (32659, 31915, 0, 110)
        );
        let [gate] = eclipse.quest_tracks.as_slice() else {
            return Err("one Eclipse gate".into());
        };
        assert_eq!(
            gate.track,
            "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline"
        );
        assert_eq!((gate.minimum, gate.maximum), (4, Some(5)));
        for (key, fare, x, y, floor) in [
            (
                "oteryn:service.travel.brodrosch/cormaya",
                160,
                33_311,
                31_989,
                15,
            ),
            ("oteryn:service.travel.eremo/cormaya", 0, 33_288, 31_956, 6),
        ] {
            let route = corrections
                .iter()
                .find(|route| route.key == key)
                .ok_or(key)?;
            assert!(route.native_addition && !route.held && !route.premium);
            assert_eq!(
                (route.price, route.x, route.y, route.floor),
                (fare, x, y, floor)
            );
        }
        let sources = source["rows"].as_array().ok_or("source rows")?;
        let evidence = evidence["records"].as_array().ok_or("evidence rows")?;
        let nargor = "oteryn:service.travel.sebastian/nargor";
        let corrected = corrections
            .iter()
            .filter(|row| !row.native_addition || row.key == nargor)
            .collect::<Vec<_>>();
        assert_eq!(corrected.len(), 195);
        let travora_ports = [
            ("captain_bluebear", "thais", 32_310, 32_210, 6),
            ("captain_breezelda", "svargrond", 32_341, 31_108, 6),
            ("captain_fearless", "venore", 32_954, 32_022, 6),
            ("captain_greyhound", "carlin", 32_387, 31_820, 6),
            ("captain_pelagia", "krailos", 33_493, 31_712, 6),
            ("captain_seagull", "ab_dendriel", 32_734, 31_668, 6),
            ("captain_seahorse", "edron", 33_175, 31_764, 6),
            ("captain_sinbeard", "ankrahmun", 33_092, 32_883, 6),
            (
                "captain_waverider",
                "liberty_bay_waverider",
                32_349,
                32_856,
                7,
            ),
            (
                "captain_waverider",
                "treasure_hunt_island",
                32_045,
                32_931,
                7,
            ),
            ("charles", "port_hope", 32_527, 32_784, 6),
            ("jack_fate", "liberty_bay", 32_285, 32_892, 6),
            ("karith", "yalahar", 32_816, 31_272, 6),
            ("petros", "darashia", 33_289, 32_480, 6),
        ];
        let inbound_routes = travora_ports
            .iter()
            .map(|(captain, port, ..)| {
                let destination = if *port == "treasure_hunt_island" {
                    "travora_treasure_hunt_island"
                } else {
                    "travora"
                };
                format!("oteryn:service.travel.{captain}/{destination}")
            })
            .collect::<Vec<_>>();
        let ticket = super::TravelTicketRequirement {
            track: "oteryn:quest-progress/travora/ticket_purchased".into(),
            valid_for_seconds: 86400,
        };
        for ((_, port, x, y, floor), inbound_key) in travora_ports.iter().zip(&inbound_routes) {
            let inbound = corrections
                .iter()
                .find(|row| row.key == *inbound_key)
                .ok_or("Travora inbound")?;
            assert!(inbound.native_addition && !inbound.held && !inbound.premium);
            assert_eq!(
                (inbound.price, inbound.x, inbound.y, inbound.floor),
                (0, 32_055, 32_368, 6)
            );
            assert_eq!(inbound.ticket.as_ref(), Some(&ticket));
            assert!(inbound.return_origin.is_none());

            let return_key = format!("oteryn:service.travel.captain_tiberius/{port}");
            let outbound = corrections
                .iter()
                .find(|row| row.key == return_key)
                .ok_or("Travora return")?;
            assert!(outbound.native_addition && !outbound.held && !outbound.premium);
            assert_eq!(
                (outbound.price, outbound.x, outbound.y, outbound.floor),
                (0, *x, *y, *floor)
            );
            assert!(outbound.ticket.is_none(), "{return_key}");
            assert_eq!(
                outbound.return_origin.as_ref(),
                Some(&super::TravelReturnRequirement {
                    inbound_routes: inbound_routes.clone(),
                    return_from: inbound_key.clone(),
                }),
            );
        }
        let platform = corrections
            .iter()
            .find(|row| row.key == "oteryn:service.travel.sharon/departure_platform")
            .ok_or("Sharon platform")?;
        assert_eq!(
            (
                platform.keyword.as_str(),
                platform.price,
                platform.x,
                platform.y,
                platform.floor
            ),
            ("check", 0, 32062, 32367, 5)
        );
        assert_eq!(
            corrections.len() - corrected.len(),
            61 + 2 * travora_ports.len()
        );
        assert_eq!(sources.len(), 195);
        assert_eq!(evidence.len(), 195);
        for ((row, source), evidence) in corrected.into_iter().zip(sources).zip(evidence) {
            assert_eq!(serde_json::json!(row.key), source["key"]);
            // Historical evidence stores slugs; talk keywords use the destination spelling.
            let keyword = match source["keyword"].as_str().ok_or("source keyword")? {
                "ab_dendriel" => "ab'dendriel",
                "robsons_isle" => "robson's isle",
                keyword => keyword,
            };
            assert_eq!(row.keyword, keyword);
            // Crystal harlow_vengoth.lua:53-55 supplies a free return.
            let price = if row.key == "oteryn:service.travel.harlow_vengoth/yalahar" {
                serde_json::json!(0)
            } else {
                source["selected_fare"].clone()
            };
            assert_eq!(serde_json::json!(row.price), price);
            assert_eq!(serde_json::json!(row.premium), source["selected_premium"]);
            match row.key.as_str() {
                // The native Marapur landing is one tile north of the donor destination.
                "oteryn:service.travel.melian/marapur"
                | "oteryn:service.travel.pino/marapur"
                | "oteryn:service.travel.tanyt/marapur" => assert_eq!(row.y, 32_766, "{}", row.key),
                // Share Ziyad's Kazordoon landing, clear of Gewen's occupied donor tile.
                "oteryn:service.travel.uzon/kazor" | "oteryn:service.travel.uzon/kazordoon" => {
                    assert_eq!(row.y, 31_941, "{}", row.key);
                }
                // Captain Max stands on the donor arrival cell; land on the clear tile south.
                "oteryn:service.travel.captain_max_calassa/liberty_bay" => {
                    assert_eq!(row.y, 32_896, "{}", row.key);
                }
                _ => assert_eq!(serde_json::json!(row.y), source["destination"]["y"]),
            }
            assert_eq!(serde_json::json!(row.x), source["destination"]["x"]);
            assert_eq!(serde_json::json!(row.floor), source["destination"]["z"]);
            let facts = &evidence["facts"];
            assert_eq!(row.carpet, facts["transport"]["value"] == "carpet");
            if row.carpet {
                assert!(row.premium, "{}", row.key);
            }
            // Source-backed access needs a live quest owner or event before admission.
            if row.key == nargor {
                assert!(!row.held);
                assert_eq!(
                    row.quest_gate.as_deref(),
                    Some("oteryn:quest-progress/quest/u7_8/the_shattered_isles/access_to_nargor"),
                );
            } else if facts["quest_requirements"]["value"] != serde_json::json!([])
                || !facts["travel_guard_conditions"]["value"].is_null()
                || row.key == "oteryn:service.travel.kendra_thais/nostalgia"
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
        let corrections: Vec<super::Correction> = serde_json::from_str(revision_material())?;
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
                !matches!(
                    npc,
                    "captain_jack_rat" | "captain_max" | "captain_waverider"
                )
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
    fn regional_captain_routes_resolve_to_donor_fares_and_premium()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let catalogue = crate::content::NpcDataCatalogue::load(
            &root.join("content"),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )?;
        let model = catalogue.service_model_for_project()?;
        let source: serde_json::Value = serde_json::from_slice(&std::fs::read(
            root.join("docs/reference/content-prep/travel-content-1/source-correction-check.json"),
        )?)?;
        let rows = source["rows"].as_array().ok_or("travel source rows")?;
        for (npc, slug, held) in [
            ("captain_gulliver", "issavi", false),
            ("captain_gulliver", "krailos", true),
            ("captain_gulliver", "thais", false),
            ("petros", "issavi", true),
        ] {
            let key = format!("oteryn:npc.{npc}");
            let entry = model
                .npcs()
                .iter()
                .find(|(reference, _)| reference.key == key)
                .map(|(_, entry)| entry)
                .ok_or("NPC")?;
            let route = super::resolve(&model, &key, &entry.services)
                .into_iter()
                .find(|candidate| candidate.route.key == slug)
                .ok_or("route")?;
            let route_key = format!("{}/{slug}", route.service);
            let donor = rows
                .iter()
                .find(|row| row["key"] == route_key)
                .ok_or("donor route")?;
            assert_eq!(
                (
                    route.route.price,
                    route.route.premium,
                    route.held,
                    route.postman
                ),
                (
                    donor["donor_cost"].as_u64().ok_or("donor fare")?,
                    donor["donor_premium"].as_bool().ok_or("donor premium")?,
                    held,
                    true
                ),
                "{route_key}"
            );
            let destination = &route.route.destination;
            let x = donor["destination"]["x"].as_i64().ok_or("donor x")?;
            let y = donor["destination"]["y"].as_i64().ok_or("donor y")?;
            let floor = donor["destination"]["z"].as_i64().ok_or("donor floor")?;
            assert_eq!(
                (
                    i64::from(destination.x),
                    i64::from(destination.y),
                    i64::from(destination.floor)
                ),
                (x, y, floor),
                "{route_key}"
            );
        }
        Ok(())
    }

    #[test]
    fn postman_tour_routes_accept_free_accounts() -> Result<(), Box<dyn std::error::Error>> {
        let catalogue = crate::content::NpcDataCatalogue::load(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../content"),
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )?;
        let model = catalogue.service_model_for_project()?;
        let mut routes = Vec::new();
        for (key, npc) in model.npcs() {
            for route in super::resolve(&model, &key.key, &npc.services) {
                if let Some((stage, _)) = route.postman_tour_passage() {
                    routes.push((stage, route.route.premium));
                }
            }
        }
        routes.sort_unstable();
        assert_eq!(routes, [(1, false), (2, false), (3, false), (4, false)]);
        Ok(())
    }

    #[test]
    fn remaining_captain_routes_preserve_unlocks_and_custom_fares()
    -> Result<(), Box<dyn std::error::Error>> {
        let rows: Vec<super::Correction> = serde_json::from_str(revision_material())?;
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
            if row.key == "oteryn:service.travel.karith/travora" {
                assert!(row.yalahar_town_gate.is_none());
                assert_eq!(
                    row.ticket.as_ref().map(|ticket| ticket.valid_for_seconds),
                    Some(86400)
                );
            } else {
                assert!(row.yalahar_town_gate.is_some());
            }
        }
        // These donor routes cannot be safely offered with the currently admitted owners.
        for key in ["captain_max/yalahar", "captain_bluebear/targuna"] {
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
