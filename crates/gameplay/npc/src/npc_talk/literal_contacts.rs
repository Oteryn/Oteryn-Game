//! Closed lore-only contacts; no quest, currency or exchange operations are admitted.
pub(super) const REVISION_MATERIAL: &str = "literal-contacts:v2;mayor_pocaro:literal;wayland_smythers:literal;minzy:name,job,story;rascalio:name,job;rascalio-greet:Be greeted, |PLAYERNAME|!";

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
