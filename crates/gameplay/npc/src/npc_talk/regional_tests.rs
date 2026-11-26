use super::*;

fn model() -> Result<NpcServiceModel, String> {
    crate::content::NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(|error| format!("{error:?}"))?
    .service_model_for_project()
    .map_err(|error| format!("{error:?}"))
}

#[test]
fn placed_c16_rune_shops_retain_their_catalogue_offers() -> Result<(), String> {
    let book = NpcTalkBook::from_model(&model()?);
    for (npc, item) in [
        ("oteryn:npc.alexander", "oteryn:item.tibia.i3203"),
        ("oteryn:npc.asima", "oteryn:item.tibia.i3177"),
        ("oteryn:npc.aurelia", "oteryn:item.tibia.i21351"),
        ("oteryn:npc.chartan", "oteryn:item.tibia.i3177"),
        ("oteryn:npc.fenech", "oteryn:item.tibia.i3177"),
        ("oteryn:npc.frederik", "oteryn:item.tibia.i3177"),
        ("oteryn:npc.ghorza", "oteryn:item.tibia.i3203"),
        ("oteryn:npc.gnomegica", "oteryn:item.tibia.i3177"),
        ("oteryn:npc.nelly", "oteryn:item.tibia.i3177"),
        ("oteryn:npc.tandros", "oteryn:item.tibia.i3177"),
        ("oteryn:npc.topsy", "oteryn:item.tibia.i3177"),
        ("oteryn:npc.xodet", "oteryn:item.tibia.i3177"),
    ] {
        let entry = book
            .get(npc)
            .ok_or_else(|| format!("{npc} missing from the talk book"))?;
        assert!(!entry.greeting_reply.is_empty(), "{npc} has no greeting");
        assert!(
            entry.offers().iter().any(|offer| {
                offer.item.key == item
                    && offer.direction
                        == crate::content::ProjectV2ServiceOfferDirection::SellToPlayer
                    && offer.currency.is_none()
            }),
            "{npc} does not sell {item}"
        );
    }
    for (npc, item) in [
        ("oteryn:npc.aurelia", "oteryn:item.tibia.i3178"),
        ("oteryn:npc.topsy", "oteryn:item.tibia.i8084"),
        ("oteryn:npc.xodet", "oteryn:item.tibia.i8084"),
    ] {
        let entry = book.get(npc).ok_or_else(|| format!("{npc} missing"))?;
        assert!(
            !entry.offers().iter().any(|offer| {
                offer.item.key == item
                    && offer.direction
                        == crate::content::ProjectV2ServiceOfferDirection::SellToPlayer
            }),
            "{npc} exposes a donor-only sale of {item}"
        );
    }
    Ok(())
}

#[test]
fn regional_source_conversations_open_focus_and_keep_services() -> Result<(), String> {
    let book = NpcTalkBook::from_model(&model()?);
    let source: serde_json::Value =
        serde_json::from_str(regional_contacts::DATA).map_err(|error| error.to_string())?;
    let contacts = source["contacts"].as_object().ok_or("regional contacts")?;
    let missing: Vec<_> = contacts
        .keys()
        .filter(|npc| book.get(npc).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "regional NPCs missing from the talk book: {missing:?}"
    );

    let greetings = [
        (
            "armenius",
            "Ah, an adventurer. Be greeted and have a seat. How may I {serve} you?",
        ),
        (
            "esrik",
            "Hello, |PLAYERNAME| and welcome to my little forge.",
        ),
        ("rock_steady", "Greetings, |PLAYERNAME|."),
        (
            "azalea",
            "Welcome, young |PLAYERNAME|! If you are heavily wounded or poisoned, I can {heal} you for free.",
        ),
        ("sandomo", "Hm. Greetings."),
        ("taya", "Suon's blessing!"),
        ("tarisu", "May the tides always guide you safely."),
    ];
    let at = ChatPosition {
        x: 32858,
        y: 31325,
        floor: 7,
    };
    for (stem, expected) in greetings {
        let entry = book
            .get(&format!("oteryn:npc.{stem}"))
            .ok_or_else(|| format!("{stem} missing from NPC talk book"))?;
        let npcs = [TalkNpc {
            id: 1u8,
            position: at,
            entry,
        }];
        let mut engaged = None;
        let reply =
            talk(at, "hi", &npcs, &mut engaged).ok_or_else(|| format!("{stem} did not greet"))?;
        assert_eq!(reply.text, expected, "{stem}");
        assert_eq!(engaged, Some(1));
    }
    let sandomo = book.get("oteryn:npc.sandomo").ok_or("Sandomo")?;
    let npcs = [TalkNpc {
        id: 1u8,
        position: at,
        entry: sandomo,
    }];
    assert_eq!(
        talk(at, "supplies", &npcs, &mut Some(1))
            .ok_or("Sandomo supplies keyword")?
            .text,
        "Head to brother Maun if you are in need of basic supplies. He will help you - for a small fee."
    );
    let trade = book.get("oteryn:npc.maun").ok_or("Maun")?;
    assert!(!trade.offers.is_empty());
    assert!(book.get("oteryn:npc.eighty").ok_or("Eighty")?.bank());
    Ok(())
}

