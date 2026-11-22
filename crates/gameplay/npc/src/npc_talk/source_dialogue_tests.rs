use super::*;

fn book() -> Result<NpcTalkBook, String> {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content"));
    let catalogue = crate::content::NpcDataCatalogue::load(
        root,
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(|error| format!("{error:?}"))?;
    Ok(NpcTalkBook::from_model(
        &catalogue
            .service_model_for_project()
            .map_err(|error| format!("{error:?}"))?,
    ))
}

#[test]
fn placed_npcs_open_source_conversations() -> Result<(), String> {
    let book = book()?;
    let contacts = [
        ("a_seagull", "hi", "Squawk!"),
        (
            "alexander",
            "hi",
            "Hi there |PLAYERNAME|, and welcome to the {magic} store.",
        ),
        (
            "anna",
            "hi",
            "Hello, my friend. Do you seek {passage} to the island {Targuna}?",
        ),
        (
            "aurelia",
            "hi",
            "Welcome to my {shop}! Are you looking for a {trade}?",
        ),
        ("asima", "hi", "Greetings, |PLAYERNAME|."),
        (
            "billy",
            "hi",
            "Howdy |PLAYERNAME|. I'm a farmer and cook, maybe I can interest you in a {trade} with food? You can also ask me for general {hints} about the game.",
        ),
        ("blind_orc", "charach", "Ikem Charach maruk."),
        (
            "carlos",
            "hi",
            "Be greeted, |PLAYERNAME|! As a tailor and merchant I have to say - we need to do something about your {outfit}, shall we?",
        ),
        ("dixi", "hi", "Greetings, |PLAYERNAME|."),
        (
            "hyacinth",
            "hi",
            "Greetings, traveller |PLAYERNAME|. As you have found the way to my hut, how can I {help} you?",
        ),
        (
            "lily",
            "hi",
            "Welcome, |PLAYERNAME|! You look a little stressed today. If you like to view my offers of potions, just ask me for a {trade}. In case you're looking for the marketplace and dungeons, just follow the path to the east!",
        ),
        (
            "obi",
            "hi",
            "Hello, hello, |PLAYERNAME|! Please come in, look, and buy! If you like to see my offers, ask me for a {trade}!",
        ),
        (
            "santiago",
            "hi",
            "Hello |PLAYERNAME|, nice to see you on Rookgaard! I saw you walking by and wondered if you could help me. Could you? Please, say {yes}!",
        ),
        (
            "willie",
            "hi",
            "Hiho |PLAYERNAME|. I hope you're here to {trade}.",
        ),
        ("zarifan", "hi", "... ... hello...magic... words?"),
        ("zethra", "hi", "Greetings, |PLAYERNAME|."),
        (
            "zirella",
            "hi",
            "Oh, heaven must have sent you! Could you please help me with a {quest}?",
        ),
    ];
    for (key, word, greeting) in contacts {
        let id = format!("oteryn:npc.{key}");
        let entry = book.get(&id).ok_or_else(|| format!("missing {id}"))?;
        let position = ChatPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        let npc = [TalkNpc {
            id: 1u8,
            position,
            entry,
        }];
        let mut focused = None;
        assert_eq!(
            talk(position, word, &npc, &mut focused)
                .map(|reply| reply.text)
                .as_deref(),
            Some(greeting),
            "{id}"
        );
        assert_eq!(focused, Some(1), "{id}");
        if key == "blind_orc" {
            assert!(talk(position, "bye", &npc, &mut focused).is_none());
            let farewell = talk(position, "futchi", &npc, &mut focused).ok_or("Orc farewell")?;
            assert_eq!(farewell.text, "Futchi!");
            assert_eq!(focused, None);
            assert!(talk(position, "hi", &npc, &mut focused).is_none());
        }
    }
    Ok(())
}

#[test]
fn placed_npcs_answer_source_keywords() -> Result<(), String> {
    let book = book()?;
    let entry = book.get("oteryn:npc.al_dee").ok_or("Al Dee")?;
    let position = ChatPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    let npc = [TalkNpc {
        id: 1u8,
        position,
        entry,
    }];
    let mut focused = None;
    assert!(talk(position, "hi", &npc, &mut focused).is_some());
    assert_eq!(
        talk(position, "name", &npc, &mut focused).map(|reply| reply.text),
        Some("My name is Al Dee, but you can call me Al. Can I interest you in a {trade}?".into()),
    );
    Ok(())
}

#[test]
fn crystal_return_ferries_resolve_source_routes() -> Result<(), String> {
    let book = book()?;
    for (npc, keyword, fare, destination) in [
        ("captain_jack", "tibia", 0, (32204, 31756, 6)),
        ("eremo", "cormaya", 0, (33288, 31956, 6)),
        ("ferryman_kamil", "meluna", 0, (32088, 32470, 7)),
        ("ferryman_kamil_meluna", "fibula", 0, (32153, 32457, 7)),
        ("harlow_vengoth", "yalahar", 0, (32837, 31366, 7)),
        ("hawkhurst", "passage", 400, (33710, 32602, 6)),
        ("hawkhurst_ingol", "passage", 400, (33356, 31983, 7)),
        ("kendra", "thais", 180, (32259, 32262, 7)),
        ("tonar", "passage", 0, (33070, 32916, 7)),
    ] {
        let key = format!("oteryn:npc.{npc}");
        let entry = book.get(&key).ok_or_else(|| format!("missing {key}"))?;
        let route = entry
            .travel_routes
            .iter()
            .find(|route| !route.held && route.keyword == keyword)
            .ok_or_else(|| format!("missing {key}/{keyword}"))?;
        assert_eq!(route.route.price, fare, "{key}");
        assert_eq!(route.route.premium, npc == "kendra", "{key}");
        assert_eq!(route.postman, matches!(npc, "kendra" | "tonar"), "{key}");
        if npc == "eremo" {
            for alias in ["back", "passage", "pemaret"] {
                assert!(route.matches_keyword(alias), "{alias}");
            }
            assert!(!route.matches_keyword("yes"));
        }
        let target = &route.route.destination;
        assert_eq!((target.x, target.y, target.floor), destination, "{key}");
    }
    Ok(())
}

#[test]
fn emael_greets_with_both_lines_in_source_order() -> Result<(), String> {
    let book = book()?;
    let entry = book.get("oteryn:npc.emael").ok_or("Emael")?;
    let position = ChatPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    let npc = [TalkNpc {
        id: 1u8,
        position,
        entry,
    }];
    let mut focused = None;
    let greeting = talk(position, "hi", &npc, &mut focused).ok_or("Emael greeting")?;
    assert_eq!(
        greeting.into_lines().collect::<Vec<_>>(),
        [
            "Hello! Ever asked yourself who killed all the monsters for the wall trophies? Yeah, that was me, Emael the Beasthunter! I am an expert in displaying trophies. ...",
            "So if you have at least some dangerous monster to show off I strongly advise you to aquire a {podium} of vigour.",
        ]
    );
    assert_eq!(focused, Some(1));
    assert!(talk(position, "yes", &npc, &mut focused).is_none());
    assert_eq!(
        talk(position, "bye", &npc, &mut focused)
            .map(|reply| reply.text)
            .as_deref(),
        Some("I wish you a good hunt. Goodbye!")
    );
    assert_eq!(focused, None);
    Ok(())
}

pub(super) fn assert_reward_lore(name: &str, entry: &NpcTalkEntry) {
    let expected: &[&str] = match name {
        "morgan" => &["addon", "weapons"],
        "sam" => &["job"],
        "zoltan" => &["spells", "support", "attack"],
        "rafzan" => &[
            "task", "name", "goblin", "human", "profit", "swamp", "dwarf", "help", "thais",
            "elves", "job", "venore", "gold", "ratmen",
        ],
        _ => &[],
    };
    let topics: Vec<_> = entry
        .dialogue_keywords
        .iter()
        .map(|keyword| keyword.key.as_str())
        .collect();
    assert_eq!(topics, expected, "only source lore opens for {name}");
}
