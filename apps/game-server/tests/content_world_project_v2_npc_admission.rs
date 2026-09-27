#![allow(clippy::expect_used)]

//! OTERYN_WORLD_PROJECT_V2_NPC_ADMISSION_V1 §4 and §6: an NPC bound to Presentation and Behavior
//! profiles (outfit, wander), with typed trade offers and travel routes on declarative Services.

use oteryn_game_server::content::*;
use serde_json::{Value, json};

const REVISION: &str = "definition-r1";
const NPC: &str = "oteryn:npc.captain_bluebear";
const PRESENTATION: &str = "oteryn:presentation.npc.captain_bluebear";
const BEHAVIOR: &str = "oteryn:behavior.npc.captain_bluebear";
const TRADE: &str = "oteryn:service.trade.captain_bluebear";
const TRAVEL: &str = "oteryn:service.travel.captain_bluebear";
const AXE: &str = "oteryn:item.registry.i00003155";
const RUNE: &str = "oteryn:item.registry.i00003161";
const TOKEN: &str = "oteryn:item.registry.i00021718";
const FRAME: &str = "global-target-2026-07-28";

fn limits() -> ProjectEvidenceLimits {
    ProjectEvidenceLimits {
        max_documents: 12,
        max_document_bytes: 65_536,
        max_total_bytes: 262_144,
        max_json_depth: 24,
        max_decoded_fields: 4_096,
        max_string_bytes: 32_768,
        max_locator_bytes: 160,
        max_locator_segments: 8,
        max_reference_records: 64,
        max_import_records: 8,
        max_reimport_states: 8,
    }
}

fn reference(family: ProjectV2Family, key: &str) -> ProjectV2DefinitionRef {
    ProjectV2DefinitionRef {
        family,
        key: key.into(),
        revision: REVISION.into(),
    }
}

fn identity(key: &str) -> ProjectV2Identity {
    ProjectV2Identity {
        key: key.into(),
        revision: REVISION.into(),
    }
}

fn item(key: &str) -> ProjectReferenceRecord {
    ProjectReferenceRecord::Item {
        identity: DefinitionIdentityDocument {
            family: "Item".into(),
            key: key.into(),
            revision: REVISION.into(),
        },
        client_projection: ProjectionDocument::ClientSafe,
        materializable: false,
        stack_class: ItemStackDocument::Unknown,
        semantics: ReferenceItemSemantics::default(),
    }
}

fn generic(family: &str, key: &str, projection: ProjectionDocument) -> ProjectReferenceRecord {
    ProjectReferenceRecord::Generic {
        identity: DefinitionIdentityDocument {
            family: family.into(),
            key: key.into(),
            revision: REVISION.into(),
        },
        client_projection: projection,
    }
}

fn wander() -> ProjectV2Wander {
    ProjectV2Wander {
        interval_ms: 2_000,
        radius_tiles: 2,
    }
}

fn profiles() -> Vec<ProjectV2AuthoringProfile> {
    let palette = |slot, color: u16| ProjectV2SlotBinding {
        slot,
        asset_binding: format!("canary.appearance:palette/{color}"),
    };
    vec![
        ProjectV2AuthoringProfile {
            target: reference(ProjectV2Family::Presentation, PRESENTATION),
            data: ProjectV2AuthoringProfileData::Presentation(ProjectV2PresentationAuthoring {
                asset_binding: Some("canary.appearance:outfit/129".into()),
                selection: None,
                palette_bindings: vec![
                    palette(ProjectV2PaletteSlot::Head, 19),
                    palette(ProjectV2PaletteSlot::Body, 69),
                    palette(ProjectV2PaletteSlot::Legs, 125),
                    palette(ProjectV2PaletteSlot::Feet, 50),
                ],
                attachment_bindings: vec![ProjectV2SlotBinding {
                    slot: ProjectV2AttachmentSlot::Addon,
                    asset_binding: "canary.appearance:outfit/129/addon-1".into(),
                }],
                visual_effect_bindings: vec![],
                light_level: 0,
                light_color_binding: None,
                audio_bindings: vec![],
                variant_label: None,
                status_marker: None,
            }),
        },
        ProjectV2AuthoringProfile {
            target: reference(ProjectV2Family::Behavior, BEHAVIOR),
            data: ProjectV2AuthoringProfileData::Behavior(ProjectV2BehaviorAuthoring {
                movement: ProjectV2Movement {
                    can_walk: true,
                    pass_through: false,
                    pushable: false,
                    push_items: false,
                    push_creatures: false,
                    walks_on_energy: false,
                    walks_on_fire: false,
                    walks_on_poison: false,
                    wander: Some(wander()),
                },
                targeting: ProjectV2Targeting {
                    hostile: false,
                    can_target: false,
                    sense_invisible: false,
                    target_distance_tiles: 0,
                    static_attack_chance_ppm: 0,
                    flee_health: 0,
                    change_target: None,
                    strategy_weights: None,
                },
                attacks: vec![],
                defenses: vec![],
                voices: None,
                summons: None,
                periodic_audio: None,
                faction: None,
                event_bindings: vec![],
            }),
        },
    ]
}

