//! Targuna's source NPC branches; these return intents, never write state or mint.
//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/
//! {lizzie,camilla,sterling,morla,emiliana}.lua; data/npclib/npc.lua:14-21.
//! Full text is source-addressed by NPC name and original line in the dialogue data.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Npc {
    Lizzie,
    Camilla,
    Sterling,
    Morla,
    Emiliana,
    Adrian,
}

impl Npc {
    pub const fn source_name(self) -> &'static str {
        match self {
            Self::Lizzie => "lizzie",
            Self::Camilla => "camilla",
            Self::Sterling => "sterling",
            Self::Morla => "morla",
            Self::Emiliana => "emiliana",
            Self::Adrian => "adrian",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NpcState {
    pub questline: i64,
    pub ancient_mission: i64,
    pub commander_killed: i64,
    pub camilla_reward: i64,
    pub treasure_mission: i64,
    pub pirates: i64,
    pub eggs: i64,
    pub morla_done: i64,
    pub treasure_marked: i64,
    pub burning_mission: i64,
    pub herald_killed: i64,
    pub turnips_reward: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Topic {
    Start,
    MapTurnIn,
    Reward,
    ReplaceTurnips,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mutation {
    StartAncient,
    ReportCommander,
    CamillaReward,
    StartTreasure,
    MorlaReward,
    MarkTreasure,
    StartBurning,
    Briefing,
    TurnipsReward,
    ReplaceTurnips,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmilianaPlace {
    Targuna,
    CrimsonCourt,
    OutsideCrimsonCourt,
}

/// NPC z <= 7 is the surface recruiter. The second NPC only advances the quest
/// when the player stands in the exact source box (emiliana.lua:72-83,209-212).
pub fn emiliana_place(
    npc_source_z: i16,
    player_x: i32,
    player_y: i32,
    player_source_z: i16,
) -> EmilianaPlace {
    if npc_source_z <= 7 {
        EmilianaPlace::Targuna
    } else if player_source_z == 12
        && (32409..=32420).contains(&player_x)
        && (32683..=32693).contains(&player_y)
    {
        EmilianaPlace::CrimsonCourt
    } else {
        EmilianaPlace::OutsideCrimsonCourt
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    pub source_line: u16,
    /// Values for the source's %d placeholders, in source order; no arbitrary format string.
    pub numbers: Vec<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NpcStep {
    pub topic: Option<Topic>,
    pub reply: Reply,
    pub mutation: Option<Mutation>,
}

fn reply(line: u16, topic: Option<Topic>) -> NpcStep {
    NpcStep {
        topic,
        reply: Reply {
            source_line: line,
            numbers: Vec::new(),
        },
        mutation: None,
    }
}

fn write(line: u16, mutation: Mutation) -> NpcStep {
    NpcStep {
        mutation: Some(mutation),
        ..reply(line, None)
    }
}

/// Literal MsgContains for the ASCII keyword set used by these five callbacks.
/// Lua's second pattern excludes the match if *any* occurrence is preceded by %w;
/// it does not require a trailing boundary and Lua %w does not include underscore.
pub fn message_contains(message: &str, keyword: &str) -> bool {
    let message = message.to_ascii_lowercase();
    let keyword = keyword.to_ascii_lowercase();
    if message == keyword {
        return true;
    }
    if keyword.is_empty()
        || !keyword
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b' ')
    {
        return false;
    }
    message.contains(&keyword)
        && !message
            .match_indices(&keyword)
            .any(|(index, _)| index > 0 && message.as_bytes()[index - 1].is_ascii_alphanumeric())
}

/// Uses the original source order: a mission keyword may precede a yes/no keyword.
/// Confirmation state belongs to the exact current NPC conversation, not the character.
pub fn choose(
    npc: Npc,
    message: &str,
    topic: Option<Topic>,
    state: NpcState,
    level: u32,
    place: EmilianaPlace,
) -> Option<NpcStep> {
    let has = |keyword| message_contains(message, keyword);
    match npc {
        Npc::Lizzie => {
            if state.ancient_mission >= 2 && has("mission") {
                return Some(if state.ancient_mission >= 3 {
                    reply(88, topic)
                } else if state.commander_killed == 1 {
                    // lizzie.lua:92-98: a report changes Mission, never the killed flag.
                    NpcStep {
                        topic,
                        ..write(93, Mutation::ReportCommander)
                    }
                } else {
                    reply(101, topic)
                });
            }
            if has("save") || has("lizard") || has("mission") {
                return Some(if state.ancient_mission >= 2 {
                    reply(108, topic)
                } else {
                    reply(111, Some(Topic::Start))
                });
            }
            if has("yes") && topic == Some(Topic::Start) {
                return Some(write(122, Mutation::StartAncient));
            }
            if has("no") && topic == Some(Topic::Start) {
                return Some(reply(139, None));
            }
        }
        Npc::Camilla => {
            if has("mission") {
                return Some(if state.camilla_reward == 1 {
                    reply(74, topic)
                } else if state.commander_killed != 1 {
                    reply(79, topic)
                } else {
                    reply(83, Some(Topic::Reward))
                });
            }
            if has("yes") && topic == Some(Topic::Reward) {
                return Some(if state.camilla_reward == 1 {
                    reply(94, None)
                } else {
                    write(98, Mutation::CamillaReward)
                });
            }
            if has("no") && topic == Some(Topic::Reward) {
                return Some(reply(112, None));
            }
        }
        Npc::Sterling => {
            if has("mission") {
                return Some(match state.treasure_mission {
                    value if value < 1 => reply(91, Some(Topic::Start)),
                    1 => reply(100, topic),
                    2 => reply(105, topic),
                    3 => reply(110, Some(Topic::MapTurnIn)),
                    _ => reply(119, topic),
                });
            }
            if has("yes") {
                if topic == Some(Topic::Start) {
                    return Some(if level < 10 {
                        reply(128, None)
                    } else {
                        write(133, Mutation::StartTreasure)
                    });
                }
                if topic == Some(Topic::MapTurnIn) {
                    return Some(if state.morla_done != 1 {
                        reply(144, None)
                    } else {
                        // Source neither requires nor consumes item53158.
                        write(148, Mutation::MarkTreasure)
                    });
                }
            }
            if has("no") && matches!(topic, Some(Topic::Start | Topic::MapTurnIn)) {
                return Some(reply(164, None));
            }
        }
        Npc::Morla => {
            if has("mission") {
                return Some(if state.morla_done == 1 {
                    reply(73, topic)
                } else if state.treasure_mission < 1 {
                    reply(78, topic)
                } else {
                    reply(82, Some(Topic::Reward))
                });
            }
            if has("yes") && topic == Some(Topic::Reward) {
                return Some(if state.morla_done == 1 {
                    reply(95, None)
                } else if state.pirates.max(0) < 20 || state.eggs.max(0) < 10 {
                    let mut result = reply(102, None);
                    result.reply.numbers = vec![
                        (20 - state.pirates.max(0)).max(0),
                        (10 - state.eggs.max(0)).max(0),
                    ];
                    result
                } else {
                    write(106, Mutation::MorlaReward)
                });
            }
            if has("no") && topic == Some(Topic::Reward) {
                return Some(reply(121, None));
            }
        }
        Npc::Emiliana => match place {
            EmilianaPlace::Targuna => {
                if has("help") || has("mission") {
                    return Some(if state.turnips_reward == 1 {
                        reply(120, topic)
                    } else if state.herald_killed == 1 {
                        reply(125, Some(Topic::Reward))
                    } else if state.burning_mission >= 1 {
                        reply(133, topic)
                    } else if level < 12 {
                        reply(138, topic)
                    } else {
                        reply(141, Some(Topic::Start))
                    });
                }
                if has("lose") {
                    return Some(if state.turnips_reward != 1 {
                        reply(149, topic)
                    } else {
                        reply(152, Some(Topic::ReplaceTurnips))
                    });
                }
                if has("yes") {
                    return match topic {
                        Some(Topic::Start) => Some(write(160, Mutation::StartBurning)),
                        Some(Topic::Reward) => Some(if state.turnips_reward == 1 {
                            reply(170, None)
                        } else {
                            write(173, Mutation::TurnipsReward)
                        }),
                        Some(Topic::ReplaceTurnips) => Some(write(192, Mutation::ReplaceTurnips)),
                        _ => None,
                    };
                }
                if has("no") {
                    return Some(reply(201, None));
                }
            }
            EmilianaPlace::CrimsonCourt if has("mission") => {
                return Some(if state.turnips_reward == 1 {
                    reply(216, topic)
                } else if state.burning_mission < 1 {
                    reply(221, topic)
                } else if state.herald_killed == 1 {
                    reply(226, topic)
                } else if state.burning_mission == 1 {
                    NpcStep {
                        topic,
                        ..write(231, Mutation::Briefing)
                    }
                } else {
                    reply(240, topic)
                });
            }
            EmilianaPlace::CrimsonCourt | EmilianaPlace::OutsideCrimsonCourt => {}
        },
        // Adrian retains the existing banking owner; only its successful committed balance
        // delta may trigger secondary progress (adrian.lua:77-88).
        Npc::Adrian => {}
    }
    None
}

pub fn greet(npc: Npc, state: NpcState, place: EmilianaPlace) -> u16 {
    match npc {
        Npc::Lizzie if state.ancient_mission < 2 => 68,
        Npc::Lizzie => 70,
        Npc::Camilla => 133,
        Npc::Sterling if state.treasure_mission < 1 => 72,
        Npc::Sterling => 74,
        Npc::Morla => 137,
        Npc::Emiliana if place == EmilianaPlace::Targuna => {
            if state.turnips_reward == 1 || state.burning_mission >= 4 {
                90
            } else if state.herald_killed == 1 {
                92
            } else {
                94
            }
        }
        Npc::Emiliana if state.herald_killed == 1 => 98,
        Npc::Emiliana => 100,
        Npc::Adrian => 111,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(
        npc: Npc,
        message: &str,
        topic: Option<Topic>,
        state: NpcState,
        level: u32,
    ) -> Option<NpcStep> {
        choose(npc, message, topic, state, level, EmilianaPlace::Targuna)
    }

    #[test]
    fn source_msgcontains_preserves_prefix_rule_and_any_bad_occurrence() {
        assert!(message_contains("yes please", "yes"));
        assert!(message_contains("yesno", "yes"));
        assert!(message_contains("_yes", "yes"));
        assert!(!message_contains("eyes", "yes"));
        assert!(!message_contains("yes eyes", "yes"));
        assert!(message_contains("Please save", "save"));
        assert!(!message_contains("Please SAVEgarb garbageSave", "save"));
    }

    #[test]
    fn source_greetings_follow_live_mission_and_emiliana_location() {
        let mut state = NpcState::default();
        assert_eq!(greet(Npc::Lizzie, state, EmilianaPlace::Targuna), 68);
        state.ancient_mission = 2;
        assert_eq!(greet(Npc::Lizzie, state, EmilianaPlace::Targuna), 70);
        assert_eq!(greet(Npc::Sterling, state, EmilianaPlace::Targuna), 72);
        state.treasure_mission = 1;
        assert_eq!(greet(Npc::Sterling, state, EmilianaPlace::Targuna), 74);
        assert_eq!(greet(Npc::Emiliana, state, EmilianaPlace::Targuna), 94);
        assert_eq!(
            greet(Npc::Emiliana, state, EmilianaPlace::CrimsonCourt),
            100
        );
        state.herald_killed = 1;
        assert_eq!(greet(Npc::Emiliana, state, EmilianaPlace::Targuna), 92);
        assert_eq!(greet(Npc::Emiliana, state, EmilianaPlace::CrimsonCourt), 98);
        state.turnips_reward = 1;
        assert_eq!(greet(Npc::Emiliana, state, EmilianaPlace::Targuna), 90);
    }

    #[test]
    fn lizzie_report_does_not_regress_completed_mission_and_yes_needs_topic() -> Result<(), String>
    {
        assert_eq!(step(Npc::Lizzie, "yes", None, NpcState::default(), 8), None);
        let mut state = NpcState {
            ancient_mission: 2,
            commander_killed: 1,
            ..NpcState::default()
        };
        assert_eq!(
            step(Npc::Lizzie, "mission", None, state, 8)
                .ok_or("source NPC rule")?
                .mutation,
            Some(Mutation::ReportCommander)
        );
        state.ancient_mission = 4;
        assert_eq!(
            step(Npc::Lizzie, "mission", None, state, 8)
                .ok_or("source NPC rule")?
                .mutation,
            None
        );
        assert_eq!(
            step(Npc::Lizzie, "mission yes", Some(Topic::Start), state, 8)
                .ok_or("source NPC rule")?
                .reply
                .source_line,
            88
        );
        Ok(())
    }

    #[test]
    fn camilla_offers_reward_only_after_commander_and_rechecks_claim() -> Result<(), String> {
        let state = NpcState::default();
        assert_eq!(
            step(Npc::Camilla, "mission", None, state, 8)
                .ok_or("source NPC rule")?
                .topic,
            None
        );
        let state = NpcState {
            commander_killed: 1,
            ..state
        };
        assert_eq!(
            step(Npc::Camilla, "mission", None, state, 8)
                .ok_or("source NPC rule")?
                .topic,
            Some(Topic::Reward)
        );
        assert_eq!(
            step(Npc::Camilla, "yes", Some(Topic::Reward), state, 8)
                .ok_or("source NPC rule")?
                .mutation,
            Some(Mutation::CamillaReward)
        );
        let state = NpcState {
            camilla_reward: 1,
            ..state
        };
        assert_eq!(
            step(Npc::Camilla, "yes", Some(Topic::Reward), state, 8)
                .ok_or("source NPC rule")?
                .mutation,
            None
        );
        Ok(())
    }

    #[test]
    fn sterling_level_gate_is_at_confirmation_and_map_requires_morla_not_item() -> Result<(), String>
    {
        let state = NpcState::default();
        assert_eq!(
            step(Npc::Sterling, "mission", None, state, 9)
                .ok_or("source NPC rule")?
                .topic,
            Some(Topic::Start)
        );
        assert_eq!(
            step(Npc::Sterling, "yes", Some(Topic::Start), state, 9)
                .ok_or("source NPC rule")?
                .mutation,
            None
        );
        assert_eq!(
            step(Npc::Sterling, "yes", Some(Topic::Start), state, 10)
                .ok_or("source NPC rule")?
                .mutation,
            Some(Mutation::StartTreasure)
        );
        let state = NpcState {
            treasure_mission: 3,
            morla_done: 1,
            ..state
        };
        assert_eq!(
            step(Npc::Sterling, "yes", Some(Topic::MapTurnIn), state, 10)
                .ok_or("source NPC rule")?
                .mutation,
            Some(Mutation::MarkTreasure)
        );
        Ok(())
    }

    #[test]
    fn morla_requires_both_counts_normalizes_source_absence_and_does_not_repeat()
    -> Result<(), String> {
        let state = NpcState {
            treasure_mission: 2,
            pirates: -1,
            eggs: -1,
            ..NpcState::default()
        };
        assert_eq!(
            step(Npc::Morla, "yes", Some(Topic::Reward), state, 10)
                .ok_or("source NPC rule")?
                .reply
                .numbers,
            vec![20, 10]
        );
        let state = NpcState {
            pirates: 20,
            eggs: 9,
            ..state
        };
        assert_eq!(
            step(Npc::Morla, "yes", Some(Topic::Reward), state, 10)
                .ok_or("source NPC rule")?
                .mutation,
            None
        );
        let state = NpcState { eggs: 10, ..state };
        assert_eq!(
            step(Npc::Morla, "yes", Some(Topic::Reward), state, 10)
                .ok_or("source NPC rule")?
                .mutation,
            Some(Mutation::MorlaReward)
        );
        let state = NpcState {
            morla_done: 1,
            ..state
        };
        assert_eq!(
            step(Npc::Morla, "yes", Some(Topic::Reward), state, 10)
                .ok_or("source NPC rule")?
                .mutation,
            None
        );
        Ok(())
    }

    #[test]
    fn emiliana_location_box_and_level_gate_follow_source_order() -> Result<(), String> {
        assert_eq!(emiliana_place(7, 0, 0, 15), EmilianaPlace::Targuna);
        assert_eq!(
            emiliana_place(12, 32409, 32683, 12),
            EmilianaPlace::CrimsonCourt
        );
        assert_eq!(
            emiliana_place(12, 32420, 32693, 12),
            EmilianaPlace::CrimsonCourt
        );
        assert_eq!(
            emiliana_place(12, 32421, 32693, 12),
            EmilianaPlace::OutsideCrimsonCourt
        );
        assert_eq!(
            emiliana_place(12, 32420, 32693, 11),
            EmilianaPlace::OutsideCrimsonCourt
        );
        assert_eq!(
            step(Npc::Emiliana, "help", None, NpcState::default(), 11)
                .ok_or("source NPC rule")?
                .topic,
            None
        );
        assert_eq!(
            step(Npc::Emiliana, "help", None, NpcState::default(), 12)
                .ok_or("source NPC rule")?
                .topic,
            Some(Topic::Start)
        );
        let state = NpcState {
            burning_mission: 1,
            ..NpcState::default()
        };
        assert_eq!(
            choose(
                Npc::Emiliana,
                "mission",
                None,
                state,
                12,
                EmilianaPlace::CrimsonCourt
            )
            .ok_or("source NPC rule")?
            .mutation,
            Some(Mutation::Briefing)
        );
        assert_eq!(
            choose(
                Npc::Emiliana,
                "mission",
                None,
                state,
                12,
                EmilianaPlace::OutsideCrimsonCourt
            ),
            None
        );
        Ok(())
    }

    #[test]
    fn emiliana_reward_and_replacement_are_separate_and_surface_only() -> Result<(), String> {
        let state = NpcState {
            burning_mission: 3,
            herald_killed: 1,
            ..NpcState::default()
        };
        assert_eq!(
            step(Npc::Emiliana, "mission", None, state, 12)
                .ok_or("source NPC rule")?
                .topic,
            Some(Topic::Reward)
        );
        assert_eq!(
            step(Npc::Emiliana, "yes", Some(Topic::Reward), state, 12)
                .ok_or("source NPC rule")?
                .mutation,
            Some(Mutation::TurnipsReward)
        );
        assert_eq!(
            choose(
                Npc::Emiliana,
                "mission",
                None,
                state,
                12,
                EmilianaPlace::CrimsonCourt
            )
            .ok_or("source NPC rule")?
            .mutation,
            None
        );
        assert_eq!(
            step(Npc::Emiliana, "lose", None, state, 12)
                .ok_or("source NPC rule")?
                .topic,
            None
        );
        let state = NpcState {
            turnips_reward: 1,
            burning_mission: 5,
            ..state
        };
        assert_eq!(
            step(Npc::Emiliana, "lose", None, state, 12)
                .ok_or("source NPC rule")?
                .topic,
            Some(Topic::ReplaceTurnips)
        );
        assert_eq!(
            step(Npc::Emiliana, "yes", Some(Topic::ReplaceTurnips), state, 12)
                .ok_or("source NPC rule")?
                .mutation,
            Some(Mutation::ReplaceTurnips)
        );
        Ok(())
    }
}
