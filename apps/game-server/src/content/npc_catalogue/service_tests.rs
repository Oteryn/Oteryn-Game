#![allow(clippy::expect_used)]
use super::replies::NpcReplyTemplate;
use super::service::build;
use super::service::*;
use crate::content::{
    ProjectV2CandidateField, ProjectV2Declaration, ProjectV2DefinitionRef, ProjectV2Family,
    ProjectV2Identity, ProjectV2ServiceOffer, ProjectV2ServiceOfferDirection as Dir,
    ProjectV2TravelDestination, ProjectV2TravelRoute,
};
use std::collections::BTreeMap;

#[derive(Default)]
struct Facts(BTreeMap<&'static str, (bool, bool, bool, Option<u32>)>);
impl NpcItemFacts for Facts {
    fn known(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }
    fn stackable(&self, key: &str) -> bool {
        self.0.get(key).is_some_and(|f| f.0)
    }
    fn charged(&self, key: &str) -> bool {
        self.0.get(key).is_some_and(|f| f.1)
    }
    fn fluid(&self, key: &str) -> bool {
        self.0.get(key).is_some_and(|f| f.2)
    }
    fn timed_charges(&self, key: &str) -> Option<u32> {
        self.0.get(key).and_then(|f| f.3)
    }
}

fn facts() -> Facts {
    let mut f = Facts::default();
    f.0.insert("oteryn:item.stack", (true, false, false, None));
    f.0.insert("oteryn:item.plain", (false, false, false, None));
    f.0.insert("oteryn:item.rune", (false, true, false, None));
    f.0.insert("oteryn:item.fluid", (false, false, true, None));
    f.0.insert("oteryn:item.timed", (false, true, false, Some(500)));
    f
}

fn item(key: &str) -> ProjectV2DefinitionRef {
    ProjectV2DefinitionRef {
        family: ProjectV2Family::Item,
        key: key.to_owned(),
        revision: "definition-r1".to_owned(),
    }
}

fn reference(family: ProjectV2Family, key: &str) -> ProjectV2DefinitionRef {
    ProjectV2DefinitionRef {
        family,
        key: key.to_owned(),
        revision: "definition-r1".to_owned(),
    }
}

fn offer(key: &str, direction: Dir, price: u64, count: Option<u32>) -> ProjectV2ServiceOffer {
    ProjectV2ServiceOffer {
        item: item(key),
        direction,
        unit_price: price,
        currency: None,
        count,
        sub_type: None,
        parity_pending: false,
    }
}

fn identity(key: &str) -> ProjectV2Identity {
    ProjectV2Identity {
        key: key.to_owned(),
        revision: "definition-r1".to_owned(),
    }
}

fn service(
    key: &str,
    offers: Vec<ProjectV2ServiceOffer>,
    routes: Vec<ProjectV2TravelRoute>,
) -> (ProjectV2DefinitionRef, ProjectV2Declaration) {
    (
        reference(ProjectV2Family::Service, key),
        ProjectV2Declaration::Service {
            identity: identity(key),
            offers,
            recipes: Vec::new(),
            routes,
            fields: Vec::<ProjectV2CandidateField>::new(),
        },
    )
}

fn route(key: &str, price: u64) -> ProjectV2TravelRoute {
    ProjectV2TravelRoute {
        key: key.to_owned(),
        destination: ProjectV2TravelDestination {
            coordinate_frame: "global-target-2026-09-27".to_owned(),
            x: 1,
            y: 2,
            floor: 7,
        },
        price,
        premium: false,
        min_level: None,
    }
}

fn npc(
    key: &str,
    dialogue: Option<&str>,
    services: &[&str],
) -> (ProjectV2DefinitionRef, ProjectV2Declaration) {
    (
        reference(ProjectV2Family::Npc, key),
        ProjectV2Declaration::Npc {
            identity: identity(key),
            presentation: None,
            behavior: None,
            dialogue: dialogue.map(|d| reference(ProjectV2Family::Dialogue, d)),
            services: services
                .iter()
                .map(|s| reference(ProjectV2Family::Service, s))
                .collect(),
            fields: Vec::new(),
        },
    )
}

fn dialogue(key: &str) -> (ProjectV2DefinitionRef, ProjectV2Declaration) {
    (
        reference(ProjectV2Family::Dialogue, key),
        ProjectV2Declaration::Dialogue {
            identity: identity(key),
            greet: vec!["Hello.".to_owned()],
            farewell: Vec::new(),
            walkaway: Vec::new(),
            send_trade: Vec::new(),
            keywords: Vec::new(),
            voices: None,
            source_incomplete: vec![],
            fields: Vec::new(),
        },
    )
}

fn model(records: &[(ProjectV2DefinitionRef, ProjectV2Declaration)]) -> NpcServiceModel {
    let refs: Vec<_> = records.iter().map(|(r, d)| (r, d)).collect();
    build(&refs, "test", "test", &facts())
}

/// The held reason of the single offer of a one-service model, `None` when admitted.
fn reason_of(offer: ProjectV2ServiceOffer) -> Option<NpcOfferHeldReason> {
    let records = [service("oteryn:service.t", vec![offer], vec![])];
    let m = model(&records);
    let s = m.services().values().next().expect("service");
    assert_eq!(s.offers.len() + s.held_offers.len(), 1);
    s.held_offers.first().map(|h| h.reason)
}

#[test]
fn non_gold_currency_is_held() {
    let mut o = offer("oteryn:item.plain", Dir::SellToPlayer, 5, None);
    o.currency = Some(item("oteryn:item.token"));
    assert_eq!(reason_of(o), Some(NpcOfferHeldReason::NonGoldCurrency));
}

#[test]
fn unknown_item_is_held() {
    let o = offer("oteryn:item.missing", Dir::SellToPlayer, 5, None);
    assert_eq!(reason_of(o), Some(NpcOfferHeldReason::UnknownItem));
}

#[test]
fn stackable_count_boundaries() {
    for (count, held) in [(0, true), (1, false), (100, false), (101, true)] {
        let o = offer("oteryn:item.stack", Dir::SellToPlayer, 5, Some(count));
        let want = held.then_some(NpcOfferHeldReason::CountOutOfRange);
        assert_eq!(reason_of(o), want, "count {count}");
    }
    let default = offer("oteryn:item.stack", Dir::SellToPlayer, 5, None);
    assert_eq!(reason_of(default), None);
}

#[test]
fn plain_item_count_must_be_one() {
    let o = offer("oteryn:item.plain", Dir::SellToPlayer, 5, Some(2));
    assert_eq!(reason_of(o), Some(NpcOfferHeldReason::CountOutOfRange));
}

#[test]
fn charged_and_fluid_count_is_charges() {
    for key in ["oteryn:item.rune", "oteryn:item.fluid"] {
        for (count, held) in [(0, true), (1, false), (65_535, false), (65_536, true)] {
            let o = offer(key, Dir::SellToPlayer, 5, Some(count));
            let want = held.then_some(NpcOfferHeldReason::CountOutOfRange);
            assert_eq!(reason_of(o), want, "{key} count {count}");
        }
    }
}

#[test]
fn timed_count_equals_charges() {
    for (count, held) in [
        (Some(500), false),
        (Some(499), true),
        (Some(501), true),
        (None, true),
    ] {
        let o = offer("oteryn:item.timed", Dir::SellToPlayer, 5, count);
        let want = held.then_some(NpcOfferHeldReason::TimedCountMismatch);
        assert_eq!(reason_of(o), want, "count {count:?}");
    }
}

#[test]
fn sell_price_capacity_boundary_applies_to_selling_only() {
    for (price, held) in [(1_009_999, false), (1_010_000, true)] {
        let o = offer("oteryn:item.plain", Dir::SellToPlayer, price, None);
        let want = held.then_some(NpcOfferHeldReason::SellPriceAboveCoinCapacity);
        assert_eq!(reason_of(o), want, "price {price}");
    }
    let buy = offer("oteryn:item.plain", Dir::BuyFromPlayer, 1_010_000, None);
    assert_eq!(reason_of(buy), None);
}

#[test]
fn parity_pending_is_held() {
    let mut o = offer("oteryn:item.plain", Dir::SellToPlayer, 5, None);
    o.parity_pending = true;
    assert_eq!(reason_of(o), Some(NpcOfferHeldReason::ParityPending));
}

#[test]
fn arbitrage_holds_only_paying_offers_per_unit() {
    // The lowest sell is 100 gold for 10 units: 10 per unit, from the second NPC.
    for (buy_price, buy_count, held) in [
        (10, Some(1), false),   // equal
        (9, Some(1), false),    // below
        (11, Some(1), true),    // above
        (100, Some(10), false), // equal per unit
        (101, Some(10), true),  // above per unit
    ] {
        let records = [
            service(
                "oteryn:service.dear",
                vec![offer("oteryn:item.stack", Dir::SellToPlayer, 50, Some(1))],
                vec![],
            ),
            service(
                "oteryn:service.cheap",
                vec![offer("oteryn:item.stack", Dir::SellToPlayer, 100, Some(10))],
                vec![],
            ),
            service(
                "oteryn:service.buyer",
                vec![offer(
                    "oteryn:item.stack",
                    Dir::BuyFromPlayer,
                    buy_price,
                    buy_count,
                )],
                vec![],
            ),
        ];
        let m = model(&records);
        let buyer = &m.services()[&reference(ProjectV2Family::Service, "oteryn:service.buyer")];
        assert_eq!(
            buyer.held_offers.len(),
            usize::from(held),
            "{buy_price}/{buy_count:?}"
        );
        // Selling offers always stay.
        let cheap = &m.services()[&reference(ProjectV2Family::Service, "oteryn:service.cheap")];
        assert_eq!(cheap.offers.len(), 1);
        if held {
            assert_eq!(
                buyer.held_offers[0].reason,
                NpcOfferHeldReason::ArbitragePaying
            );
        }
    }
}

#[test]
fn arbitrage_ignores_held_sell_offers_and_other_sub_types() {
    let mut other_sub_type = offer("oteryn:item.stack", Dir::SellToPlayer, 1, None);
    other_sub_type.sub_type = Some(3);
    let mut held_sell = offer("oteryn:item.stack", Dir::SellToPlayer, 1, None);
    held_sell.parity_pending = true;
    let records = [
        service("oteryn:service.a", vec![other_sub_type, held_sell], vec![]),
        service(
            "oteryn:service.b",
            vec![offer("oteryn:item.stack", Dir::BuyFromPlayer, 50, None)],
            vec![],
        ),
    ];
    let m = model(&records);
    assert_eq!(m.admitted_offer_count(), 2);
    assert_eq!(m.held_offer_counts()["ArbitragePaying"], 0);
}

#[test]
fn routes_all_load_and_free_routes_are_kept() {
    let records = [service(
        "oteryn:service.travel",
        vec![],
        vec![route("thais", 0), route("carlin", 110)],
    )];
    let m = model(&records);
    assert_eq!(m.admitted_route_count(), 2);
}

#[test]
fn held_npc_and_generated_replies() {
    let records = [
        service(
            "oteryn:service.trade",
            vec![offer("oteryn:item.plain", Dir::SellToPlayer, 5, None)],
            vec![],
        ),
        service(
            "oteryn:service.travel",
            vec![],
            vec![route("thais", 0), route("carlin", 110)],
        ),
        service("oteryn:service.empty", vec![], vec![]),
        dialogue("oteryn:dialogue.npc.talker"),
        npc(
            "oteryn:npc.talker",
            Some("oteryn:dialogue.npc.talker"),
            &["oteryn:service.trade"],
        ),
        npc(
            "oteryn:npc.a_boatman",
            None,
            &["oteryn:service.travel", "oteryn:service.empty"],
        ),
        npc(
            "oteryn:npc.lost_dialogue",
            Some("oteryn:dialogue.npc.missing"),
            &[],
        ),
        npc("oteryn:npc.lost_service", None, &["oteryn:service.missing"]),
    ];
    let m = model(&records);
    assert_eq!(m.npcs().len(), 2);
    assert_eq!(m.held_npcs().len(), 2);
    assert!(
        m.held_npcs()
            .values()
            .any(|r| *r == NpcHeldReason::UnresolvedDialogue)
    );
    assert!(
        m.held_npcs()
            .values()
            .any(|r| *r == NpcHeldReason::UnresolvedService)
    );
    let talker = &m.npcs()[&reference(ProjectV2Family::Npc, "oteryn:npc.talker")];
    assert!(talker.generated_replies.is_none());
    let boatman = &m.npcs()[&reference(ProjectV2Family::Npc, "oteryn:npc.a_boatman")];
    assert_eq!(boatman.name, "A Boatman");
    let replies = boatman.generated_replies.as_ref().expect("generated");
    assert_eq!(replies.greeting.text, "Greetings, I am A Boatman.");
    assert_eq!(replies.farewell.text, "Farewell.");
    // One line per referenced service; the empty service is retained with an idle line.
    assert_eq!(replies.services.len(), 2);
    assert_eq!(
        replies.services[0].1.text,
        "A Boatman can take you to thais for free, carlin for 110 gold."
    );
    assert_eq!(replies.services[1].1.template, NpcReplyTemplate::Idle);
    assert_eq!(m.generated_reply_npc_count(), 1);
}

#[test]
fn generated_reply_has_exactly_one_line_per_service() {
    let mut held = offer("oteryn:item.plain", Dir::SellToPlayer, 5, None);
    held.currency = Some(item("oteryn:item.token"));
    let records = [
        service(
            "oteryn:service.mixed",
            vec![offer("oteryn:item.plain", Dir::SellToPlayer, 5, None)],
            vec![route("thais", 0)],
        ),
        service("oteryn:service.empty", vec![], vec![]),
        service("oteryn:service.allheld", vec![held], vec![]),
        npc(
            "oteryn:npc.a_merchant",
            None,
            &[
                "oteryn:service.mixed",
                "oteryn:service.empty",
                "oteryn:service.allheld",
            ],
        ),
    ];
    let m = model(&records);
    let merchant = &m.npcs()[&reference(ProjectV2Family::Npc, "oteryn:npc.a_merchant")];
    assert_eq!(
        merchant.services.len(),
        3,
        "empty and held services retained"
    );
    let lines = &merchant
        .generated_replies
        .as_ref()
        .expect("generated")
        .services;
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].1.template, NpcReplyTemplate::TradeAndTravel);
    assert_eq!(
        lines[0].1.text,
        "A Merchant buys and sells goods and can take you to thais for free."
    );
    assert_eq!(lines[1].1.template, NpcReplyTemplate::Idle);
    assert_eq!(lines[2].1.template, NpcReplyTemplate::Idle);
}

#[test]
fn model_is_identical_when_built_twice_and_in_any_record_order() {
    let mut records = vec![
        service(
            "oteryn:service.a",
            vec![offer("oteryn:item.stack", Dir::SellToPlayer, 5, None)],
            vec![route("x", 1)],
        ),
        service(
            "oteryn:service.b",
            vec![offer("oteryn:item.stack", Dir::BuyFromPlayer, 9, None)],
            vec![],
        ),
        npc(
            "oteryn:npc.one",
            None,
            &["oteryn:service.a", "oteryn:service.b"],
        ),
    ];
    let first = model(&records);
    assert_eq!(first, model(&records));
    records.reverse();
    assert_eq!(first, model(&records));
}
