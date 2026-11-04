//! Jack Springer's Crystal201189f branches. Durable effects use the existing
//! QuestState schema; native dispatch owns the exact NPC and topic authority.

use serde::Deserialize;
use std::collections::BTreeSet;
use std::sync::OnceLock;

pub const NPC: &str = "oteryn:npc.jack_springer";
pub const QUESTLINE: &str = "oteryn:quest-progress/quest/u12_20/grave_danger/questline";
pub const DOOR: &str =
    "oteryn:quest-progress/crystalserver/quest/u12_20/grave_danger/bosses/king_zelos_door";
pub const OUTFIT: &str = "oteryn:quest-progress/crystalserver/quest/u12_20/grave_danger/bosses/inquisition_outfit_received";
pub const START: &str = "oteryn:quest-transition/grave-danger/start";
pub const ACCESS: &str = "oteryn:quest-transition/grave-danger/zelos-access";
pub const FINISH: &str = "oteryn:quest-transition/grave-danger/final-report";
pub const FINISH_HELD: &str = "oteryn:quest-transition/grave-danger/final-report-outfit-held";
pub const ACHIEVEMENT: &str = "Inquisition's Hand";
const DIALOGUE: &str = include_str!("../../../../../content/dialogues/grave-danger.json");
const STATE: &str = include_str!("../../../../../content/quests/grave-danger-npc-state.json");
const PIN: &str = "201189f871d23597b82caf88fd611454a2ddefb8";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Topic {
    Ultimate,
    Threat,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Reply {
    FinalReport,
    ReportReady,
    ReportGoodLuck,
    ReportMissing,
    Ultimate,
    Threat,
    Start,
    BasicDiscuss,
    BasicUrgency,
    BasicStart,
    BasicSource,
    BasicEvents,
    BasicMore,
    BasicMany,
    BasicIceberg,
    BasicHand,
    BasicPredates,
    BasicLanguage,
    BasicLegion,
    BasicGoal,
    BasicStopped,
    BasicClashes,
    BasicHurry,
    BasicProblem,
    BasicKnights,
    BasicLich,
    BasicScheme,
    BasicThreats,
    BasicRituals,
    BasicPurge,
    BasicLocations,
    BasicName,
    BasicJob,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    pub reply: Option<Reply>,
    pub transition: Option<&'static str>,
    pub topic: Option<Topic>,
    /// Source branch retained, but its indivisible reward owner is not admitted.
    pub deferred: bool,
}

// npc/jack_springer.lua:155-230, in registration order. The keyword handler
// runs before the default callback (npc_system/npc_handler.lua:448-451) and
// uses string.find, not MsgContains (keyword_handler.lua:58-67).
const BASIC: &[(&str, Reply)] = &[
    ("discuss", Reply::BasicDiscuss),
    ("urgency", Reply::BasicUrgency),
    ("start", Reply::BasicStart),
    ("source", Reply::BasicSource),
    ("events", Reply::BasicEvents),
    ("more", Reply::BasicMore),
    ("many", Reply::BasicMany),
    ("iceberg", Reply::BasicIceberg),
    ("hand", Reply::BasicHand),
    ("predates", Reply::BasicPredates),
    ("language", Reply::BasicLanguage),
    ("legion", Reply::BasicLegion),
    ("goal", Reply::BasicGoal),
    ("stopped", Reply::BasicStopped),
    ("clashes", Reply::BasicClashes),
    ("hurry", Reply::BasicHurry),
    ("problem", Reply::BasicProblem),
    ("knights", Reply::BasicKnights),
    ("lich", Reply::BasicLich),
    ("scheme", Reply::BasicScheme),
    ("threats", Reply::BasicThreats),
    ("rituals", Reply::BasicRituals),
    ("purge", Reply::BasicPurge),
    ("locations", Reply::BasicLocations),
    ("name", Reply::BasicName),
    ("job", Reply::BasicJob),
];

fn contains(message: &str, keyword: &str) -> bool {
    message == keyword
        || (message.contains(keyword)
            && !message.match_indices(keyword).any(|(start, _)| {
                start > 0 && message.as_bytes()[start - 1].is_ascii_alphanumeric()
            }))
}

#[must_use]
pub fn choose(message: &str, topic: Option<Topic>, questline: i64, _outfit: i64) -> Option<Choice> {
    let message = message.to_ascii_lowercase();
    let choice = |reply, transition, topic| {
        Some(Choice {
            reply: Some(reply),
            transition,
            topic,
            deferred: false,
        })
    };
    if let Some((_, reply)) = BASIC.iter().find(|(keyword, _)| message.contains(keyword)) {
        return choice(*reply, None, topic);
    }
    if super::quests_2_focus::intercepts(&message) {
        return None;
    }
    // Crystal npc/jack_springer.lua:79-123 returns immediately in these branches.
    if contains(&message, "report") {
        match questline {
            // No completion or OutfitReceived flag without the admitted reward.
            // https://github.com/Oteryn/Oteryn-Game/pull/2844#issuecomment-6589624980
            4 => Some(Choice {
                reply: None,
                transition: None,
                topic,
                deferred: true,
            }),
            2 => choice(Reply::ReportReady, None, Some(Topic::Ultimate)),
            3.. => choice(Reply::ReportGoodLuck, None, topic),
            1 => choice(Reply::ReportMissing, None, topic),
            _ => None,
        }
    } else if contains(&message, "ultimate") && topic == Some(Topic::Ultimate) {
        choice(Reply::Ultimate, None, Some(Topic::Threat))
    } else if contains(&message, "threat") && topic == Some(Topic::Threat) {
        choice(Reply::Threat, (questline == 2).then_some(ACCESS), None)
    } else if contains(&message, "late") && questline < 1 {
        // The13-effect start is source-only under the coordinator's point5:
        // https://github.com/Oteryn/Oteryn-Game/pull/2844#issuecomment-6589521017.
        // No success reply and no partial reset may escape this closed policy.
        Some(Choice {
            reply: None,
            transition: None,
            topic,
            deferred: true,
        })
    } else {
        None
    }
}

#[derive(Deserialize)]
struct Document {
    schema: String,
    family: String,
    records: Vec<Record>,
}
#[derive(Deserialize)]
struct Record {
    declaration: Declaration,
}
#[derive(Deserialize)]
struct Declaration {
    identity: Identity,
    greet: Vec<String>,
    keywords: Vec<Keyword>,
    fields: Vec<Field>,
}
#[derive(Deserialize)]
struct Identity {
    key: String,
}
#[derive(Deserialize)]
struct Keyword {
    key: Reply,
    reply: Vec<String>,
}
#[derive(Deserialize)]
struct Field {
    field_path: String,
    value: Text,
}
#[derive(Deserialize)]
struct Text {
    value: String,
}
#[derive(Deserialize)]
struct Source {
    repository: String,
    revision: String,
    path: String,
}

struct Dialogue {
    keywords: Vec<Keyword>,
    greeting: Vec<String>,
    started_greeting: Vec<String>,
}

fn parse_dialogue(text: &str) -> Result<Dialogue, String> {
    let mut document: Document = serde_json::from_str(text).map_err(|error| error.to_string())?;
    if document.schema != "OTERYN_DIALOGUE_AUTHORING_SHARD/v1"
        || document.family != "Dialogue"
        || document.records.len() != 1
    {
        return Err("Grave Danger dialogue envelope".into());
    }
    let declaration = document
        .records
        .pop()
        .ok_or("Grave Danger dialogue record")?
        .declaration;
    if declaration.identity.key != "oteryn:dialogue.npc.jack_springer" {
        return Err("Grave Danger dialogue NPC".into());
    }
    let source = declaration
        .fields
        .iter()
        .find(|field| field.field_path == "oteryn:source.quest.grave_danger.dialogue")
        .ok_or("Grave Danger dialogue source")?;
    let source: Source =
        serde_json::from_str(&source.value.value).map_err(|error| error.to_string())?;
    if source.repository != "zimbadev/crystalserver"
        || source.revision != PIN
        || source.path != "data-global/npc/jack_springer.lua"
    {
        return Err("Grave Danger dialogue pin".into());
    }
    let keys: BTreeSet<_> = declaration.keywords.iter().map(|entry| entry.key).collect();
    if declaration.keywords.len() != 33
        || keys.len() != 33
        || declaration.keywords.iter().any(|entry| {
            entry.reply.is_empty()
                || entry.reply.len() > 6
                || entry
                    .reply
                    .iter()
                    .any(|line| line.is_empty() || line.len() > 2048 || line.contains(['\0', '\r']))
        })
    {
        return Err("Grave Danger dialogue replies".into());
    }
    let started = declaration
        .fields
        .iter()
        .find(|field| field.field_path == "oteryn:source.quest.grave_danger.started_greeting")
        .ok_or("Grave Danger started greeting")?
        .value
        .value
        .clone();
    if declaration.greet.len() != 1
        || declaration
            .greet
            .iter()
            .chain(std::iter::once(&started))
            .any(|line| line.is_empty() || line.len() > 2048 || line.contains(['\0', '\r']))
    {
        return Err("Grave Danger greetings".into());
    }
    Ok(Dialogue {
        keywords: declaration.keywords,
        greeting: declaration.greet,
        started_greeting: vec![started],
    })
}

fn dialogue() -> Option<&'static Dialogue> {
    static DATA: OnceLock<Result<Dialogue, String>> = OnceLock::new();
    DATA.get_or_init(|| parse_dialogue(DIALOGUE)).as_ref().ok()
}

