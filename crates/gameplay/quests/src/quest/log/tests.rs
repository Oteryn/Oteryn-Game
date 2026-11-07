#![allow(clippy::expect_used)]

use std::collections::BTreeMap;

use super::super::QuestTrack;
use super::*;

const QUEST: &str = "oteryn:quest/test.rats";
const OTHER: &str = "oteryn:quest/test.addon";
const START: &str = "oteryn:quest-progress/test.rats.start";
const STAGE: &str = "oteryn:quest-progress/test.rats.stage";
const KILLS: &str = "oteryn:quest-progress/test.rats.kills";
const ADDON: &str = "oteryn:quest-progress/test.addon.stage";

#[derive(Default)]
struct Session {
    tracks: BTreeMap<String, i64>,
    states: BTreeMap<String, (String, bool)>,
}

impl Session {
    fn with(tracks: &[(&str, i64)]) -> Self {
        Self {
            tracks: tracks
                .iter()
                .map(|(track, value)| ((*track).to_owned(), *value))
                .collect(),
            states: BTreeMap::new(),
        }
    }

    fn state(mut self, quest: &str, revision: &str, completed: bool) -> Self {
        self.states
            .insert(quest.to_owned(), (revision.to_owned(), completed));
        self
    }
}

impl QuestLogFacts for Session {
    fn track_value(&self, track: &str) -> Option<i64> {
        self.tracks.get(track).copied()
    }

    fn quest_state(&self, quest: &str) -> Option<QuestLogState<'_>> {
        self.states
            .get(quest)
            .map(|(revision, completed)| QuestLogState {
                pinned_content_revision: revision,
                completed: *completed,
            })
    }
}

fn track(key: &str, quest: &str, initial: i64) -> QuestTrack {
    QuestTrack {
        key: key.into(),
        quest: quest.into(),
        initial,
        min: -1,
        max: 1_000,
    }
}

fn state(revision: &str) -> QuestStateCatalogue {
    QuestStateCatalogue::new(
        revision,
        vec![
            track(START, QUEST, 0),
            track(STAGE, QUEST, 0),
            track(KILLS, QUEST, 0),
            track(ADDON, OTHER, 0),
        ],
        vec![],
    )
    .expect("state catalogue")
}

fn text(text: &str) -> JournalText {
    JournalText::Text(text.into())
}

fn mission(
    name: &str,
    track: &str,
    start: i64,
    end: i64,
    journal: QuestJournal,
) -> QuestLogMission {
    QuestLogMission {
        name: name.into(),
        track: track.into(),
        start_value: start,
        end_value: end,
        journal,
    }
}

/// The rats quest: listed once `START >= 1`; two missions on `STAGE` and one counter mission.
fn rats(stage_text: &str) -> QuestLogQuest {
    QuestLogQuest {
        key: QUEST.into(),
        kind: QuestLogKind::Storyline,
        shown_in_quest_log: true,
        start: Some((START.into(), 1)),
        hide_when_completed: false,
        missions: vec![
            mission(
                "Find the rats",
                STAGE,
                1,
                2,
                QuestJournal::PerStage(BTreeMap::from([
                    (1, text(stage_text)),
                    (2, text("The rats are found.")),
                ])),
            ),
            mission(
                "Report back",
                STAGE,
                2,
                3,
                QuestJournal::Fixed(text("Report to the guard.")),
            ),
            mission(
                "Hunt rats",
                KILLS,
                0,
                5,
                QuestJournal::Fixed(JournalText::Template(vec![
                    JournalPiece::Text("You hunted ".into()),
                    JournalPiece::Track(KILLS.into()),
                    JournalPiece::Text("/5 rats.".into()),
                ])),
            ),
        ],
    }
}

/// An addon quest: hidden once completed.
fn addon() -> QuestLogQuest {
    QuestLogQuest {
        key: OTHER.into(),
        kind: QuestLogKind::Storyline,
        shown_in_quest_log: true,
        start: Some((ADDON.into(), 1)),
        hide_when_completed: true,
        missions: vec![mission("Addon", ADDON, 1, 2, QuestJournal::None)],
    }
}

fn content() -> QuestLogContent {
    let active = QuestLogCatalogue::new(&state("content-2"), vec![rats("Find them."), addon()])
        .expect("active");
    let pinned = QuestLogCatalogue::new(&state("content-1"), vec![rats("Old text."), addon()])
        .expect("pinned");
    QuestLogContent::new(active, vec![pinned])
}

/// The canonical indexes: ascending Oteryn keys, so the addon quest is 0 and rats 1.
const ADDON_INDEX: u32 = 0;
const RATS_INDEX: u32 = 1;