fn behavior_mut(draft: &mut ProjectV2Draft) -> &mut ProjectV2BehaviorAuthoring {
    draft
        .state
        .authoring_profiles
        .iter_mut()
        .find_map(|profile| match &mut profile.data {
            ProjectV2AuthoringProfileData::Behavior(behavior) => Some(behavior),
            _ => None,
        })
        .expect("behavior profile")
}

fn offer(
    key: &str,
    direction: ProjectV2ServiceOfferDirection,
    unit_price: u64,
) -> ProjectV2ServiceOffer {
    ProjectV2ServiceOffer {
        item: reference(ProjectV2Family::Item, key),
        direction,
        unit_price,
        currency: None,
        count: None,
        sub_type: None,
    }
}

fn route(key: &str, x: i32, y: i32, price: u64) -> ProjectV2TravelRoute {
    ProjectV2TravelRoute {
        key: key.into(),
        destination: ProjectV2TravelDestination {
            coordinate_frame: FRAME.into(),
            x,
            y,
            floor: 6,
        },
        price,
        premium: false,
        min_level: None,
    }
}

fn offers() -> Vec<ProjectV2ServiceOffer> {
    let mut rune = offer(RUNE, ProjectV2ServiceOfferDirection::SellToPlayer, 175);
    rune.count = Some(3);
    let mut token = offer(AXE, ProjectV2ServiceOfferDirection::SellToPlayer, 2);
    token.currency = Some(reference(ProjectV2Family::Item, TOKEN));
    vec![
        offer(AXE, ProjectV2ServiceOfferDirection::SellToPlayer, 20),
        offer(AXE, ProjectV2ServiceOfferDirection::BuyFromPlayer, 7),
        rune,
        token,
    ]
}

fn routes() -> Vec<ProjectV2TravelRoute> {
    let mut edron = route("edron", 33175, 31764, 160);
    edron.premium = true;
    edron.min_level = Some(8);
    vec![route("carlin", 32387, 31820, 110), edron]
}

fn draft() -> ProjectV2Draft {
    ProjectV2Draft {
        core: ProjectDraft {
            project_revision: "project-r1".into(),
            package_key: "oteryn:content.world-project".into(),
            semantic_schema_version: "reference-schema-v1".into(),
            licensing_metadata: "license:project-owned-v1".into(),
            world_id: "0123456789ab70cd8ef0123456789abc".into(),
            coordinate_frame: FRAME.into(),
            records: vec![
                generic("Presentation", PRESENTATION, ProjectionDocument::ClientSafe),
                generic("Behavior", BEHAVIOR, ProjectionDocument::ServerOnly),
                item(AXE),
                item(RUNE),
                item(TOKEN),
            ],
            imports: vec![],
            metadata: vec![],
        },
        state: ProjectV2State {
            declarations: vec![
                ProjectV2Declaration::Npc {
                    identity: identity(NPC),
                    presentation: Some(reference(ProjectV2Family::Presentation, PRESENTATION)),
                    behavior: Some(reference(ProjectV2Family::Behavior, BEHAVIOR)),
                    dialogue: None,
                    services: vec![
                        reference(ProjectV2Family::Service, TRADE),
                        reference(ProjectV2Family::Service, TRAVEL),
                    ],
                    fields: vec![ProjectV2CandidateField {
                        field_path: "oteryn:source.npc.profession".into(),
                        value: ProjectV2CandidateValue::Text("sailor".into()),
                    }],
                },
                ProjectV2Declaration::Service {
                    identity: identity(TRADE),
                    offers: offers(),
                    recipes: vec![],
                    routes: vec![],
                    fields: vec![],
                },
                ProjectV2Declaration::Service {
                    identity: identity(TRAVEL),
                    offers: vec![],
                    recipes: vec![],
                    routes: routes(),
                    fields: vec![],
                },
            ],
            authoring_profiles: profiles(),
            ..ProjectV2State::default()
        },
    }
}

