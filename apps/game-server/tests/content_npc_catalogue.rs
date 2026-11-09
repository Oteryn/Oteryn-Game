#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]
use oteryn_game_server::content::*;
use std::{ffi::OsStr, path::PathBuf};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/world")
}
fn captured() -> WorldProject {
    capture_world_project(
        root().parent().expect("parent"),
        OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .expect("validated repository source")
}
#[test]
fn repository_import_preserves_all_data_and_approximation_flags() {
    let project = captured();
    let baseline = project.v2().expect("v2");
    let npc = baseline.declarations.iter().find(|d| matches!(d,ProjectV2Declaration::Npc {identity,..} if identity.key=="oteryn:npc.mud")).expect("MUD").clone();
    let sources = baseline.sources.clone();
    let bindings = baseline.source_identity_bindings.clone();
    let catalogue =
        NpcDataCatalogue::from_project(project, npc_catalogue_preproduction_limits().project)
            .expect("data import");
    // The source tree digest is refreshed by regenerate_content.py with the package pins.
    assert_eq!(catalogue.project_revision(), "npcfix-retail-20261106-r1");
    assert_eq!(
        catalogue.source_tree_digest(),
        "76631766b485deb9e3f0917ca3cbcdf35b1802198decb299925f49b17c9e12f5"
    );
    assert_eq!(catalogue.npc_count(), 1290);
    assert_eq!(catalogue.dialogue_count(), 839);
    assert_eq!(catalogue.service_count(), 405);
    assert_eq!(catalogue.sources(), sources);
    assert_eq!(catalogue.source_identity_bindings(), bindings);
    let (reference, imported) = catalogue
        .records()
        .find(|(r, _)| r.key == "oteryn:npc.mud")
        .expect("imported MUD");
    assert_eq!(imported, &npc);
    assert_eq!(catalogue.declaration(reference), Some(&npc));
    if let ProjectV2Declaration::Npc {
        presentation,
        behavior,
        fields,
        ..
    } = imported
    {
        assert!(
            presentation
                .as_ref()
                .is_some_and(|r| catalogue.profile(r).is_some())
        );
        assert!(
            behavior
                .as_ref()
                .is_some_and(|r| catalogue.profile(r).is_some())
        );
        assert!(fields.iter().any(|f|matches!(&f.value,ProjectV2CandidateValue::Text(v) if v.contains("APPROXIMATE_WIKI_VISUAL_INVISIBLE_MAPPING"))));
    }
}
#[test]
fn dangling_dialogue_is_rejected_by_the_existing_import_admission() {
    let mut draft = captured().migrate_to_v2();
    let actor = draft
        .state
        .declarations
        .iter_mut()
        .find(|d| {
            matches!(
                d,
                ProjectV2Declaration::Npc {
                    dialogue: Some(_),
                    ..
                }
            )
        })
        .expect("NPC with dialogue");
    if let ProjectV2Declaration::Npc {
        dialogue: Some(reference),
        ..
    } = actor
    {
        reference.key = "oteryn:dialogue.npc.missing_data_import_target".to_owned();
    }
    assert!(
        CanonicalProjectDocuments::from_v2_draft(
            draft,
            npc_catalogue_preproduction_limits().project
        )
        .is_err()
    );
}
#[test]
fn malformed_and_mismatched_source_pins_fail_closed() {
    assert!(load_data_only_npc_catalogue(&root(), &"0".repeat(64)).is_err());
    assert!(
        load_data_only_npc_catalogue(&PathBuf::from("/nonexistent/npc-project"), "not-a-sha256")
            .is_err()
    );
}

#[test]
fn legacy_project_without_v2_cannot_be_imported_as_npc_data() {
    let core = captured().migrate_to_v2().core;
    let limits = npc_catalogue_preproduction_limits().project;
    let legacy = CanonicalProjectDocuments::from_draft(core, limits)
        .expect("valid legacy project")
        .into_snapshot(limits)
        .expect("legacy snapshot")
        .parse(limits)
        .expect("validated legacy");
    assert!(legacy.v2().is_none());
    assert!(NpcDataCatalogue::from_project(legacy, limits).is_err());
}

#[test]
fn npcfix_retail_services_and_greeting_match_the_audited_repairs() {
    let packet: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tools/content-schema/npc-authoring/samples/npcfix-native-repairs.json"
    ))
    .expect("NPCFIX evidence");
    let project = captured();
    let state = project.v2().expect("v2");
    for repair in packet["repairs"].as_array().expect("repairs") {
        let expected: ProjectV2Declaration =
            serde_json::from_value(repair["after"].clone()).expect("typed repair");
        assert!(
            state.declarations.contains(&expected),
            "missing repair: {}",
            repair["after"]["identity"]["key"]
        );
    }
    let catalogue =
        NpcDataCatalogue::from_project(project, npc_catalogue_preproduction_limits().project)
            .expect("NPCFIX catalogue");
    let model = catalogue
        .service_model_for_project()
        .expect("NPCFIX services");
    let book = oteryn_gameplay_npc::npc_talk::NpcTalkBook::from_model(&model);
    let mut unavailable = Vec::new();
    for repair in packet["repairs"].as_array().expect("repairs") {
        let after: ProjectV2Declaration =
            serde_json::from_value(repair["after"].clone()).expect("typed repair");
        if let ProjectV2Declaration::Service {
            identity, offers, ..
        } = after
        {
            if offers.is_empty() {
                continue;
            }
            let npc = identity
                .key
                .replacen("oteryn:service.trade.", "oteryn:npc.", 1);
            let entry = book.get(&npc).expect(&npc);
            let before: Vec<ProjectV2ServiceOffer> = repair["before"]["offers"]
                .as_array()
                .map(|rows| {
                    rows.iter()
                        .map(|r| serde_json::from_value(r.clone()).expect("before offer"))
                        .collect()
                })
                .unwrap_or_default();
            for offer in offers.iter().filter(|offer| !before.contains(offer)) {
                if !entry.offers().contains(offer) {
                    unavailable.push(format!(
                        "{npc}: repaired offer must be executable: {offer:?}"
                    ));
                }
            }
        }
    }
    assert!(unavailable.is_empty(), "{}", unavailable.join("\n"));
    for (key, name) in [
        ("fitzgerald", "Fitzgerald"),
        ("lizard_prisoner", "Lizard Prisoner"),
        ("eustacios_spy_rat", "Eustacio's Spy Rat"),
        ("ellipsis", "..."),
        ("glimpse_of_allusion", "Glimpse of Allusion"),
        ("wox", "WOX"),
        ("yielothax", "Yielothax"),
        ("doctor_marrow", "Doctor Marrow"),
    ] {
        assert_eq!(book.display_name(&format!("oteryn:npc.{key}")), Some(name));
    }
    assert!(
        book.get("oteryn:npc.lizard_prisoner").is_none(),
        "the retail prisoners do not answer greetings"
    );
    for key in ["murim", "muzir", "znozel"] {
        assert!(
            book.get(&format!("oteryn:npc.{key}"))
                .is_some_and(|entry| entry.bank()),
            "bank context: {key}"
        );
    }
    let albinius = book.get("oteryn:npc.albinius").expect("Albinius shop");
    assert!(
        albinius.offers().iter().any(|offer| {
            offer.item.key == "oteryn:item.tibia.i23986"
                && offer.direction == ProjectV2ServiceOfferDirection::BuyFromPlayer
                && offer.unit_price == 30
        }),
        "Albinius buys Heavy Old Tomes for 30 gold"
    );
    assert!(
        book.get("oteryn:npc.a_ghostly_guardian").is_some(),
        "retail Dream Realm conversation"
    );
    for shop in ["flint", "romella"] {
        let entry = book.get(&format!("oteryn:npc.{shop}")).expect("wraps shop");
        for (item, price) in [(50293, 347_222), (50294, 1_250_000), (50295, 10_000_000)] {
            assert!(
                entry.offers().iter().any(|offer| {
                    offer.item.key == format!("oteryn:item.tibia.i{item}")
                        && offer.direction == ProjectV2ServiceOfferDirection::SellToPlayer
                        && offer.unit_price == price
                        && offer.count.unwrap_or(1) == 1
                }),
                "{shop}: full exercise wraps {item} must be executable"
            );
        }
    }
    assert!(
        book.get("oteryn:npc.pugwah")
            .expect("Pugwah")
            .offers()
            .iter()
            .any(|offer| { offer.item.key == "oteryn:item.tibia.i2880" && offer.unit_price == 4 }),
        "Pugwah sells empty mugs for four gold"
    );
}