#[test]
fn quests_are_indexed_by_ascending_key() {
    let content = content();
    let keys: Vec<&str> = content
        .active()
        .quests()
        .iter()
        .map(|quest| quest.key.as_str())
        .collect();
    assert_eq!(keys, [OTHER, QUEST]);
    // The active revision first, then the loadable pinned ones.
    let revisions: Vec<&str> = content
        .catalogues()
        .map(QuestLogCatalogue::content_revision)
        .collect();
    assert_eq!(revisions, ["content-2", "content-1"]);
}

#[test]
fn a_quest_is_listed_once_its_start_track_reaches_its_value() {
    let content = content();
    assert!(
        content.list(&Session::default()).is_empty(),
        "initial 0 < 1"
    );
    assert_eq!(
        content.list(&Session::with(&[(START, 1)])),
        [ListedQuest {
            index: RATS_INDEX,
            completed: false
        }]
    );
    assert_eq!(
        content
            .list(&Session::with(&[(START, 7), (ADDON, 1)]))
            .len(),
        2
    );
    assert!(!content.is_listed(&Session::with(&[(START, 0)]), RATS_INDEX));
    assert!(content.line(&Session::default(), RATS_INDEX).is_none());
    assert!(
        content.line(&Session::with(&[(START, 1)]), 2).is_none(),
        "unknown"
    );
    assert!(
        content
            .line(&Session::with(&[(START, 1)]), u32::MAX)
            .is_none()
    );
}

#[test]
fn reward_only_script_only_unshown_and_startless_quests_are_never_listed() {
    let started = Session::with(&[(START, 5)]);
    let changes: [fn(&mut QuestLogQuest); 4] = [
        |quest: &mut QuestLogQuest| quest.kind = QuestLogKind::RewardOnly,
        |quest: &mut QuestLogQuest| quest.kind = QuestLogKind::ScriptOnly,
        |quest: &mut QuestLogQuest| quest.shown_in_quest_log = false,
        |quest: &mut QuestLogQuest| quest.start = None,
    ];
    for change in changes {
        let mut quest = rats("Find them.");
        change(&mut quest);
        let content = QuestLogContent::new(
            QuestLogCatalogue::new(&state("content-2"), vec![quest]).expect("catalogue"),
            vec![],
        );
        assert!(content.list(&started).is_empty());
        assert!(content.line(&started, 0).is_none());
    }
}

#[test]
fn completion_is_flagged_and_hide_when_completed_leaves_the_list() {
    let content = content();
    let copy = Session::with(&[(START, 1), (ADDON, 2)])
        .state(QUEST, "content-1", true)
        .state(OTHER, "content-1", true);
    assert_eq!(
        content.list(&copy),
        [ListedQuest {
            index: RATS_INDEX,
            completed: true
        }],
        "the completed addon quest leaves the list"
    );
    assert!(content.line(&copy, ADDON_INDEX).is_none());
    // Before completion it is listed.
    let in_progress = Session::with(&[(ADDON, 1)]).state(OTHER, "content-2", false);
    assert_eq!(
        content.list(&in_progress),
        [ListedQuest {
            index: ADDON_INDEX,
            completed: false
        }]
    );
}

#[test]
fn missions_show_within_their_range_done_at_the_end_with_their_journal_entry() {
    let content = content();
    let line = |stage: i64, kills: i64| {
        content
            .line(
                &Session::with(&[(START, 1), (STAGE, stage), (KILLS, kills)]),
                RATS_INDEX,
            )
            .expect("listed")
    };
    let at_one = line(1, 3);
    assert_eq!(
        at_one.missions,
        [
            VisibleMission {
                index: 0,
                name: "Find the rats".into(),
                text: "Find them.".into(),
                done: false
            },
            VisibleMission {
                index: 2,
                name: "Hunt rats".into(),
                text: "You hunted 3/5 rats.".into(),
                done: false
            },
        ]
    );
    // At 2 the first mission is done and the next one starts.
    let at_two = line(2, 5);
    let shown: Vec<(u32, &str, bool)> = at_two
        .missions
        .iter()
        .map(|mission| (mission.index, mission.text.as_str(), mission.done))
        .collect();
    assert_eq!(
        shown,
        [
            (0, "The rats are found.", true),
            (1, "Report to the guard.", false),
            (2, "You hunted 5/5 rats.", true),
        ]
    );
    // Below its start or above its end a mission is not shown.
    assert_eq!(line(0, 6).missions, []);
    let at_three: Vec<u32> = line(3, 6).missions.iter().map(|m| m.index).collect();
    assert_eq!(at_three, [1]);
    assert!(line(3, 6).missions[0].done);
}