fn admit(draft: ProjectV2Draft) -> Result<WorldProject, String> {
    let documents = CanonicalProjectDocuments::from_v2_draft(draft, limits())
        .map_err(|error| format!("{error:?}"))?;
    ProjectSnapshot::new(documents.documents().clone(), limits())
        .map_err(|error| format!("{error:?}"))?
        .parse(limits())
        .map_err(|error| format!("{error:?}"))
}

fn service_mut<'a>(draft: &'a mut ProjectV2Draft, key: &str) -> &'a mut ProjectV2Declaration {
    draft
        .state
        .declarations
        .iter_mut()
        .find(|declaration| matches!(declaration, ProjectV2Declaration::Service { identity, .. } if identity.key == key))
        .expect("service")
}

fn service<'a>(
    project: &'a WorldProject,
    key: &str,
) -> (&'a [ProjectV2ServiceOffer], &'a [ProjectV2TravelRoute]) {
    project
        .v2()
        .expect("v2 state")
        .declarations
        .iter()
        .find_map(|declaration| match declaration {
            ProjectV2Declaration::Service {
                identity,
                offers,
                routes,
                ..
            } if identity.key == key => Some((offers.as_slice(), routes.as_slice())),
            _ => None,
        })
        .expect("service")
}

#[test]
fn npc_services_round_trip_and_stay_declarative() {
    let documents =
        CanonicalProjectDocuments::from_v2_draft(draft(), limits()).expect("npc documents");
    let parsed = ProjectSnapshot::new(documents.documents().clone(), limits())
        .expect("admit npc")
        .parse(limits())
        .expect("parse npc");
    let (trade, _) = service(&parsed, TRADE);
    assert_eq!(trade.len(), 4);
    assert!(
        trade
            .iter()
            .any(|offer| offer.count == Some(3) && offer.unit_price == 175)
    );
    let (_, travel) = service(&parsed, TRAVEL);
    assert_eq!(
        travel
            .iter()
            .map(|route| route.key.as_str())
            .collect::<Vec<_>>(),
        ["carlin", "edron"]
    );
    assert_eq!(travel[1].min_level, Some(8));
    assert_eq!(
        parsed
            .canonical_documents(limits())
            .expect("canonical rewrite")
            .documents(),
        documents.documents()
    );
    let reference = parsed
        .lower_reference_source()
        .expect("reference projection stays executable-only");
    assert_eq!(
        reference.definitions.len(),
        5,
        "only the Presentation, the Behavior and the three Items lower"
    );
    let state = parsed.v2().expect("v2 state");
    let mut expected = profiles();
    expected.sort_by(|left, right| left.target.cmp(&right.target));
    assert_eq!(state.authoring_profiles, expected);
}

#[test]
fn offers_and_routes_are_canonicalized() {
    let mut changed = draft();
    if let ProjectV2Declaration::Service { offers, .. } = service_mut(&mut changed, TRADE) {
        offers.reverse();
    }
    if let ProjectV2Declaration::Service { routes, .. } = service_mut(&mut changed, TRAVEL) {
        routes.reverse();
    }
    let reordered = admit(changed).expect("admit reordered npc");
    let original = admit(draft()).expect("admit npc");
    assert_eq!(service(&reordered, TRADE), service(&original, TRADE));
    assert_eq!(service(&reordered, TRAVEL).1[0].key, "carlin");
}

