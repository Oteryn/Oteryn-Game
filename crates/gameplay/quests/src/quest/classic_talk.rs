//! Dedicated Crystal 201189f NPC branch selection for batch-2 classics.
//! Sources: data-global/npc/{mr_morris,woblin,jerom,rottin_wood,the_queen_of_the_banshees}.lua;
//! every reply key is its npcHandler:say line in the pinned source.
//! Shared item exchanges and the final Queen teleport are explicitly deferred by ANSWER6589521017.
use std::collections::BTreeMap;

pub const MORRIS: &str = "oteryn:npc.mr_morris";
pub const WOBLIN: &str = "oteryn:npc.woblin";
pub const JEROM: &str = "oteryn:npc.jerom";
pub const QUEEN: &str = "oteryn:npc.the_queen_of_the_banshees";
pub const ROTTIN: &str = "oteryn:npc.rottin_wood";
const DAWN: &str = "oteryn:quest-progress/quest/u10_55/dawnport/";
const BANSHEE: &str = "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/";
pub const TROLL: &str = "oteryn:quest-progress/quest/u8_2/troll_sabotage_quest/questline";
pub const ROTTIN_STATE: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/";
pub const ROTTIN_WALLCOUNT: &str = "oteryn:quest-progress/crystalserver/kv/rottinwood-wallcount";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Topic {
    Morris(u8),
    Woblin(u8),
    Jerom,
    Queen(u8),
    Rottin(u8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub topic: Option<Topic>,
    pub branches: Vec<&'static str>,
    pub transition: Option<&'static str>,
    pub deferred: bool,
}

fn choice(topic: Option<Topic>, branch: &'static str, transition: Option<&'static str>) -> Choice {
    Choice {
        topic,
        branches: vec![branch],
        transition,
        deferred: false,
    }
}

// Shared item/teleport owners are explicitly outside this batch:
// https://github.com/Oteryn/Oteryn-Game/pull/2844#issuecomment-6589521017 (5).
fn deferred(topic: Option<Topic>) -> Choice {
    Choice {
        topic,
        branches: Vec::new(),
        transition: None,
        deferred: true,
    }
}

/// Crystal data/npclib/npc.lua:13-20. `%w+keyword` anywhere vetoes the match;
/// a right word boundary is deliberately not added to the donor's predicate.
pub(super) fn contains(text: &str, keyword: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let occurrences: Vec<_> = lower
        .match_indices(keyword)
        .map(|(index, _)| index)
        .collect();
    !occurrences.is_empty()
        && !occurrences
            .into_iter()
            .any(|index| index > 0 && lower.as_bytes()[index - 1].is_ascii_alphanumeric())
}

fn value(tracks: &BTreeMap<String, i64>, prefix: &str, name: &str) -> i64 {
    tracks.get(&format!("{prefix}{name}")).copied().unwrap_or(0)
}

// These source StdModule.say registrations precede each NPC's FocusModule and
// default callback. They belong to the existing static dialogue route, so this
// adapter must not turn a mixed message into a quest offer or confirmation.
fn static_keyword_intercepts(npc: &str, text: &str) -> bool {
    let registrations: &[&[&str]] = match npc {
        // Crystal201189f data-global/npc/mr_morris.lua:287-396.
        MORRIS => &[
            &["quest"],
            &["mission"],
            &["fetch"],
            &["kill"],
            &["name"],
            &["job"],
            &["dawnport"],
            &["secrets of dawnport"],
            &["archeological"],
            &["rookgaard"],
            &["coltrayne"],
            &["garamond"],
            &["hamish"],
            &["mr morris"],
            &["oressa"],
            &["plunderpurse"],
            &["inigo"],
            &["richard"],
            &["ser tybald"],
            &["wentworth"],
            &["menesto"],
            &["woblin"],
        ],
        // Crystal201189f data-global/npc/woblin.lua:124-159.
        WOBLIN => &[
            &["goblins"],
            &["minotaur"],
            &["troll"],
            &["dwarf", "dwarves"],
            &["cyclops"],
            &["quest"],
            &["help"],
            &["trade", "offer"],
            &["key"],
        ],
        // Crystal201189f data-global/npc/jerom.lua:89-131, including aliases.
        JEROM => &[
            &["ankrahmun"],
            &["bat"],
            &["bug"],
            &["lion"],
            &["wolf"],
            &["deer"],
            &["rotworm"],
            &["slime"],
            &["squirrel"],
            &["carlin"],
            &["cat"],
            &["daniel"],
            &["steelsoul"],
            &["dog"],
            &["edron"],
            &["farm"],
            &["pet"],
            &["country"],
            &["frog"],
            &["job"],
            &["mood"],
            &["work"],
            &["king"],
            &["tibianus"],
            &["liberty bay"],
            &["name"],
            &["parrot"],
            &["port hope"],
            &["races"],
            &["rat"],
            &["revenge"],
            &["ship"],
            &["snake"],
            &["spider"],
            &["thais"],
            &["trade"],
            &["merchant"],
            &["troll"],
            &["rock"],
            &["what"],
            &["happen"],
            &["house"],
            &["accident"],
        ],
        // Crystal201189f data-global/npc/the_queen_of_the_banshees.lua:179-213.
        QUEEN => &[
            &["name"],
            &["destruction"],
            &["last"],
            &["stay"],
            &["guardian"],
            &["place"],
            &["race"],
            &["seed"],
            &["complex"],
            &["ghostlands"],
            &["banshee"],
        ],
        _ => return false,
    };
    let lower = text.to_ascii_lowercase();
    registrations.iter().any(|words| {
        // keyword_handler.lua:57-68 uses the first substring occurrence of
        // every keyword in registration order, with no word-boundary guard.
        let mut previous = 0;
        words.iter().all(|word| {
            let Some(index) = lower.find(*word) else {
                return false;
            };
            if index < previous {
                return false;
            }
            previous = index;
            true
        })
    })
}

/// Topic identity is checked against the live conversation by the dispatch adapter.
#[must_use]
pub fn choose(
    npc: &str,
    text: &str,
    topic: Option<Topic>,
    tracks: &BTreeMap<String, i64>,
) -> Option<Choice> {
    // npc_handler.lua:446-451 never calls CALLBACK_MESSAGE_DEFAULT after one
    // of these earlier handlers accepts the message. Returning None leaves
    // the existing quest topic intact and selects no durable transition.
    if static_keyword_intercepts(npc, text) || super::quests_2_focus::intercepts(text) {
        return None;
    }
    match npc {
        MORRIS => morris(text, topic, tracks),
        WOBLIN => woblin(text, topic, tracks),
        JEROM => jerom(text, topic, tracks),
        QUEEN => queen(text, topic, tracks),
        ROTTIN => rottin(text, topic, tracks),
        _ => None,
    }
}

fn morris(text: &str, topic: Option<Topic>, tracks: &BTreeMap<String, i64>) -> Option<Choice> {
    // mr_morris.lua:77-151: branch precedence is amulet, log book, herbs, key, yes.
    for (word, track, number, line) in [
        ("amulet", "the_lost_amulet", 1, "line-79"),
        ("log book", "torn_log_book", 2, "line-98"),
        ("herbs", "the_rare_herb", 3, "line-120"),
        ("key", "the_dorm_key", 4, "line-140"),
    ] {
        if contains(text, word) {
            let stage = value(tracks, DAWN, track);
            return if stage < 1 {
                Some(choice(Some(Topic::Morris(number)), line, None))
            } else if (word == "amulet" || word == "herbs") && stage == 2
                || word == "log book"
                    && stage == 1
                    && value(tracks, DAWN, "the_stolen_log_book") == 1
            {
                // mr_morris.lua:85-94,106-115,127-135. A success line or
                // completion flag must never substitute for the deferred reward.
                Some(deferred(topic))
            } else if word == "key" && stage == 4 {
                Some(choice(Some(Topic::Morris(5)), "line-149", None))
            } else {
                None
            };
        }
    }
    if contains(text, "yes") {
        let Topic::Morris(number) = topic? else {
            return None;
        };
        if number == 5 {
            return Some(deferred(topic));
        }
        let (line, transition) = match number {
            1 => (
                "line-154",
                "oteryn:quest-transition/classic/dawnport/start-the-lost-amulet",
            ),
            2 => (
                "line-164",
                "oteryn:quest-transition/classic/dawnport/start-logbook",
            ),
            3 => (
                "line-173",
                "oteryn:quest-transition/classic/dawnport/start-the-rare-herb",
            ),
            4 => (
                "line-184",
                "oteryn:quest-transition/classic/dawnport/start-the-dorm-key",
            ),
            6 => (
                "line-194",
                "oteryn:quest-transition/classic/dawnport/start-morrisk-troll",
            ),
            7 => (
                "line-204",
                "oteryn:quest-transition/classic/dawnport/start-morris-goblin",
            ),
            8 => (
                "line-214",
                "oteryn:quest-transition/classic/dawnport/start-morris-minos",
            ),
            _ => return None,
        };
        return Some(choice(None, line, Some(transition)));
    }
    // mr_morris.lua:226-281: qualification and cap are independent of reward delivery.
    for (word, active, count, number, offer, unfinished) in [
        (
            "trolls",
            "morrisk_troll",
            "morris_troll_count",
            6,
            "line-228",
            "line-241",
        ),
        (
            "goblins",
            "morris_goblin",
            "morris_goblin_count",
            7,
            "line-246",
            "line-259",
        ),
        (
            "minotaur",
            "morris_minos",
            "morris_minos_count",
            8,
            "line-264",
            "line-277",
        ),
    ] {
        if !contains(text, word) {
            continue;
        }
        let stage = value(tracks, DAWN, active);
        return if stage < 1 {
            Some(choice(Some(Topic::Morris(number)), offer, None))
        } else if stage == 1 && value(tracks, DAWN, count) < 20 {
            Some(choice(topic, unfinished, None))
        } else if stage == 1 && value(tracks, DAWN, count) >= 20 {
            Some(deferred(topic))
        } else if word == "minotaur" && stage > 1 {
            Some(choice(topic, "line-280", None))
        } else {
            None
        };
    }
    None
}

fn woblin(text: &str, topic: Option<Topic>, tracks: &BTreeMap<String, i64>) -> Option<Choice> {
    // woblin.lua:63-100 has consecutive ifs, not an elseif chain. A message such as
    // "precious reward" therefore emits both literal replies and advances once.
    let mut next = topic;
    let mut branches = Vec::new();
    let mut transition = None;
    let mut stage = value(tracks, DAWN, "the_dorm_key");
    if contains(text, "precious") {
        if stage < 1 {
            branches.push("line-65");
            next = None;
        } else if stage == 1 {
            branches.push("line-69");
            next = Some(Topic::Woblin(1));
        } else if stage >= 4 {
            branches.push("line-79");
            next = None;
        }
    }
    if contains(text, "reward") && next == Some(Topic::Woblin(1)) {
        branches.push("line-86");
        next = None;
        stage = 2;
        transition = Some("oteryn:quest-transition/classic/dawnport/find-old-nasty");
    }
    if contains(text, "old nasty") && stage >= 2 {
        // The item exchange after this question has a separate receipt owner.
        if contains(text, "yes") {
            return Some(deferred(topic));
        }
        branches.push("line-99");
        next = Some(Topic::Woblin(2));
    }
    if contains(text, "yes") && next == Some(Topic::Woblin(2)) {
        return Some(deferred(topic));
    }
    (!branches.is_empty()).then_some(Choice {
        topic: next,
        branches,
        transition,
        deferred: false,
    })
}

fn jerom(text: &str, topic: Option<Topic>, tracks: &BTreeMap<String, i64>) -> Option<Choice> {
    // jerom.lua:67-84; necklace exchange is intentionally not a state-only transition.
    if contains(text, "mission") || contains(text, "quest") {
        return match tracks.get(TROLL).copied().unwrap_or(0) {
            stage if stage < 1 => Some(choice(Some(Topic::Jerom), "line-69", None)),
            2 => Some(deferred(topic)),
            _ => None,
        };
    }
    (contains(text, "yes") && topic == Some(Topic::Jerom)).then(|| {
        choice(
            None,
            "line-82",
            Some("oteryn:quest-transition/classic/troll-sabotage/start"),
        )
    })
}

fn queen(text: &str, topic: Option<Topic>, tracks: &BTreeMap<String, i64>) -> Option<Choice> {
    // the_queen_of_the_banshees.lua:62-72,89-136. Six independent confirmations;
    // "seventh seal" is an offer, never evidence that the six flags are set.
    if contains(text, "seventh seal") {
        return Some(choice(Some(Topic::Queen(1)), "line-63", None));
    }
    if contains(text, "kiss") && topic == Some(Topic::Queen(7)) {
        return Some(if value(tracks, BANSHEE, "last_seal") < 1 {
            choice(Some(Topic::Queen(8)), "line-67", None)
        } else {
            choice(topic, "line-70", None)
        });
    }
    let Some(Topic::Queen(number)) = topic else {
        return None;
    };
    if contains(text, "yes") && number == 8 {
        return Some(deferred(topic));
    }
    if contains(text, "yes") && (1..=6).contains(&number) {
        let (flag, accepted, refused) = match number {
            1 => ("fourth_seal", "line-91", "line-94"),
            2 => ("first_seal", "line-99", "line-102"),
            3 => ("second_seal", "line-107", "line-110"),
            4 => ("third_seal", "line-115", "line-118"),
            5 => ("fifth_seal", "line-123", "line-126"),
            6 => ("sixth_seal", "line-131", "line-134"),
            _ => return None,
        };
        return Some(if value(tracks, BANSHEE, flag) == 1 {
            choice(Some(Topic::Queen(number + 1)), accepted, None)
        } else {
            choice(None, refused, None)
        });
    }
    if contains(text, "no") && (1..=8).contains(&number) {
        // The donor does not clear its topic on "no" (lines170-175).
        return Some(choice(
            topic,
            if number == 8 { "line-174" } else { "line-172" },
            None,
        ));
    }
    None
}

fn rottin(text: &str, topic: Option<Topic>, tracks: &BTreeMap<String, i64>) -> Option<Choice> {
    // rottin_wood.lua:88-179. Only Mission 02's acceptance is state-only;
    // the neighbouring acceptances include ribbons/nets and remain deferred.
    if contains(text, "mission") {
        return Some(match value(tracks, ROTTIN_STATE, "questline") {
            stage if stage < 1 => choice(Some(Topic::Rottin(1)), "line-91", None),
            1 => choice(Some(Topic::Rottin(3)), "line-94", None),
            2 => choice(Some(Topic::Rottin(4)), "line-98", None),
            3 => choice(Some(Topic::Rottin(5)), "line-107", None),
            4 | 5 => deferred(topic),
            _ => choice(None, "line-141", None),
        });
    }
    if contains(text, "yes") {
        let Some(Topic::Rottin(number)) = topic else {
            return None;
        };
        return Some(match number {
            1 => choice(Some(Topic::Rottin(2)), "line-149", None),
            4 => choice(
                None,
                "line-175",
                Some("oteryn:quest-transition/classic/rottin-wood/start-disrepair"),
            ),
            5 if tracks.get(ROTTIN_WALLCOUNT).copied().unwrap_or(0) < 6 => {
                choice(None, "line-192", None)
            }
            2 | 3 | 5 | 6 | 7 | 9 | 10 | 11 | 12 => deferred(topic),
            _ => return None,
        });
    }
    if contains(text, "no") {
        let Some(Topic::Rottin(number)) = topic else {
            return None;
        };
        return match number {
            1 => Some(choice(Some(Topic::Rottin(2)), "line-297", None)),
            3 => Some(choice(None, "line-304", None)),
            4 => Some(choice(None, "line-307", None)),
            5 => Some(choice(None, "line-310", None)),
            _ => None,
        };
    }
    if contains(text, "task") {
        return Some(match value(tracks, ROTTIN_STATE, "questline") {
            stage if stage >= 4 => choice(Some(Topic::Rottin(8)), "line-330", None),
            stage if stage >= 2 => choice(Some(Topic::Rottin(8)), "line-333", None),
            _ => choice(None, "line-336", None),
        });
    }
    if (contains(text, "lucky charms") || contains(text, "repairs"))
        && topic == Some(Topic::Rottin(8))
    {
        return Some(deferred(topic));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_static_keywords_preempt_classic_quest_transitions_and_topics() {
        // Crystal npc_handler.lua:446-451 calls the default callback only after
        // KeywordHandler declines. These earlier StdModule.say nodes keep the
        // existing topic and never accept the embedded quest confirmation.
        let tracks = BTreeMap::from([
            (format!("{DAWN}the_dorm_key"), 1),
            (format!("{BANSHEE}fourth_seal"), 1),
        ]);
        for (npc, text, topic) in [
            (MORRIS, "yes job", Some(Topic::Morris(1))),
            (MORRIS, "yes myjob", Some(Topic::Morris(1))),
            (MORRIS, "amulet quest", None),
            (WOBLIN, "reward help", Some(Topic::Woblin(1))),
            (WOBLIN, "precious key", None),
            (JEROM, "yes deer", Some(Topic::Jerom)),
            (JEROM, "mission revenge", None),
            (QUEEN, "yes name", Some(Topic::Queen(1))),
            (QUEEN, "seventh seal last", None),
        ] {
            assert!(
                choose(npc, text, topic, &tracks).is_none(),
                "earlier source StdModule.say must preempt {npc}: {text}"
            );
        }
    }

    #[test]
    fn source_focus_words_preempt_all_classic_quest_confirmations() {
        // Crystal modules.lua:5-8,286-310,361-387; every NPC registers
        // FocusModule before CALLBACK_MESSAGE_DEFAULT can receive a message.
        let tracks = BTreeMap::from([
            (format!("{DAWN}the_dorm_key"), 1),
            (format!("{BANSHEE}fourth_seal"), 1),
            (format!("{ROTTIN_STATE}questline"), 2),
        ]);
        for (npc, confirmation, topic) in [
            (MORRIS, "yes", Some(Topic::Morris(1))),
            (WOBLIN, "reward", Some(Topic::Woblin(1))),
            (JEROM, "yes", Some(Topic::Jerom)),
            (QUEEN, "yes", Some(Topic::Queen(1))),
            (ROTTIN, "yes", Some(Topic::Rottin(4))),
        ] {
            for word in ["hi", "hello", "bye", "farewell", "trade", "offers"] {
                let text = format!("{confirmation} {word}");
                assert!(
                    choose(npc, &text, topic, &tracks).is_none(),
                    "source FocusModule must preempt {npc}: {text}"
                );
            }
        }
    }

    #[test]
    fn source_preemption_preserves_ordered_multiword_nodes_and_focus_vetoes() -> Result<(), String>
    {
        let tracks = BTreeMap::from([
            (format!("{DAWN}the_dorm_key"), 1),
            (format!("{ROTTIN_STATE}questline"), 2),
        ]);
        // Woblin.lua:136 has one ordered, all-words node, not two aliases;
        // keyword_handler.lua:57-68 compares the first occurrence of each word.
        assert!(
            choose(
                WOBLIN,
                "reward dwarf dwarves",
                Some(Topic::Woblin(1)),
                &tracks
            )
            .is_none()
        );
        for text in ["reward dwarf", "reward dwarves dwarf"] {
            let chosen = choose(WOBLIN, text, Some(Topic::Woblin(1)), &tracks)
                .ok_or("unordered or incomplete keyword must fall through")?;
            assert_eq!(
                chosen.transition,
                Some("oteryn:quest-transition/classic/dawnport/find-old-nasty")
            );
        }
        for text in ["yes high", "yes +hi", "yes hi+", "yes hi xhi"] {
            let chosen = choose(ROTTIN, text, Some(Topic::Rottin(4)), &tracks)
                .ok_or("donor FocusModule boundary veto must fall through")?;
            assert_eq!(
                chosen.transition,
                Some("oteryn:quest-transition/classic/rottin-wood/start-disrepair")
            );
        }
        Ok(())
    }

    #[test]
    fn deferred_rewards_and_final_teleport_cannot_become_state_only_successes() -> Result<(), String>
    {
        let tracks = BTreeMap::from([
            (format!("{DAWN}the_lost_amulet"), 2),
            (format!("{DAWN}the_rare_herb"), 2),
            (format!("{DAWN}torn_log_book"), 1),
            (format!("{DAWN}the_stolen_log_book"), 1),
            (TROLL.into(), 2),
            (ROTTIN_WALLCOUNT.into(), 6),
        ]);
        for (npc, text, topic) in [
            (MORRIS, "amulet", None),
            (MORRIS, "herbs", None),
            (MORRIS, "log book", None),
            (MORRIS, "yes", Some(Topic::Morris(5))),
            (WOBLIN, "yes", Some(Topic::Woblin(2))),
            (JEROM, "mission", None),
            (QUEEN, "yes", Some(Topic::Queen(8))),
            (ROTTIN, "yes", Some(Topic::Rottin(2))),
            (ROTTIN, "yes", Some(Topic::Rottin(5))),
        ] {
            let step = choose(npc, text, topic, &tracks).ok_or("explicit refusal")?;
            assert!(step.deferred);
            assert!(step.transition.is_none());
            assert!(step.branches.is_empty());
        }
        Ok(())
    }

    #[test]
    fn rottin_disrepair_requires_its_exact_offer_and_cannot_give_ribbons_as_flags()
    -> Result<(), String> {
        let tracks = BTreeMap::from([(format!("{ROTTIN_STATE}questline"), 2)]);
        let offered = choose(ROTTIN, "mission", None, &tracks).ok_or("disrepair offer")?;
        assert_eq!(offered.topic, Some(Topic::Rottin(4)));
        assert_eq!(offered.branches, ["line-98"]);
        let accepted = choose(ROTTIN, "yes", offered.topic, &tracks).ok_or("disrepair start")?;
        assert_eq!(
            accepted.transition,
            Some("oteryn:quest-transition/classic/rottin-wood/start-disrepair")
        );
        assert!(!accepted.deferred);
        assert!(choose(ROTTIN, "yes", Some(Topic::Morris(4)), &tracks).is_none());
        let declined = choose(ROTTIN, "no", offered.topic, &tracks).ok_or("decline")?;
        assert_eq!(declined.branches, ["line-307"]);
        assert!(declined.transition.is_none());
        Ok(())
    }

    #[test]
    fn donor_keyword_veto_and_precedence_are_preserved() -> Result<(), String> {
        let tracks = BTreeMap::new();
        assert!(choose(MORRIS, "myamulet", None, &tracks).is_none());
        assert!(choose(MORRIS, "amulet myamulet", None, &tracks).is_none());
        let ask = choose(MORRIS, "amulet yes", Some(Topic::Morris(3)), &tracks).ok_or("offer")?;
        assert_eq!(ask.topic, Some(Topic::Morris(1)));
        assert!(ask.transition.is_none());
        assert!(choose(MORRIS, "yes", Some(Topic::Jerom), &tracks).is_none());
        Ok(())
    }

    #[test]
    fn woblin_independent_ifs_keep_both_replies_and_one_transition() -> Result<(), String> {
        let tracks = BTreeMap::from([(format!("{DAWN}the_dorm_key"), 1)]);
        let step = choose(WOBLIN, "precious reward", None, &tracks).ok_or("Woblin")?;
        assert_eq!(step.branches, ["line-69", "line-86"]);
        assert_eq!(step.topic, None);
        assert_eq!(
            step.transition,
            Some("oteryn:quest-transition/classic/dawnport/find-old-nasty")
        );
        Ok(())
    }

    #[test]
    fn queen_checks_each_seal_in_donor_order_and_does_not_grant_access() -> Result<(), String> {
        let mut tracks = BTreeMap::new();
        let refused = choose(QUEEN, "yes", Some(Topic::Queen(1)), &tracks).ok_or("refusal")?;
        assert_eq!(refused.branches, ["line-94"]);
        assert_eq!(refused.topic, None);
        let order = [
            "fourth_seal",
            "first_seal",
            "second_seal",
            "third_seal",
            "fifth_seal",
            "sixth_seal",
        ];
        for (index, flag) in order.into_iter().enumerate() {
            tracks.insert(format!("{BANSHEE}{flag}"), 1);
            let step =
                choose(QUEEN, "yes", Some(Topic::Queen(index as u8 + 1)), &tracks).ok_or("seal")?;
            assert_eq!(step.topic, Some(Topic::Queen(index as u8 + 2)));
            assert!(step.transition.is_none());
        }
        assert!(choose(QUEEN, "kiss", None, &tracks).is_none());
        let deferred = choose(QUEEN, "yes", Some(Topic::Queen(8)), &tracks)
            .ok_or("explicit deferred final teleport")?;
        assert!(deferred.deferred);
        assert!(deferred.branches.is_empty());
        assert!(deferred.transition.is_none());
        Ok(())
    }
}
