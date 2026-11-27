#![cfg(target_os = "linux")]
#![allow(clippy::expect_used, clippy::panic)]
use oteryn_game_server::content::*;
use std::{collections::BTreeMap, ffi::OsStr, path::PathBuf};

fn build() -> NpcDataCatalogue {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/world");
    let project = capture_world_project(
        root.parent().expect("parent"),
        OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .expect("validated repository source");
    NpcDataCatalogue::from_project(project, npc_catalogue_preproduction_limits().project)
        .expect("data import")
}

// Regenerate strict count pins: python tools/contracts/refresh_npc_service_counts.py
#[test]
fn pinned_catalogue_service_model_counts() {
    let catalogue = build();
    let model = catalogue
        .service_model_for_project()
        .expect("service model");
    let again = catalogue
        .service_model_for_project()
        .expect("service model");
    assert_eq!(model, again, "built twice");
    assert_eq!(model.source_tree_digest(), catalogue.source_tree_digest());
    let counts: BTreeMap<&str, usize> = model.held_offer_counts();
    // The repository refresh tool captures every count before any assertion stops the test.
    if std::env::var_os("OTERYN_CAPTURE_NPC_SERVICE_COUNTS").is_some() {
        eprintln!(
            "NPC_SERVICE_COUNTS {}",
            serde_json::json!({
                "npcs": model.npcs().len(), "held_npcs": model.held_npcs().len(),
                "services": model.services().len(), "offers": model.admitted_offer_count(),
                "routes": model.admitted_route_count(), "replies": model.generated_reply_npc_count(),
                "held": counts, "source_tree_digest": model.source_tree_digest(),
                "service_keys": model.services().keys().map(|r| r.key.as_str()).collect::<Vec<_>>(),
            })
        );
    }
    assert_eq!(model.npcs().len(), 1_292);
    assert_eq!(model.held_npcs().len(), 0);
    // NPC-ECON-1 Bank, PREM-5's services, BANK-3's Henricus blessing offer, and #2810's
    // Grizzly Adams rank shop, S10 services, exact NPC-TRADE-3 subtype restoration,
    // and the four source-owned Djinn shops.
    assert_eq!(model.services().len(), 410);
    assert_eq!(model.admitted_offer_count(), 13_699);
    assert_eq!(model.admitted_route_count(), 199);
    assert_eq!(model.generated_reply_npc_count(), 437);
    let expected: BTreeMap<&str, usize> = BTreeMap::from([
        // Exact subtype qualification preserves the existing arbitrage protection.
        ("ArbitragePaying", 0),
        // The Nah'Bob i3284 offer lacks a native subtype; i3083 lacks full-charge proof.
        ("ChargedOrFluidPending", 1),
        ("CountOutOfRange", 0),
        ("NonGoldCurrency", 36),
        ("ParityPending", 0),
        ("SellPriceAboveCoinCapacity", 17),
        ("TimedCountMismatch", 9),
        ("UnknownItem", 0),
    ]);
    assert_eq!(counts, expected);
}

#[test]
fn regional_npc_service_model_loads_crystal_dialogue() -> Result<(), Box<dyn std::error::Error>> {
    let catalogue = build();
    let model = catalogue.service_model_for_project()?;
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../content/dialogues/regional.authoring.json"
    ))?;
    let entries = source["npcs"].as_object().ok_or("regional NPC dialogue")?;
    for (name, expected) in entries {
        let key = format!("oteryn:npc.{name}");
        let (reference, npc) = model
            .npcs()
            .iter()
            .find(|(reference, _)| reference.key == key)
            .ok_or_else(|| format!("missing NPC {key}"))?;
        let dialogue = npc
            .dialogue
            .as_ref()
            .ok_or_else(|| format!("missing donor dialogue on {key}"))?;
        assert_eq!(dialogue.revision, reference.revision);
        let Some(ProjectV2Declaration::Dialogue {
            greet,
            farewell,
            walkaway,
            send_trade,
            keywords,
            ..
        }) = catalogue.declaration(dialogue)
        else {
            return Err(format!("unresolved dialogue on {key}").into());
        };
        for (message, actual) in [
            ("greet", greet),
            ("farewell", farewell),
            ("walkaway", walkaway),
            ("send_trade", send_trade),
        ] {
            if let Some(lines) = expected["fields"].get(message) {
                assert_eq!(serde_json::to_value(actual)?, *lines, "{key} {message}");
            }
        }
        for entry in expected["keywords"].as_array().ok_or("regional keywords")? {
            let keyword = keywords
                .iter()
                .find(|keyword| entry["key"] == keyword.key)
                .ok_or_else(|| format!("missing {key} keyword {}", entry["key"]))?;
            assert_eq!(serde_json::to_value(&keyword.triggers)?, entry["triggers"]);
            assert_eq!(serde_json::to_value(&keyword.reply)?, entry["reply"]);
        }
    }
    Ok(())
}