#[test]
fn each_broken_invariant_is_rejected() {
    type Mutation = fn(&mut ProjectV2Draft);
    let cases: [(&str, &str, Mutation); 11] = [
        (
            "wander without walking",
            "v2 wander requires a walking creature and a positive interval",
            |draft| {
                behavior_mut(draft).movement.can_walk = false;
            },
        ),
        (
            "zero wander interval",
            "v2 wander requires a walking creature and a positive interval",
            |draft| {
                if let Some(wander) = behavior_mut(draft).movement.wander.as_mut() {
                    wander.interval_ms = 0;
                }
            },
        ),
        (
            "duplicate route key",
            "v2 Service routes are not key sorted and unique",
            |draft| {
                if let ProjectV2Declaration::Service { routes, .. } = service_mut(draft, TRAVEL) {
                    routes[1].key = "carlin".into();
                }
            },
        ),
        (
            "route key with capitals",
            "v2 Service route key is not a lowercase slug",
            |draft| {
                if let ProjectV2Declaration::Service { routes, .. } = service_mut(draft, TRAVEL) {
                    routes[0].key = "Carlin".into();
                }
            },
        ),
        (
            "route key with an empty part",
            "v2 Service route key is not a lowercase slug",
            |draft| {
                if let ProjectV2Declaration::Service { routes, .. } = service_mut(draft, TRAVEL) {
                    routes[0].key = "port__hope".into();
                }
            },
        ),
        (
            "floor above 15",
            "v2 Service route destination is out of range",
            |draft| {
                if let ProjectV2Declaration::Service { routes, .. } = service_mut(draft, TRAVEL) {
                    routes[0].destination.floor = 16;
                }
            },
        ),
        (
            "negative x",
            "v2 Service route destination is out of range",
            |draft| {
                if let ProjectV2Declaration::Service { routes, .. } = service_mut(draft, TRAVEL) {
                    routes[0].destination.x = -1;
                }
            },
        ),
        (
            "empty coordinate frame",
            "v2 Service route destination is out of range",
            |draft| {
                if let ProjectV2Declaration::Service { routes, .. } = service_mut(draft, TRAVEL) {
                    routes[0].destination.coordinate_frame.clear();
                }
            },
        ),
        (
            "zero offer count",
            "v2 Service offer count must be positive",
            |draft| {
                if let ProjectV2Declaration::Service { offers, .. } = service_mut(draft, TRADE) {
                    offers[2].count = Some(0);
                }
            },
        ),
        (
            "one Item row with two prices",
            "v2 Service offers repeat an Item row with different prices",
            |draft| {
                if let ProjectV2Declaration::Service { offers, .. } = service_mut(draft, TRADE) {
                    offers.push(offer(AXE, ProjectV2ServiceOfferDirection::SellToPlayer, 25));
                }
            },
        ),
        (
            "offer names an unknown Item",
            "unresolved v2 typed definition reference",
            |draft| {
                if let ProjectV2Declaration::Service { offers, .. } = service_mut(draft, TRADE) {
                    offers[0].item.key = "oteryn:item.registry.i00099999".into();
                }
            },
        ),
    ];
    for (case, expected, mutate) in cases {
        let mut changed = draft();
        mutate(&mut changed);
        let error = admit(changed).expect_err(case);
        assert!(error.contains(expected), "{case}: {error}");
    }
}

#[test]
fn unknown_route_fields_fail_closed() {
    let documents =
        CanonicalProjectDocuments::from_v2_draft(draft(), limits()).expect("npc documents");
    let declarations: Value =
        serde_json::from_slice(&documents.documents()["definitions/declarations.json"])
            .expect("declarations");
    let mut travel = declarations["records"]
        .as_array()
        .expect("records")
        .iter()
        .find(|record| record["identity"]["key"] == TRAVEL)
        .expect("travel")
        .clone();
    assert!(serde_json::from_value::<ProjectV2Declaration>(travel.clone()).is_ok());
    travel["routes"][0]["discount"] = json!("postman");
    assert!(
        serde_json::from_value::<ProjectV2Declaration>(travel).is_err(),
        "unknown route field admitted"
    );
}
