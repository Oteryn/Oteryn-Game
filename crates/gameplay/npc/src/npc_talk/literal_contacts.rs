//! Closed literal dialogue bindings for existing NPC contacts and services.
use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::content::ProjectV2DialogueKeyword;
use crate::content::npc_catalogue::service::NpcBankDialogue;

pub(super) const REVISION_MATERIAL: &str = "literal-contacts:v2;mayor_pocaro:literal;wayland_smythers:literal;minzy:name,job,story;rascalio:name,job;rascalio-greet:Be greeted, |PLAYERNAME|!";
pub(super) const REGIONAL_SOURCE: &str =
    include_str!("../../../../../content/npcs/six-region-talk.json");

#[derive(serde::Deserialize)]
struct RegionalDocument {
    schema: String,
    source_revision: String,
    contacts: BTreeMap<String, RegionalContact>,
}

#[derive(serde::Deserialize)]
struct RegionalContact {
    greet: ReplySource,
    farewell: ReplySource,
    walkaway: Option<ReplySource>,
    keywords: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReplySource {
    Catalogue,
    HandlerDefault,
}

impl ReplySource {
    fn message(&self, lines: &[String], handler_default: &str) -> Option<String> {
        match self {
            Self::Catalogue => {
                let [line] = lines else { return None };
                Some(line.clone())
            }
            Self::HandlerDefault => Some(handler_default.to_owned()),
        }
    }
}

pub(super) struct RegionalMessages {
    pub(super) greet: String,
    pub(super) farewell: String,
    pub(super) walkaway: Option<String>,
    pub(super) keywords: Vec<ProjectV2DialogueKeyword>,
}

pub(super) fn regional_messages(key: &str, dialogue: &NpcBankDialogue) -> Option<RegionalMessages> {
    static REGIONAL: OnceLock<Option<RegionalDocument>> = OnceLock::new();
    let document = REGIONAL
        .get_or_init(|| {
            let document: RegionalDocument = serde_json::from_str(REGIONAL_SOURCE).ok()?;
            (document.schema == "OTERYN_REGIONAL_NPC_TALK/v1"
                && document.source_revision == "201189f871d23597b82caf88fd611454a2ddefb8")
                .then_some(document)
        })
        .as_ref()?;
    let source = document.contacts.get(key)?;
    Some(RegionalMessages {
        greet: source
            .greet
            .message(&dialogue.greet, "Greetings, |PLAYERNAME|.")?,
        farewell: source
            .farewell
            .message(&dialogue.farewell, "Good bye, |PLAYERNAME|.")?,
        walkaway: source
            .walkaway
            .as_ref()
            .and_then(|reply| reply.message(&dialogue.walkaway, "Good bye.")),
        keywords: dialogue
            .keywords
            .iter()
            .filter(|keyword| {
                super::literal_keyword(keyword) && source.keywords.contains(&keyword.key)
            })
            .cloned()
            .collect(),
    })
}

pub(super) fn admitted(key: &str) -> bool {
    matches!(
        key,
        "oteryn:npc.mayor_pocaro"
            | "oteryn:npc.wayland_smythers"
            | "oteryn:npc.minzy"
            | "oteryn:npc.rascalio"
    )
}

pub(super) fn keyword_allowed(
    key: &str,
    keyword: &crate::content::ProjectV2DialogueKeyword,
) -> bool {
    admitted(key)
        && super::literal_keyword(keyword)
        && match key {
            "oteryn:npc.minzy" => matches!(keyword.key.as_str(), "name" | "job" | "story"),
            "oteryn:npc.rascalio" => matches!(keyword.key.as_str(), "name" | "job"),
            _ => true,
        }
}

pub(super) fn greeting(key: &str) -> Option<&'static str> {
    // Explicit Oteryn adaptation: the retained greeting asserts a Cook quest mission.
    // Its neutral prefix also suits the event occurrence, without inventing quest state.
    (key == "oteryn:npc.rascalio").then_some("Be greeted, |PLAYERNAME|!")
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "repository catalogue qualification")]
mod tests {
    use super::*;
    use crate::foundation::speech::ChatPosition;
    use crate::npc_talk::{NpcTalkBook, TalkNpc, talk};

    fn model() -> crate::content::NpcServiceModel {
        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content"));
        crate::content::NpcDataCatalogue::load(
            root,
            std::ffi::OsStr::new("world"),
            crate::content::npc_catalogue_preproduction_limits(),
        )
        .expect("catalogue")
        .service_model_for_project()
        .expect("model")
    }