#[test]
fn regional_npc_talk_opens_crystal_literal_topics() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_gameplay_entities::speech::ChatPosition;
    use oteryn_gameplay_npc::npc_talk::{NpcTalkBook, TalkNpc, talk};

    let catalogue = build();
    let model = catalogue.service_model_for_project()?;
    let book = NpcTalkBook::from_model(&model);
    let entry = book
        .get("oteryn:npc.a_bearded_woman")
        .ok_or("A Bearded Woman has no talk entry")?;
    let position = ChatPosition {
        x: 32594,
        y: 31615,
        floor: 10,
    };
    let actor = [TalkNpc {
        id: 1u8,
        position,
        entry,
    }];
    let mut focused = None;
    assert_eq!(
        talk(position, "hi", &actor, &mut focused)
            .ok_or("greet")?
            .text,
        "GET ME OUT OF HERE! NOW!"
    );
    assert_eq!(
        talk(position, "job", &actor, &mut focused)
            .ok_or("keyword")?
            .text,
        "I am a great and famous actor! Not a princess, at all. I was only PRETENDING to be a princess. But try explaining that to those stupid pirates."
    );
    assert!(entry.offers().is_empty());
    assert!(talk(position, "mission", &actor, &mut focused).is_none());
    let brewmaster = book
        .get("oteryn:npc.brewmaster_bhaan")
        .ok_or("Brewmaster Bhaan has no talk entry")?;
    let actor = [TalkNpc {
        id: 2u8,
        position,
        entry: brewmaster,
    }];
    assert_eq!(
        talk(position, "hi", &actor, &mut focused)
            .ok_or("greet")?
            .text,
        "Welcome, welcome! Come, try my latest {brew}!"
    );
    assert_eq!(
        talk(position, "brew", &actor, &mut focused)
            .ok_or("brew")?
            .text,
        "Brewing is both an art and a science. The perfect balance of ingredients, temperature, and time."
    );
    assert_eq!(brewmaster.walkaway(), "Come back when you're thirsty!");
    let hardek = book
        .get("oteryn:npc.hardek")
        .ok_or("Hardek has no trade entry")?;
    assert!(hardek.offers().iter().any(|offer| {
        offer.item.key == "oteryn:item.tibia.i3274"
            && offer.direction == ProjectV2ServiceOfferDirection::SellToPlayer
            && offer.unit_price == 20
    }));
    Ok(())
}

/// Crystal 201189f8, data-global/npc/captain_max.lua:59-78.
#[test]
fn captain_max_calassa_keeps_mission_and_postman_access_without_premium()
-> Result<(), Box<dyn std::error::Error>> {
    let catalogue = build();
    let model = catalogue.service_model_for_project()?;
    let (_, npc) = model
        .npcs()
        .iter()
        .find(|(key, _)| key.key == "oteryn:npc.captain_max")
        .ok_or("Captain Max in the production catalogue")?;
    let routes = npc_catalogue::travel::resolve(&model, "oteryn:npc.captain_max", &npc.services);
    let calassa = routes
        .iter()
        .find(|route| route.keyword == "calassa")
        .ok_or("Calassa service route")?;

    assert_eq!(calassa.route.price, 200);
    assert_eq!(
        (
            calassa.route.destination.x,
            calassa.route.destination.y,
            calassa.route.destination.floor,
        ),
        (31911, 32710, 6)
    );
    assert!(calassa.postman);
    assert!(!calassa.held);
    assert_eq!(
        calassa.quest_gate.as_deref(),
        Some("oteryn:quest-progress/quest/u7_6/explorer_society/calassa_quest")
    );
    assert!(!calassa.route.premium);
    Ok(())
}