/// Crystal jack_springer.lua:64-67 selects this on a new greeting only.
#[must_use]
pub fn greeting(questline: i64) -> Option<&'static [String]> {
    let data = dialogue()?;
    Some(if questline < 1 {
        &data.greeting
    } else {
        &data.started_greeting
    })
}

#[must_use]
pub fn reply(key: Reply) -> Option<&'static [String]> {
    dialogue()?
        .keywords
        .iter()
        .find(|entry| entry.key == key)
        .map(|entry| entry.reply.as_slice())
}

pub(super) fn extend(
    catalogue: super::QuestStateCatalogue,
) -> Result<super::QuestStateCatalogue, String> {
    let extra = super::loader::parse_quest_state(STATE, &catalogue.content_revision)
        .map_err(|error| format!("{error:?}"))?;
    let mut tracks = catalogue.tracks.values().cloned().collect::<Vec<_>>();
    for track in extra.catalogue().tracks.values() {
        if let Some(existing) = tracks.iter_mut().find(|existing| existing.key == track.key) {
            if existing.quest != track.quest
                || existing.initial != track.initial
                || track.min > existing.min
                || track.max < existing.max
            {
                return Err(format!("Grave Danger conflicting track {}", track.key));
            }
            *existing = track.clone();
        } else {
            tracks.push(track.clone());
        }
    }
    let mut transitions = catalogue.transitions.values().cloned().collect::<Vec<_>>();
    transitions.extend(extra.catalogue().transitions.values().cloned());
    super::QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|error| format!("{error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn source_focus_precedes_grave_default_callback_but_not_earlier_lore() {
        // Crystal201189f jack_springer.lua:155-230 registers BASIC first, then
        // FocusModule at232; npc_handler.lua:446-451 invokes default last.
        for message in ["threat hi", "threat hello", "threat offers"] {
            assert_eq!(choose(message, Some(Topic::Threat), 2, -1), None);
        }
        assert_eq!(
            choose("name hi", Some(Topic::Threat), 2, -1),
            Some(Choice {
                reply: Some(Reply::BasicName),
                transition: None,
                topic: Some(Topic::Threat),
                deferred: false,
            }),
        );
    }

    #[test]
    fn zelos_access_requires_the_report_ultimate_threat_topic_sequence() {
        assert_eq!(choose("ultimate", None, 2, -1), None);
        assert_eq!(choose("threat", None, 2, -1), None);
        let topic = choose("report", None, 2, -1).and_then(|choice| choice.topic);
        assert_eq!(topic, Some(Topic::Ultimate));
        let topic = choose("ultimate", topic, 2, -1).and_then(|choice| choice.topic);
        assert_eq!(topic, Some(Topic::Threat));
        assert_eq!(
            choose("threat", topic, 2, -1).and_then(|choice| choice.transition),
            Some(ACCESS)
        );
        assert_eq!(
            choose("threat", topic, 3, -1).and_then(|choice| choice.transition),
            None
        );
    }

    #[test]
    fn unadmitted_start_and_final_defer_the_whole_source_change() {
        for questline in [-1, 0] {
            assert_eq!(
                choose("late", None, questline, -1),
                Some(Choice {
                    reply: None,
                    transition: None,
                    topic: None,
                    deferred: true
                })
            );
        }
        assert_eq!(choose("late", None, 1, -1), None);
        for outfit in [-1, 0, 1] {
            let decision = choose("report", None, 4, outfit);
            assert_eq!(
                decision,
                Some(Choice {
                    reply: None,
                    transition: None,
                    topic: None,
                    deferred: true
                })
            );
        }
        assert_eq!(
            choose("report", None, 3, -1).and_then(|choice| choice.transition),
            None
        );
        assert_eq!(
            choose("report", None, 5, 1).and_then(|choice| choice.transition),
            None
        );
    }

    #[test]
    fn basic_keyword_handler_precedes_and_differs_from_msg_contains() {
        assert_eq!(
            choose("threats", Some(Topic::Threat), 2, -1).and_then(|choice| choice.reply),
            Some(Reply::BasicThreats)
        );
        assert_eq!(
            choose("restart", None, -1, -1).and_then(|choice| choice.reply),
            Some(Reply::BasicStart)
        );
        assert_eq!(
            choose("report ultimate", None, 2, -1).and_then(|choice| choice.reply),
            Some(Reply::ReportReady)
        );
        assert_eq!(
            choose("many report", None, 2, -1).and_then(|choice| choice.reply),
            Some(Reply::BasicMany)
        );
    }

    #[test]
    fn all_thirty_three_source_branches_and_six_ritual_lines_are_retained() -> Result<(), String> {
        assert_eq!(parse_dialogue(DIALOGUE)?.keywords.len(), 33);
        assert_eq!(reply(Reply::Threat).ok_or("threat")?.len(), 6);
        assert!(reply(Reply::FinalReport).ok_or("final")?[0].contains("crippeled"));
        assert!(parse_dialogue(&DIALOGUE.replace(PIN, "unverified")).is_err());
        Ok(())
    }

    #[test]
    fn greeting_uses_the_source_stage_boundary_and_retains_the_name_template() -> Result<(), String>
    {
        let unstarted = greeting(0).ok_or("unstarted greeting")?;
        assert!(unstarted[0].contains("{discuss}"));
        assert_eq!(greeting(-1), greeting(0));
        let started = greeting(1).ok_or("started greeting")?;
        assert!(started[0].contains("{report}"));
        assert!(started[0].contains("|PLAYERNAME|"));
        assert_eq!(greeting(1), greeting(5));
        Ok(())
    }

    #[test]
    fn real_catalogue_opens_zelos_door_atomically_and_holds_unadmitted_start_and_final()
    -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("grave-danger")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = lowered.catalogue();
        let transition = catalogue.transition(ACCESS).ok_or("access")?;
        let mut tracks = BTreeMap::from([(QUESTLINE.into(), 2)]);
        let changes = catalogue
            .evaluate(transition, &tracks, 0)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(changes.len(), 2);
        for change in changes {
            tracks.insert(change.track, change.after);
        }
        assert_eq!(tracks.get(QUESTLINE), Some(&3));
        assert_eq!(tracks.get(DOOR), Some(&1));
        assert!(!tracks.contains_key(OUTFIT));
        assert_eq!(
            catalogue.evaluate(transition, &tracks, 0),
            Err(super::super::QuestRefusal::StageMismatch)
        );
        assert!(
            catalogue.transition(START).is_none(),
            "Q18's13-effect start must not be partially admitted"
        );
        assert!(catalogue.transition(FINISH).is_none());
        assert!(catalogue.transition(FINISH_HELD).is_none());
        Ok(())
    }
}