#[test]
fn a_stage_without_an_entry_and_a_journal_of_none_show_no_text() {
    let mut quest = rats("Find them.");
    quest.missions[0].journal = QuestJournal::PerStage(BTreeMap::from([(2, text("Found."))]));
    quest.missions[1].journal = QuestJournal::None;
    let content = QuestLogContent::new(
        QuestLogCatalogue::new(&state("content-2"), vec![quest]).expect("catalogue"),
        vec![],
    );
    let line = content
        .line(&Session::with(&[(START, 1), (STAGE, 1), (KILLS, 9)]), 0)
        .expect("listed");
    assert_eq!(line.missions.len(), 1);
    assert_eq!(line.missions[0].text, "");
    let line = content
        .line(&Session::with(&[(START, 1), (STAGE, 2), (KILLS, 9)]), 0)
        .expect("listed");
    assert_eq!(line.missions[1].text, "");
}

#[test]
fn an_in_progress_quest_resolves_against_its_pinned_revision() {
    let content = content();
    let values = [(START, 1), (STAGE, 1)];
    let text = |copy: &Session| {
        content.line(copy, RATS_INDEX).expect("listed").missions[0]
            .text
            .clone()
    };
    // No state row: the active revision.
    assert_eq!(text(&Session::with(&values)), "Find them.");
    // In progress, pinned to content-1: its journal, never the newer one.
    assert_eq!(
        text(&Session::with(&values).state(QUEST, "content-1", false)),
        "Old text."
    );
    // Completed: the active revision again.
    assert_eq!(
        text(&Session::with(&values).state(QUEST, "content-1", true)),
        "Find them."
    );
    // A pinned revision that is not loadable hides the quest.
    let unloadable = Session::with(&values).state(QUEST, "content-0", false);
    assert!(content.line(&unloadable, RATS_INDEX).is_none());
    assert!(content.list(&unloadable).is_empty());
    // A pinned revision without the quest hides it too.
    let without = QuestLogContent::new(
        QuestLogCatalogue::new(&state("content-2"), vec![rats("Find them.")]).expect("active"),
        vec![QuestLogCatalogue::new(&state("content-1"), vec![addon()]).expect("pinned")],
    );
    let pinned = Session::with(&values).state(QUEST, "content-1", false);
    assert!(without.line(&pinned, 0).is_none());
}

#[test]
fn the_pinned_revision_decides_missions_ranges_and_start() {
    let mut old = rats("Old text.");
    old.start = Some((START.into(), 5));
    old.missions[0].end_value = 9;
    let content = QuestLogContent::new(
        QuestLogCatalogue::new(&state("content-2"), vec![rats("Find them.")]).expect("active"),
        vec![QuestLogCatalogue::new(&state("content-1"), vec![old]).expect("pinned")],
    );
    let copy =
        |start| Session::with(&[(START, start), (STAGE, 2)]).state(QUEST, "content-1", false);
    assert!(content.line(&copy(4), 0).is_none(), "the pinned start is 5");
    let line = content.line(&copy(5), 0).expect("listed");
    assert!(!line.missions[0].done, "the pinned end value is 9");
}

#[test]
fn a_track_without_a_row_reads_its_declared_initial_value() {
    let state = QuestStateCatalogue::new(
        "content-2",
        vec![
            track(START, QUEST, 1),
            track(STAGE, QUEST, 1),
            track(KILLS, QUEST, 0),
        ],
        vec![],
    )
    .expect("state");
    let content = QuestLogContent::new(
        QuestLogCatalogue::new(&state, vec![rats("Find them.")]).expect("catalogue"),
        vec![],
    );
    let line = content
        .line(&Session::default(), 0)
        .expect("listed at initial 1");
    assert_eq!(line.missions[0].text, "Find them.");
    assert_eq!(line.missions[1].text, "You hunted 0/5 rats.");
}

#[test]
fn a_catalogue_naming_undeclared_or_foreign_tracks_does_not_load() {
    use QuestLogCatalogueError as E;
    let build = |quest: QuestLogQuest| QuestLogCatalogue::new(&state("content-2"), vec![quest]);
    let mut unknown = rats("x");
    unknown.key = "oteryn:quest/test.none".into();
    assert!(matches!(build(unknown), Err(E::UnknownQuest(_))));
    let mut foreign = rats("x");
    foreign.missions[0].track = ADDON.into();
    assert!(matches!(build(foreign), Err(E::UnknownTrack(_))));
    let mut foreign_start = rats("x");
    foreign_start.start = Some((ADDON.into(), 1));
    assert!(matches!(build(foreign_start), Err(E::UnknownTrack(_))));
    let mut undeclared = rats("x");
    undeclared.missions[2].journal =
        QuestJournal::Fixed(JournalText::Template(vec![JournalPiece::Track(
            "oteryn:quest-progress/test.none".into(),
        )]));
    assert!(matches!(build(undeclared), Err(E::UnknownTrack(_))));
    assert!(matches!(
        QuestLogCatalogue::new(&state("content-2"), vec![rats("x"), rats("y")]),
        Err(E::DuplicateQuest(_))
    ));
}

