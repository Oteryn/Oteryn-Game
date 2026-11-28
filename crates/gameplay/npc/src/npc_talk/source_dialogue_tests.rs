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
fn p20_king_tibianus_accepts_royal_greetings_only() -> Result<(), String> {
    let book = book()?;
    let entry = book
        .get("oteryn:npc.king_tibianus")
        .ok_or("King Tibianus")?;
    let position = ChatPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    let npcs = [TalkNpc {
        id: 1u8,
        position,
        entry,
    }];
    for plain in ["hi", "hello", "hail"] {
        let mut focused = None;
        assert!(
            talk(position, plain, &npcs, &mut focused).is_none(),
            "{plain}"
        );
        assert_eq!(focused, None);
    }
    // https://tibia.fandom.com/wiki/King_Tibianus/Transcripts
    for royal in ["Hail King!", "Hello King", "Hail to the beloved King"] {
        let mut focused = None;
        let reply = talk(position, royal, &npcs, &mut focused).ok_or(royal)?;
        assert_eq!(reply.text, "I greet thee, my loyal subject.");
        assert_eq!(focused, Some(1));
    }
    Ok(())
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
fn crystal_keywords_match_stems_and_first_occurrences_in_source_order() -> Result<(), String> {
    let book = book()?;
    let position = ChatPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    for (key, greet, request, expected) in [
        (
            "al_dee",
            "hi",
            "Cookies",
            Some(
                "I you want to find someone who may want to buy your cookies, you should meet Lily.",
            ),
        ),
        ("blind_orc", "charach", "goshak ikem", None),
        ("blind_orc", "charach", "goshak ikem goshak", None),
        (
            "blind_orc",
            "charach",
            "ikem ... goshak",
            Some("Ikem pashak porak, bata, dora. Ba goshak maruk?"),
        ),
    ] {
        let entry = book.get(&format!("oteryn:npc.{key}")).ok_or(key)?;
        let npc = [TalkNpc {
            id: 1u8,
            position,
            entry,
        }];
        let mut focused = None;
        assert!(talk(position, greet, &npc, &mut focused).is_some());
        assert_eq!(
            talk(position, request, &npc, &mut focused)
                .map(|reply| reply.text)
                .as_deref(),
            expected,
            "{key}: {request}"
        );
    }
    Ok(())
}

#[test]
fn crystal_ferries_have_no_extra_passenger_prerequisites() -> Result<(), String> {
    let book = book()?;
    for (npc, keyword) in [
        ("captain_jack", "tibia"),
        ("ferryman_kamil", "meluna"),
        ("ferryman_kamil_meluna", "fibula"),
        ("harlow_vengoth", "yalahar"),
        ("hawkhurst", "passage"),
        ("hawkhurst_ingol", "passage"),
        ("kendra", "thais"),
        ("tonar", "passage"),
    ] {
        let key = format!("oteryn:npc.{npc}");
        let route = book
            .get(&key)
            .and_then(|entry| {
                entry
                    .travel_routes
                    .iter()
                    .find(|route| route.keyword == keyword)
            })
            .ok_or_else(|| format!("missing {key}/{keyword}"))?;
        assert!(!route.held, "held {key}/{keyword}");
        assert!(route.route.min_level.is_none(), "{key}/{keyword}");
        assert!(
            route.route.required_quest_tracks.is_empty(),
            "{key}/{keyword}"
        );
        assert!(
            route.quest_gate.is_none() && route.quest_clear_gate.is_none(),
            "{key}/{keyword}"
        );
        assert!(
            !route.yalahar_gate && route.yalahar_town_gate.is_none(),
            "{key}/{keyword}"
        );
        assert!(
            route.ticket.is_none() && route.return_origin.is_none(),
            "{key}/{keyword}"
        );
        assert!(
            route.world_changes.is_empty() && route.partial_quest_callback.is_none(),
            "{key}/{keyword}"
        );
    }
    Ok(())
}

#[test]
fn crystal_return_ferries_resolve_source_routes() -> Result<(), String> {
    let book = book()?;
    let mut held = Vec::new();
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
            .find(|route| route.keyword == keyword)
            .ok_or_else(|| format!("missing {key}/{keyword}"))?;
        if route.held {
            held.push(format!("{key}/{keyword}"));
        }
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
    assert!(held.is_empty(), "ferries unexpectedly held: {held:?}");
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

#[test]
fn source_callback_lore_and_passage_replies() -> Result<(), String> {
    let book = book()?;
    let position = ChatPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    let cases: &[(&str, &str, &[&str])] = &[
        (
            "brewster",
            "cough syrup",
            &[
                "The only person who might have some cough syrup is this druid Ustan. You find him in the tavern. Hmmm the tavern ... <hicks>",
            ],
        ),
        (
            "dedoras",
            "search",
            &[
                "I gathered some lore on my own, but I desperately need more information that you might provide me. ...",
                "My leads are the {museum} in thais, something strange in the darashian {desert}, rumors about {fishmen}, an ancient {order}, the mysterious {asuri}, or a lost {isle}?",
            ],
        ),
        (
            "the_first_dragon",
            "fight",
            &[
                "Even in {retirement} I sometimes succumb to the temptation of sweet battle. Fighting was so different back in the old {times}. You can read about it in my {memoirs}. There should be a copy lying here somewhere.",
            ],
        ),
        (
            "the_first_dragon",
            "retirement",
            &[
                "When you are young, you are much more tolerant towards the burden of life. At a certain age even a {dragon} will start to think about if things will go on forever. ...",
                "Eventually there comes a time when you decide all the {hassle} isn't worth it anymore and you decide to concentrate on the {finer} things life has to offer.",
            ],
        ),
        (
            "the_first_dragon",
            "memoirs",
            &[
                "I dictated my memoirs to a human servant. I like the idea to share my thoughts and memories with humanity with the help of a {book}. ...",
                "You are my greatest fans after all. ...",
                "If it becomes popular, I might consider an orcish translation or even one in bonelord language. ...",
                "By the way, it's a funny story how I learnt the bonelord language. However, I saved it for a possible part two of my memoirs.",
            ],
        ),
        (
            "the_first_dragon",
            "hassle",
            &[
                "I really enjoy a good fight now and then. ...",
                "A real pain, however, is the constant annoyance caused by tedious fights against adventurers with more healing pots than brain.",
            ],
        ),
        (
            "the_first_dragon",
            "worthy",
            &[
                "To be honest, the first dragon hunters weren't {worthy}. ...",
                "They used {weaknesses} to their advantage. It took some time before a dragon could be killed in a fair fight.",
            ],
        ),
        (
            "the_first_dragon",
            "weaknesses",
            &[
                "The first awoken dragons were quite disoriented when leaving the {mists} and re-entering reality. ...",
                "This led to certain vulnerabilities. However, the dragons overcame such shortcomings quickly and soon took their rightful place in the food chain, so to say.",
            ],
        ),
        (
            "the_first_dragon",
            "mists",
            &[
                "There was a time before the gods cast the mists of healing over the world. At that time the dragons ruled and burnt the {world}. ...",
                "In hindsight I'm not too proud of this. This was some kind of juvenile bullying.",
            ],
        ),
        (
            "the_first_dragon",
            "world",
            &[
                "The world has changed so much; I'm beginning to feel really old. People have changed, the face of the world has changed, even the laws of nature are no longer the same. ...",
                "I think it is the greatest advantage of you {humans} to be more flexible and adaptable.",
            ],
        ),
        (
            "the_first_dragon",
            "books",
            &[
                "Your books are an amazing thing. Draconic glyphs are so different. ...",
                "They are not only more complicated but also more specific. A single word which always keeps the same meaning is astonishingly effective in its primitive way. ...",
                "It helps you to conserve thoughts, stories and history and to share them with others - a {concept} I really appreciate. ...",
                "Books are a wonderful {invention}. If you are ever tired of your heroic exploits, take the time to read one.",
            ],
        ),
        (
            "the_first_dragon",
            "invention",
            &[
                "Being powerful creatures themselves, dragons had no need for inventions. ...",
                "However, now that times have changed and the pressure increases, even my kin might slowly start to change in order to {adapt}.",
            ],
        ),
        (
            "the_first_dragon",
            "adapt",
            &[
                "Well, surely some of my kin might whine and complain as they always do. ...",
                "You have no idea how backwards thinking some dragons can be. A few are even still idealising the times before the {mists}.",
            ],
        ),
        (
            "the_first_dragon",
            "finer",
            &[
                "The definition of finer things is different for a dragon than for a human. ...",
                "A young dragon appreciates diving into a lava pool, while an old dragon just can't stand to get rid of the {lava} afterwards and prefers bathing in boiling water.",
            ],
        ),
        (
            "tom",
            "cough syrup",
            &[
                "I had some cough syrup a while ago. It was stolen in an ape raid. I fear if you want more cough syrup you will have to buy it in the druids guild in carlin.",
            ],
        ),
        ("captain_jack", "passage", &["I can travel you to {Tibia}."]),
    ];
    let mut differences = Vec::new();
    for &(name, keyword, expected) in cases {
        let key = format!("oteryn:npc.{name}");
        let entry = book.get(&key).ok_or_else(|| format!("missing {key}"))?;
        let npc = [TalkNpc {
            id: 1u8,
            position,
            entry,
        }];
        let mut focused = None;
        assert!(talk(position, "hi", &npc, &mut focused).is_some(), "{key}");
        let actual: Vec<_> = talk(position, keyword, &npc, &mut focused)
            .map(|reply| reply.into_lines().collect())
            .unwrap_or_default();
        if actual != expected {
            differences.push(format!(
                "{key} {keyword}: expected {expected:?}, got {actual:?}"
            ));
        }
        assert_eq!(focused, Some(1), "{key} {keyword}");
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
    Ok(())
}

#[test]
fn elf_focus_words_open_and_close_the_production_conversation() -> Result<(), String> {
    let book = book()?;
    let position = ChatPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    for key in ["oteryn:npc.edala", "oteryn:npc.maealil"] {
        let entry = book.get(key).ok_or(key)?;
        let npcs = [TalkNpc {
            id: 1,
            position,
            entry,
        }];
        let mut focus = None;
        assert!(
            talk(position, "ashari", &npcs, &mut focus).is_some(),
            "{key}"
        );
        assert_eq!(focus, Some(1), "{key}");
        assert!(
            talk(position, "asgha thrazi", &npcs, &mut focus).is_some(),
            "{key}"
        );
        assert_eq!(focus, None, "{key}");
    }
    Ok(())
}

#[test]
fn dixi_uses_source_focus_replies() -> Result<(), String> {
    let book = book()?;
    let position = ChatPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    let entry = book.get("oteryn:npc.dixi").ok_or("Dixi")?;
    let npcs = [TalkNpc {
        id: 1,
        position,
        entry,
    }];
    let mut focus = None;
    let _ = talk(position, "hello", &npcs, &mut focus).ok_or("Dixi greeting")?;
    let reply = talk(position, "farewell", &npcs, &mut focus).ok_or("Dixi farewell")?;
    assert_eq!(reply.text, "Good bye, Sir.");
    assert_eq!(focus, None);
    let source = source_dialogue::catalogue()
        .get("oteryn:npc.dixi")
        .ok_or("Dixi source")?;
    for (word, variant) in [
        ("hello", source.focus_greet.as_ref()),
        ("farewell", source.focus_farewell.as_ref()),
    ] {
        let expected = variant.ok_or("missing female focus reply")?;
        let reply = talk_with_speaker(position, word, &npcs, &mut focus, true)
            .ok_or("Dixi female focus reply")?;
        assert_eq!(reply.text, expected.reply);
    }
    assert_eq!(focus, None);
    Ok(())
}

#[test]
fn source_keywords_do_not_intercept_goodbye() -> Result<(), String> {
    let book = book()?;
    let position = ChatPosition {
        x: 100,
        y: 100,
        floor: 7,
    };
    for key in [
        "buddel",
        "buddel_helheim",
        "buddel_okolnir",
        "buddel_raider_camp",
        "buddel_tyrsung",
        "kendra",
        "king_tibianus",
        "queen_eloise",
    ] {
        let entry = book.get(&format!("oteryn:npc.{key}")).ok_or(key)?;
        let npc = [TalkNpc {
            id: 1u8,
            position,
            entry,
        }];
        let mut focused = Some(1);
        assert_eq!(
            talk(position, "goodbye", &npc, &mut focused)
                .map(|reply| reply.text)
                .as_deref(),
            Some(entry.farewell()),
            "{key}"
        );
        assert_eq!(focused, None, "{key}");
    }
    Ok(())
}