#[test]
fn regional_variant_names_come_from_the_donor_npc_configuration()
-> Result<(), Box<dyn std::error::Error>> {
    let catalogue = build();
    let model = catalogue.service_model_for_project()?;
    let names: BTreeMap<_, _> = model
        .npcs()
        .iter()
        .map(|(key, npc)| (key.key.as_str(), npc.name.as_str()))
        .collect();
    for (key, name) in [
        ("oteryn:npc.anaztassja_moroia_init", "Anaztassja Moroia"),
        ("oteryn:npc.baa_leal", "Baa'Leal"),
        ("oteryn:npc.bo_ques", "Bo'Ques"),
        ("oteryn:npc.buddel_helheim", "Buddel"),
        ("oteryn:npc.buddel_okolnir", "Buddel"),
        ("oteryn:npc.buddel_raider_camp", "Buddel"),
        ("oteryn:npc.buddel_tyrsung", "Buddel"),
        ("oteryn:npc.captain_max_calassa", "Captain Max"),
        ("oteryn:npc.fa_hradin", "Fa'Hradin"),
        ("oteryn:npc.gnomish_operative_catching", "Gnomish Operative"),
        ("oteryn:npc.gnomish_operative_gt_m_01", "Gnomish Operative"),
        ("oteryn:npc.gnomish_operative_raiders", "Gnomish Operative"),
        (
            "oteryn:npc.gnomish_operative_resonating",
            "Gnomish Operative",
        ),
        ("oteryn:npc.gnomish_operative_volcano", "Gnomish Operative"),
        ("oteryn:npc.jack_fate_goroma", "Jack Fate"),
        ("oteryn:npc.john_bounac", "John"),
        ("oteryn:npc.lesh_the_seer", "Lesh the Seer"),
        ("oteryn:npc.nah_bob", "Nah'Bob"),
        ("oteryn:npc.one_eyed_joe", "One-Eyed Joe"),
        ("oteryn:npc.rata_mari", "Rata'Mari"),
        ("oteryn:npc.sebastian_nargor", "Sebastian"),
    ] {
        assert_eq!(names.get(key), Some(&name), "{key}");
    }
    Ok(())
}

#[test]
fn crystal_global_contacts_preserve_names_dialogue_and_victors_offers() {
    let model = build().service_model_for_project().expect("service model");
    let npc = |key: &str| {
        model
            .npcs()
            .iter()
            .find_map(|(reference, npc)| (reference.key == key).then_some(npc))
            .expect("admitted Crystal NPC")
    };
    assert_eq!(npc("oteryn:npc.ellipses").name, "...");
    assert_eq!(
        npc("oteryn:npc.eustacios_spy_rat").name,
        "Eustacio's Spy Rat"
    );
    assert!(npc("oteryn:npc.doctor_marrow").dialogue.is_some());
    assert!(npc("oteryn:npc.harlow").dialogue.is_some());
    let [service] = npc("oteryn:npc.victor").services.as_slice() else {
        panic!("Victor has one shop");
    };
    let shop = &model.services()[service];
    let offers: BTreeMap<_, _> = shop
        .offers
        .iter()
        .map(|offer| {
            assert_eq!(
                offer.direction,
                ProjectV2ServiceOfferDirection::SellToPlayer
            );
            (offer.item.key.as_str(), offer.unit_price)
        })
        .collect();
    assert_eq!(
        offers,
        BTreeMap::from([
            ("oteryn:item.tibia.i236", 150),
            ("oteryn:item.tibia.i239", 300),
            ("oteryn:item.tibia.i266", 75),
            ("oteryn:item.tibia.i7643", 500),
        ])
    );
    assert_eq!(shop.held_offers.len(), 0);
}