#[test]
fn a_catalogue_over_the_bounds_does_not_load() {
    use QuestLogCatalogueError as E;
    let build = |quest: QuestLogQuest| QuestLogCatalogue::new(&state("content-2"), vec![quest]);
    let with_missions = |count: usize| {
        let mut quest = rats("x");
        quest.missions = (0..count)
            .map(|n| mission(&format!("Mission {n}"), STAGE, 0, 1, QuestJournal::None))
            .collect();
        quest
    };
    assert!(build(with_missions(QUESTGATE0_RL_08)).is_ok());
    assert_eq!(
        build(with_missions(QUESTGATE0_RL_08 + 1)),
        Err(E::CapacityExceeded)
    );
    let named = |name: String| {
        let mut quest = rats("x");
        quest.missions[0].name = name;
        build(quest)
    };
    assert!(named("n".repeat(QUESTGATE0_RL_08_NAME)).is_ok());
    for name in [String::new(), "n".repeat(QUESTGATE0_RL_08_NAME + 1)] {
        assert!(matches!(named(name), Err(E::InvalidMission(_))));
    }
    let mut inverted = rats("x");
    inverted.missions[0].start_value = 3;
    assert!(matches!(build(inverted), Err(E::InvalidMission(_))));
    // The text bound counts a template counter as its longest decimal.
    let journal = |pieces: Vec<JournalPiece>| {
        let mut quest = rats("x");
        quest.missions[2].journal =
            QuestJournal::PerStage(BTreeMap::from([(1, JournalText::Template(pieces))]));
        build(quest)
    };
    let filler = |len: usize| JournalPiece::Text("t".repeat(len));
    let counter = || JournalPiece::Track(KILLS.into());
    assert!(
        journal(vec![
            filler(QUESTGATE0_RL_08_TEXT - COUNTER_BYTES),
            counter()
        ])
        .is_ok()
    );
    assert!(matches!(
        journal(vec![
            filler(QUESTGATE0_RL_08_TEXT - COUNTER_BYTES + 1),
            counter()
        ]),
        Err(E::InvalidMission(_))
    ));
    assert!(matches!(
        build({
            let mut quest = rats("x");
            quest.missions[1].journal =
                QuestJournal::Fixed(text(&"t".repeat(QUESTGATE0_RL_08_TEXT + 1)));
            quest
        }),
        Err(E::InvalidMission(_))
    ));
    assert_eq!(
        worst_case_text_bytes(&JournalText::Template(vec![filler(3), counter()])),
        3 + COUNTER_BYTES
    );
}

#[test]
fn at_most_1024_quests_are_listable() {
    let listable = |count: usize| {
        let mut tracks = Vec::new();
        let mut quests = Vec::new();
        for n in 0..count {
            let key = format!("oteryn:quest/test.q{n:05}");
            let start = format!("oteryn:quest-progress/test.q{n:05}");
            tracks.push(track(&start, &key, 0));
            quests.push(QuestLogQuest {
                key,
                kind: QuestLogKind::Storyline,
                shown_in_quest_log: true,
                start: Some((start, 1)),
                hide_when_completed: false,
                missions: Vec::new(),
            });
        }
        // An unlisted quest does not count.
        let mut hidden = quests[0].clone();
        hidden.key = "oteryn:quest/test.hidden".into();
        hidden.start = None;
        tracks.push(track("oteryn:quest-progress/test.hidden", &hidden.key, 0));
        quests.push(hidden);
        let state = QuestStateCatalogue::new("content-2", tracks, vec![]).expect("state");
        QuestLogCatalogue::new(&state, quests)
    };
    assert!(listable(QUESTGATE0_RL_07).is_ok());
    assert_eq!(
        listable(QUESTGATE0_RL_07 + 1),
        Err(QuestLogCatalogueError::CapacityExceeded)
    );
}

#[test]
fn the_worst_case_line_reads_the_longest_entry_of_each_mission() {
    let quest = rats("Find them now.");
    let sizes: Vec<(usize, usize)> = quest.worst_case_missions().collect();
    assert_eq!(
        sizes,
        [
            ("Find the rats".len(), "The rats are found.".len()),
            ("Report back".len(), "Report to the guard.".len()),
            (
                "Hunt rats".len(),
                "You hunted ".len() + COUNTER_BYTES + "/5 rats.".len()
            ),
        ]
    );
}