#[test]
fn tarak_uses_the_paid_travel_service_for_both_donor_fares() -> Result<(), String> {
    let book = NpcTalkBook::from_model(&model()?);
    for (stem, keyword, cost, cell) in [
        ("tarak_inner", "monument tower", 50, (32940, 31182, 7)),
        ("tarak_inner", "passage", 50, (32940, 31182, 7)),
        ("tarak_inner", "trip", 50, (32940, 31182, 7)),
        ("tarak_sunken", "passage", 0, (32916, 31199, 7)),
    ] {
        let npc = format!("oteryn:npc.{stem}");
        let entry = book
            .get(&npc)
            .ok_or_else(|| format!("{stem} missing from NPC talk book"))?;
        let route = entry
            .travel_routes
            .iter()
            .find(|route| route.route.key.replace('_', " ") == keyword && !route.held)
            .ok_or_else(|| format!("{stem} does not serve {keyword}"))?;
        let greeting = regional_contacts::get(&npc)
            .and_then(|contact| contact.greet.as_deref())
            .ok_or("Tarak source greeting")?;
        let destination = if stem == "tarak_inner" {
            greeting
                .split_once('{')
                .and_then(|(_, tail)| tail.split_once('}'))
        } else {
            greeting
                .split_once("back to ")
                .and_then(|(_, tail)| tail.split_once('?'))
        }
        .map(|(name, _)| name.to_ascii_lowercase())
        .ok_or("Tarak destination in source greeting")?;
        assert_eq!(route.keyword, destination, "{stem}/{keyword}");
        assert_eq!(route.route.price, cost);
        let destination = &route.route.destination;
        assert_eq!((destination.x, destination.y, destination.floor), cell);
    }
    Ok(())
}

#[test]
fn regional_shops_serve_crystal_prices_and_gate_farmine_stock() -> Result<(), String> {
    use crate::content::ProjectV2ServiceOfferDirection::{BuyFromPlayer, SellToPlayer};
    let model = model()?;
    let book = NpcTalkBook::from_model(&model);
    for (stem, id, direction, price) in [
        ("alaistar", 21103, BuyFromPlayer, 350),
        ("armenius", 3600, SellToPlayer, 3),
        ("asnarus", 3447, SellToPlayer, 2),
        ("dorbin", 2998, SellToPlayer, 50),
    ] {
        let entry = book
            .get(&format!("oteryn:npc.{stem}"))
            .ok_or_else(|| format!("missing {stem} shop"))?;
        let key = format!("oteryn:item.tibia.i{id}");
        assert!(
            entry.offers().iter().any(|offer| {
                offer.item.key == key && offer.direction == direction && offer.unit_price == price
            }),
            "{stem} must serve {key} at {price}"
        );
    }

    use crate::content::npc_catalogue::quest_shops;
    for (stem, item_id, price, minimum) in [("pompan", 10409, 700, 1), ("esrik", 10384, 14000, 9)] {
        let npc = format!("oteryn:npc.{stem}");
        let key = format!("oteryn:item.tibia.i{item_id}");
        let entry = book
            .get(&npc)
            .ok_or_else(|| format!("missing {stem} shop"))?;
        let offer = entry
            .offers()
            .iter()
            .find(|offer| offer.item.key == key && offer.direction == BuyFromPlayer)
            .ok_or_else(|| format!("{stem} must buy {key}"))?;
        assert_eq!(offer.unit_price, price);
        let shops = quest_shops::shops().map_err(str::to_owned)?;
        let required = quest_shops::required(shops, &npc, &offer.item, offer.direction, price, 1)
            .map_err(str::to_owned)?
            .ok_or_else(|| format!("{stem} must require tomes for {key}"))?;
        assert!(!quest_shops::eligible(required, |_| Some(minimum - 1)));
        assert!(quest_shops::eligible(required, |_| Some(minimum)));
        assert!(
            quest_shops::required(shops, &npc, &offer.item, offer.direction, price + 1, 1).is_err()
        );
    }
    Ok(())
}