    #[test]
    fn regional_guide_and_merchant_use_their_catalogue_dialogue_and_offers() {
        let model = model();
        let book = NpcTalkBook::from_model(&model);
        let at = ChatPosition {
            x: 1,
            y: 1,
            floor: 7,
        };
        for (key, keyword, trade) in [
            ("oteryn:npc.guide_elena", "temple", false),
            ("oteryn:npc.gorn", "job", true),
        ] {
            let source = model
                .npcs()
                .iter()
                .find(|(r, _)| r.key == key)
                .expect("source NPC")
                .1;
            let dialogue = source.bank_dialogue.as_ref().expect("source dialogue");
            let topic = dialogue
                .keywords
                .iter()
                .find(|k| k.key == keyword)
                .expect("source topic");
            let entry = book.get(key).expect("regional contact is admitted");
            let npcs = [TalkNpc {
                id: 1u8,
                position: at,
                entry,
            }];
            let mut engaged = None;
            assert_eq!(
                talk(at, "hi", &npcs, &mut engaged).expect("greet").text,
                dialogue.greet[0]
            );
            assert_eq!(
                talk(at, keyword, &npcs, &mut engaged).expect("topic").text,
                topic.reply[0]
            );
            let offers: Vec<_> = source
                .services
                .iter()
                .flat_map(|r| &model.services()[r].offers)
                .cloned()
                .collect();
            assert_eq!(entry.offers(), offers);
            assert_eq!(!entry.offers().is_empty(), trade);
        }
    }

    #[test]
    fn regional_merchants_charge_the_pinned_crystal_prices() {
        let model = model();
        let book = NpcTalkBook::from_model(&model);
        for (npc, item, price) in [
            ("oteryn:npc.allen", "oteryn:item.tibia.i32477", 40),
            ("oteryn:npc.bezil", "oteryn:item.tibia.i3308", 6),
            ("oteryn:npc.iwar", "oteryn:item.tibia.i32481", 40),
            ("oteryn:npc.nydala", "oteryn:item.tibia.i32490", 40),
            ("oteryn:npc.ukea", "oteryn:item.tibia.i32486", 40),
        ] {
            let offer = book
                .get(npc)
                .expect("regional merchant")
                .offers()
                .iter()
                .find(|offer| {
                    offer.item.key == item
                        && offer.direction
                            == crate::content::ProjectV2ServiceOfferDirection::SellToPlayer
                })
                .expect("donor shop item");
            assert_eq!(offer.unit_price, price, "{npc} sells {item}");
        }
    }

    #[test]
    fn m38_minzy_literal_contact_reads_published_dialogue_without_enabling_exchanges() {
        let model = model();
        let book = NpcTalkBook::from_model(&model);
        let entry = book
            .get("oteryn:npc.minzy")
            .expect("Minzy literal contact is not admitted");
        assert!(entry.admitted_dialogue);
        assert!(!entry.bank);
        assert!(entry.offers.is_empty());
        assert!(entry.travel_routes.is_empty());
        assert_eq!(entry.dialogue_keywords.len(), 3);
        let at = ChatPosition {
            x: 32742,
            y: 31971,
            floor: 12,
        };
        let npcs = [TalkNpc {
            id: 1u8,
            position: ChatPosition {
                x: 32743,
                y: 31971,
                floor: 12,
            },
            entry,
        }];
        let mut engaged = None;
        assert_eq!(
            talk(at, "hi", &npcs, &mut engaged).expect("greeting").text,
            "Welcome. Good you're showing up"
        );
        assert_eq!(
            talk(at, "name", &npcs, &mut engaged).expect("name").text,
            "I am known as Minzy."
        );
    }

    #[test]
    fn m38_literal_contacts_hold_unknown_and_stateful_topics() {
        let model = model();
        let mut topic = model
            .npcs()
            .iter()
            .find(|(r, _)| r.key == "oteryn:npc.minzy")
            .expect("Minzy")
            .1
            .bank_dialogue
            .as_ref()
            .expect("source dialogue")
            .keywords[0]
            .clone();
        topic.key = "favour".into();
        topic.triggers = vec!["favour".into()];
        for key in ["oteryn:npc.minzy", "oteryn:npc.rascalio"] {
            assert!(!keyword_allowed(key, &topic));
        }
        topic.key = "name".into();
        topic.reset = true;
        for key in ["oteryn:npc.minzy", "oteryn:npc.rascalio"] {
            assert!(!keyword_allowed(key, &topic));
        }
    }

    #[test]
    fn m38_rascalio_event_contact_answers_literal_topics_without_granting_travel() {
        let model = model();
        let book = NpcTalkBook::from_model(&model);
        let entry = book
            .get("oteryn:npc.rascalio")
            .expect("Rascalio event contact is admitted");
        assert!(entry.admitted_dialogue);
        assert!(!entry.bank);
        assert!(entry.offers.is_empty());
        assert!(entry.travel_routes.is_empty());
        assert_eq!(
            entry.render_source_text(&entry.greeting_reply, "Quest witness"),
            "Be greeted, Quest witness!"
        );
        let at = ChatPosition {
            x: 32258,
            y: 31643,
            floor: 7,
        };
        let npcs = [TalkNpc {
            id: 1u8,
            position: ChatPosition {
                x: 32259,
                y: 31643,
                floor: 7,
            },
            entry,
        }];
        let mut engaged = None;
        assert_eq!(
            talk(at, "hi", &npcs, &mut engaged).expect("greeting").text,
            "Be greeted, |PLAYERNAME|!"
        );
        assert_eq!(
            talk(at, "name", &npcs, &mut engaged).expect("name").text,
            "I am known as Rascalio."
        );
        assert_eq!(
            talk(at, "job", &npcs, &mut engaged).expect("job").text,
            "My role here is Ferryman."
        );
        assert!(talk(at, "passage", &npcs, &mut engaged).is_none());
    }
}
