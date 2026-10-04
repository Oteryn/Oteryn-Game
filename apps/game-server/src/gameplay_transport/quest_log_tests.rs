#![allow(clippy::expect_used)]

use std::collections::BTreeMap;

use super::*;
use crate::durability::quest_state::quest::log::{
    JournalText, QuestJournal, QuestLogCatalogue, QuestLogKind, QuestLogMission, QuestLogQuest,
};
use crate::durability::quest_state::quest::{QuestStateCatalogue, QuestTrack};
use oteryn_protocol_oteryn::quest_log::{
    MAX_JOURNAL_TEXT_BYTES, MAX_MISSION_NAME_BYTES, decode_quest_log,
};

const RATS: &str = "oteryn:quest/test.rats";
const TOMB: &str = "oteryn:quest/test.tomb";
const RATS_START: &str = "oteryn:quest-progress/test.rats.start";
const RATS_STAGE: &str = "oteryn:quest-progress/test.rats.stage";
const TOMB_START: &str = "oteryn:quest-progress/test.tomb.start";

/// Canonical indexes: ascending keys.
const RATS_INDEX: u32 = 0;
const TOMB_INDEX: u32 = 1;

/// A session copy as the owner would hold it.
#[derive(Debug, Default, Clone)]
struct Facts {
    tracks: BTreeMap<String, i64>,
    states: BTreeMap<String, (String, bool)>,
}

impl Facts {
    fn with(tracks: &[(&str, i64)]) -> Self {
        Self {
            tracks: tracks
                .iter()
                .map(|(track, value)| ((*track).to_owned(), *value))
                .collect(),
            states: BTreeMap::new(),
        }
    }
}

impl QuestLogFacts for Facts {
    fn track_value(&self, track: &str) -> Option<i64> {
        self.tracks.get(track).copied()
    }

    fn quest_state(&self, quest: &str) -> Option<CopyQuestState<'_>> {
        self.states
            .get(quest)
            .map(|(revision, completed)| CopyQuestState {
                pinned_content_revision: revision,
                completed: *completed,
            })
    }
}

fn track(key: &str, quest: &str) -> QuestTrack {
    QuestTrack {
        key: key.into(),
        quest: quest.into(),
        initial: 0,
        min: 0,
        max: 10,
    }
}

fn quest(key: &str, start: &str, missions: Vec<QuestLogMission>) -> QuestLogQuest {
    QuestLogQuest {
        key: key.into(),
        kind: QuestLogKind::Storyline,
        shown_in_quest_log: true,
        start: Some((start.into(), 1)),
        hide_when_completed: false,
        missions,
    }
}

fn mission(name: &str, track: &str, start: i64, end: i64, text: &str) -> QuestLogMission {
    QuestLogMission {
        name: name.into(),
        track: track.into(),
        start_value: start,
        end_value: end,
        journal: QuestJournal::Fixed(JournalText::Text(text.into())),
    }
}

fn state_catalogue() -> QuestStateCatalogue {
    QuestStateCatalogue::new(
        "content-1",
        vec![
            track(RATS_START, RATS),
            track(RATS_STAGE, RATS),
            track(TOMB_START, TOMB),
        ],
        vec![],
    )
    .expect("state catalogue")
}

fn content() -> QuestLogLines {
    let rats = quest(
        RATS,
        RATS_START,
        vec![mission("Rats", RATS_STAGE, 1, 2, "Hunt the rats.")],
    );
    let tomb = quest(
        TOMB,
        TOMB_START,
        vec![mission("Tomb", TOMB_START, 1, 3, "Enter the tomb.")],
    );
    let catalogue = QuestLogCatalogue::new(&state_catalogue(), vec![rats, tomb]).expect("log");
    QuestLogLines::new(QuestLogContent::new(catalogue, vec![])).expect("lines fit")
}

fn source(facts: Facts, version: u64) -> QuestLogSource {
    QuestLogSource {
        content: content(),
        copy: Arc::new(facts),
        version,
    }
}

fn changed(facts: Facts, version: u64) -> QuestLogObservation {
    QuestLogObservation::Changed(source(facts, version))
}

fn rats_line(text: &str, done: bool) -> QuestLine {
    QuestLine {
        quest_index: RATS_INDEX,
        completed: false,
        missions: vec![QuestMission {
            mission_index: 0,
            name: "Rats".into(),
            text: text.into(),
            done,
        }],
    }
}

fn shown(domain: &QuestLogDomain) -> QuestLog {
    decode_quest_log(&domain.payload).expect("domain 16 payload")
}

#[test]
fn queries_are_limited_to_2_per_second_per_game_session() {
    let mut window = QueryWindow::EMPTY;
    let start = Instant::now();
    // max = 2 within one second.
    assert!(window.admit(start));
    assert!(window.admit(start + Duration::from_millis(10)));
    // max + 1 within the window is refused, up to its last instant.
    assert!(!window.admit(start + Duration::from_millis(20)));
    assert!(!window.admit(start + QUEST_LOG_QUERY_WINDOW - Duration::from_millis(1)));
    // The window slides: the first query leaves it after one second.
    assert!(window.admit(start + QUEST_LOG_QUERY_WINDOW));
    assert!(!window.admit(start + QUEST_LOG_QUERY_WINDOW));
    assert!(window.admit(start + QUEST_LOG_QUERY_WINDOW + Duration::from_millis(10)));
}

#[test]
fn the_tracked_set_holds_10_quests_and_refuses_more() {
    let ten: Vec<u32> = (0..MAX_TRACKED_QUESTS as u32).collect();
    assert_eq!(
        TrackedQuests::new(&ten).expect("ten").as_slice(),
        ten.as_slice()
    );
    assert_eq!(TrackedQuests::new(&(0..11).collect::<Vec<_>>()), None);
    assert!(TrackedQuests::NONE.as_slice().is_empty());
}

#[test]
fn content_whose_worst_case_line_is_over_the_bound_does_not_load() {
    let missions = |count: usize| {
        (0..count)
            .map(|n| {
                mission(
                    &format!("{n:0>width$}", width = MAX_MISSION_NAME_BYTES),
                    RATS_STAGE,
                    0,
                    1,
                    &"t".repeat(MAX_JOURNAL_TEXT_BYTES),
                )
            })
            .collect::<Vec<_>>()
    };
    let build = |count: usize| {
        let catalogue = QuestLogCatalogue::new(
            &state_catalogue(),
            vec![quest(RATS, RATS_START, missions(count))],
        )
        .expect("each mission is within its own bounds");
        QuestLogLines::new(QuestLogContent::new(catalogue, vec![]))
    };
    // A worst-case mission element is 3 + 6 + 131 + 1027 + 2 = 1,169 bytes: 14 fit 16,384 with
    // the 8-byte header, 15 do not.
    assert!(build(14).is_some());
    assert!(build(15).is_none());
    // A pinned revision over the bound fails the whole content too.
    let small = QuestLogCatalogue::new(&state_catalogue(), vec![quest(RATS, RATS_START, vec![])])
        .expect("active");
    let old_state = QuestStateCatalogue::new(
        "content-0",
        vec![track(RATS_START, RATS), track(RATS_STAGE, RATS)],
        vec![],
    )
    .expect("old state");
    let large = QuestLogCatalogue::new(&old_state, vec![quest(RATS, RATS_START, missions(15))])
        .expect("pinned");
    assert!(QuestLogLines::new(QuestLogContent::new(small, vec![large])).is_none());
}

#[test]
fn the_session_copy_is_read_as_the_projection_facts() {
    use crate::domain::{CharacterId, CharacterRevision};
    use crate::durability::quest_state::quest::QuestTrackChange;
    use crate::durability::quest_state::{CommittedQuestTransition, QuestCause};
    use crate::foundation::{CommandId, CommandRef, GameSessionId};

    let mut id = [0x42; 16];
    id[6] = 0x72;
    id[8] = 0x82;
    let receipt = |completes: bool, after: i64| CommittedQuestTransition {
        character_id: CharacterId::from_bytes(id).expect("character"),
        cause: QuestCause::Command(CommandRef::new(
            GameSessionId::decode(&id).expect("session"),
            CommandId::new(1).expect("command"),
        )),
        transition_key: "oteryn:t/1".into(),
        quest_key: RATS.into(),
        original_character_revision: CharacterRevision::new(1).expect("revision"),
        committed_character_revision: CharacterRevision::new(2).expect("revision"),
        pinned_content_revision: "content-1".into(),
        definition_hash: [7; 32],
        completes,
        changes: vec![QuestTrackChange {
            track: RATS_START.into(),
            before: 0,
            after,
        }],
        experience: None,
    };
    let mut copy = QuestStateCopy::default();
    assert_eq!(copy.track_value(RATS_START), None);
    assert_eq!(copy.quest_state(RATS), None);
    assert!(copy.apply(&receipt(false, 1)));
    assert_eq!(copy.track_value(RATS_START), Some(1));
    assert_eq!(
        copy.quest_state(RATS),
        Some(CopyQuestState {
            pinned_content_revision: "content-1",
            completed: false
        })
    );
    // The projection reads it: the quest is listed and in progress against its pin.
    let lines = content();
    assert!(lines.0.is_listed(&copy, RATS_INDEX));
    assert!(copy.apply(&receipt(true, 2)));
    assert_eq!(
        copy.quest_state(RATS).map(|state| state.completed),
        Some(true)
    );
    assert!(
        lines.0.list(&copy)[0].completed,
        "a completed receipt marks the quest completed"
    );
}

#[test]
fn the_snapshot_has_no_view_and_the_carried_tracked_quests() {
    let mut continuity = QuestLogContinuity::FRESH;
    // No copy: an empty domain at revision 1.
    let (state, domain) =
        QuestLogState::snapshot(&mut continuity, QuestLogObservation::Unavailable)
            .expect("snapshot");
    assert_eq!(domain.to, 1);
    assert!(domain.payload.is_empty());
    assert_eq!(state.version(), None);
    // A later connection with a tracked quest: above every revision, with that quest's line.
    continuity.tracked = TrackedQuests::new(&[RATS_INDEX]).expect("tracked");
    let facts = Facts::with(&[(RATS_START, 1), (RATS_STAGE, 1)]);
    let (state, domain) =
        QuestLogState::snapshot(&mut continuity, changed(facts, 5)).expect("snapshot");
    assert_eq!(domain.to, 2);
    assert_eq!(continuity.revision, 2);
    assert_eq!(state.version(), Some(5));
    assert_eq!(
        shown(&domain),
        QuestLog {
            view: None,
            tracked: vec![rats_line("Hunt the rats.", false)],
        }
    );
}

#[test]
fn list_and_quest_queries_send_the_whole_domain() {
    let mut continuity = QuestLogContinuity::FRESH;
    let (mut state, _) = QuestLogState::snapshot(&mut continuity, QuestLogObservation::Unavailable)
        .expect("snapshot");
    let facts = Facts::with(&[(RATS_START, 1), (RATS_STAGE, 2)]);
    let listed = state
        .query(
            &mut continuity,
            &QuestLogQuery::List,
            changed(facts.clone(), 1),
        )
        .expect("accepted")
        .expect("delta");
    assert_eq!(listed.to, 2);
    assert_eq!(
        shown(&listed).view,
        Some(QuestLogView::List(vec![QuestListEntry {
            quest_index: RATS_INDEX,
            completed: false
        }]))
    );
    let line = state
        .query(
            &mut continuity,
            &QuestLogQuery::Quest {
                quest_index: RATS_INDEX,
            },
            changed(facts.clone(), 1),
        )
        .expect("accepted")
        .expect("delta");
    assert_eq!(line.to, 3);
    assert_eq!(
        shown(&line).view,
        Some(QuestLogView::Line(rats_line("Hunt the rats.", true)))
    );
    // An unlisted or unknown quest, and a copy that is not loaded, are refused: nothing changes.
    for (query, observation) in [
        (
            QuestLogQuery::Quest {
                quest_index: TOMB_INDEX,
            },
            changed(facts.clone(), 1),
        ),
        (
            QuestLogQuery::Quest { quest_index: 9 },
            changed(facts.clone(), 1),
        ),
        (QuestLogQuery::List, QuestLogObservation::Unavailable),
        (QuestLogQuery::List, QuestLogObservation::Unchanged),
    ] {
        assert!(state.query(&mut continuity, &query, observation).is_none());
        assert_eq!(continuity.revision, 3);
    }
}

#[test]
fn track_replaces_the_set_and_refuses_unlisted_or_eleven_quests() {
    let mut continuity = QuestLogContinuity::FRESH;
    let (mut state, _) = QuestLogState::snapshot(&mut continuity, QuestLogObservation::Unavailable)
        .expect("snapshot");
    let facts = Facts::with(&[(RATS_START, 1), (TOMB_START, 2)]);
    let track = |indexes: &[u32]| QuestLogQuery::Track {
        quest_indexes: indexes.to_vec(),
    };
    let both = state
        .query(
            &mut continuity,
            &track(&[TOMB_INDEX, RATS_INDEX]),
            changed(facts.clone(), 1),
        )
        .expect("accepted")
        .expect("delta");
    let tracked: Vec<u32> = shown(&both)
        .tracked
        .iter()
        .map(|line| line.quest_index)
        .collect();
    assert_eq!(tracked, [TOMB_INDEX, RATS_INDEX], "in the client's order");
    assert_eq!(continuity.tracked.as_slice(), [TOMB_INDEX, RATS_INDEX]);
    // The rats quest shows no mission yet (stage 0 is below 1); the tomb its first.
    assert_eq!(shown(&both).tracked[1].missions, []);
    // An unlisted quest refuses the whole set; so does an eleventh index.
    let unlisted = Facts::with(&[(TOMB_START, 2)]);
    assert!(
        state
            .query(
                &mut continuity,
                &track(&[TOMB_INDEX, RATS_INDEX]),
                changed(unlisted, 2)
            )
            .is_none()
    );
    let eleven: Vec<u32> = (0..11).collect();
    assert!(
        state
            .query(&mut continuity, &track(&eleven), changed(facts.clone(), 1))
            .is_none()
    );
    assert_eq!(continuity.tracked.as_slice(), [TOMB_INDEX, RATS_INDEX]);
    // An empty set tracks nothing.
    let none = state
        .query(&mut continuity, &track(&[]), changed(facts, 1))
        .expect("accepted")
        .expect("delta");
    assert!(shown(&none).tracked.is_empty());
    assert!(continuity.tracked.as_slice().is_empty());
}

#[test]
fn a_changed_copy_sends_a_delta_only_when_the_domain_changes() {
    let mut continuity = QuestLogContinuity::FRESH;
    continuity.tracked = TrackedQuests::new(&[RATS_INDEX]).expect("tracked");
    let at = |stage| Facts::with(&[(RATS_START, 1), (RATS_STAGE, stage)]);
    let (mut state, snapshot) =
        QuestLogState::snapshot(&mut continuity, changed(at(1), 1)).expect("snapshot");
    // Unchanged or unavailable: nothing.
    for observation in [
        QuestLogObservation::Unchanged,
        QuestLogObservation::Unavailable,
    ] {
        assert_eq!(state.refresh(&mut continuity, observation), Ok(None));
    }
    // A new version that shows the same: nothing, but the version is recorded.
    assert_eq!(state.refresh(&mut continuity, changed(at(1), 2)), Ok(None));
    assert_eq!(state.version(), Some(2));
    assert_eq!(continuity.revision, snapshot.to);
    // The mission completes: the tracker follows it.
    let delta = state
        .refresh(&mut continuity, changed(at(2), 3))
        .expect("refresh")
        .expect("delta");
    assert_eq!(delta.to, snapshot.to + 1);
    assert_eq!(shown(&delta).tracked, [rats_line("Hunt the rats.", true)]);
    // The quest leaves the list: the tracker no longer shows it.
    let gone = state
        .refresh(&mut continuity, changed(Facts::default(), 4))
        .expect("refresh")
        .expect("delta");
    assert_eq!(shown(&gone), QuestLog::default());
    assert_eq!(
        continuity.tracked.as_slice(),
        [RATS_INDEX],
        "the set is kept"
    );
}

mod connection {
    use super::super::super::super::capabilities::{OfferedCapability, SelectedCapabilities};
    use super::super::super::super::world_spatial::ActorPosition;
    use super::super::super::super::world_spatial::{
        SNAPSHOT_TYPE_WORLD_SPATIAL_V1, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, StepDirection,
        WorldSpatialObservation, encode_world_spatial,
    };
    use super::super::super::{
        AdmissionRefusal, AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAttempt,
        FreshAdmissionAuthority, IDLE_LIVENESS, SessionContinuity, StepOutcome, serve_admitted,
    };
    use super::*;
    use crate::foundation::{
        ChannelId, CommandStatus, DomainSnapshot, ExactActorRef, GameSessionId, WorldId,
        encode_command_result, encode_single_chunk_snapshot, encode_state_delta,
    };
    use oteryn_protocol_oteryn::quest_log::{
        CAPABILITY_QUEST_LOG_V1, COMMAND_TYPE_QUEST_LOG_QUERY, DELTA_TYPE_QUEST_LOG_V1,
        SNAPSHOT_TYPE_QUEST_LOG_V1, STATE_DOMAIN_QUEST_LOG, encode_quest_log_query,
    };
    use oteryn_protocol_oteryn::{ClientCommandValue, encode_client_command};
    use std::cell::RefCell;
    use std::error::Error;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

    type TestResult = Result<(), Box<dyn Error>>;

    const fn uuid_v7(tag: u8) -> [u8; 16] {
        let mut bytes = [tag; 16];
        bytes[6] = 0x70 | (tag & 0x0f);
        bytes[8] = 0x80 | (tag & 0x3f);
        bytes
    }

    const OFFERED: &[OfferedCapability] = &[OfferedCapability {
        id: CAPABILITY_QUEST_LOG_V1,
        requires: &[],
    }];

    /// The Channel owner as the quest log sees it: one copy at version 1, the copy a receipt
    /// commits after the first read (version 2), and the reads made.
    struct QuestAuthority {
        facts: RefCell<Option<Facts>>,
        version: RefCell<u64>,
        receipt: RefCell<Option<Facts>>,
        reads: RefCell<Vec<Option<u64>>>,
    }

    impl QuestAuthority {
        fn new(facts: Option<Facts>) -> Self {
            Self {
                facts: RefCell::new(facts),
                version: RefCell::new(1),
                receipt: RefCell::new(None),
                reads: RefCell::new(Vec::new()),
            }
        }
    }

    fn observation() -> WorldSpatialObservation {
        WorldSpatialObservation {
            content_generation: [0x5c; 32],
            actor_position: ActorPosition {
                x: 10,
                y: 10,
                floor: 7,
            },
        }
    }

    impl FreshAdmissionAuthority for QuestAuthority {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }

        async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(observation())
        }

        async fn step(&self, _actor: ExactActorRef, _direction: StepDirection) -> StepOutcome {
            StepOutcome::rejected()
        }

        async fn observe_quest_log(
            &self,
            _actor: ExactActorRef,
            _session: GameSessionId,
            since: Option<u64>,
        ) -> QuestLogObservation {
            self.reads.borrow_mut().push(since);
            let version = *self.version.borrow();
            let observation = match self.facts.borrow().clone() {
                None => QuestLogObservation::Unavailable,
                Some(_) if since == Some(version) => QuestLogObservation::Unchanged,
                Some(facts) => changed(facts, version),
            };
            if let Some(next) = self.receipt.borrow_mut().take() {
                *self.facts.borrow_mut() = Some(next);
                *self.version.borrow_mut() += 1;
            }
            observation
        }
    }

    fn session(continuity: SessionContinuity) -> Result<AdmittedSession, Box<dyn Error>> {
        let world_id = WorldId::decode(&uuid_v7(0x33))?;
        let channel_id = ChannelId::decode(&uuid_v7(0x44))?;
        Ok(AdmittedSession {
            game_session_id: GameSessionId::decode(&uuid_v7(0x22))?,
            world_id,
            channel_id,
            runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity,
            item_fence: None,
        })
    }

    fn selected(supported: &[u32]) -> SessionContinuity {
        SessionContinuity {
            selected_capabilities: SelectedCapabilities::select(OFFERED, supported)
                .expect("select"),
            ..SessionContinuity::FRESH
        }
    }

    fn query(id: u64, query: &QuestLogQuery) -> Result<Vec<u8>, Box<dyn Error>> {
        Ok(encode_client_command(
            1,
            &ClientCommandValue {
                command_id: id,
                command_type: COMMAND_TYPE_QUEST_LOG_QUERY,
                payload: &encode_quest_log_query(query).map_err(|error| format!("{error:?}"))?,
            },
        )?)
    }

    fn result(sequence: u64, id: u64, status: CommandStatus) -> Vec<u8> {
        encode_command_result(1, sequence, id, status, &[]).expect("result")
    }

    fn delta(sequence: u64, to: u64, log: &QuestLog) -> Vec<u8> {
        encode_state_delta(
            1,
            sequence,
            STATE_DOMAIN_QUEST_LOG,
            to - 1,
            to,
            DELTA_TYPE_QUEST_LOG_V1,
            &oteryn_protocol_oteryn::quest_log::encode_quest_log(log).expect("log"),
        )
        .expect("delta")
    }

    /// The join snapshot: domain 1, then domain 16 when given.
    fn snapshot(quest_log: Option<(u64, &QuestLog)>) -> Vec<Vec<u8>> {
        let spatial = encode_world_spatial(&observation());
        let payload = quest_log
            .map(|(_, log)| oteryn_protocol_oteryn::quest_log::encode_quest_log(log).expect("log"));
        let mut domains = vec![DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            revision: 1,
            snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
            payload: &spatial,
        }];
        if let (Some((revision, _)), Some(payload)) = (quest_log, &payload) {
            domains.push(DomainSnapshot {
                domain_id: STATE_DOMAIN_QUEST_LOG,
                revision,
                snapshot_type: SNAPSHOT_TYPE_QUEST_LOG_V1,
                payload,
            });
        }
        encode_single_chunk_snapshot(1, 1, 0, &domains)
            .expect("snapshot")
            .into()
    }

    async fn serve(
        authority: &QuestAuthority,
        admitted: AdmittedSession,
        frames: &[Vec<u8>],
    ) -> Result<(ConnectionEnd, Vec<Vec<u8>>), Box<dyn Error>> {
        let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 20);
        for frame in frames {
            let mut framed = (frame.len() as u32).to_be_bytes().to_vec();
            framed.extend_from_slice(frame);
            client.write_all(&framed).await?;
        }
        client.shutdown().await?;
        let end = serve_admitted(&mut server, admitted, authority, IDLE_LIVENESS).await;
        drop(server);
        let mut output = Vec::new();
        client.read_to_end(&mut output).await?;
        let mut frames = Vec::new();
        let mut cursor = 0;
        while cursor < output.len() {
            let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
            frames.push(output[cursor + 4..cursor + 4 + length].to_vec());
            cursor += 4 + length;
        }
        Ok((end, frames))
    }

    fn run(test: impl std::future::Future<Output = TestResult>) -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()?
            .block_on(test)
    }

    fn ended(end: ConnectionEnd) -> Result<SessionContinuity, Box<dyn Error>> {
        match end {
            ConnectionEnd::AdmittedThenDisconnected(admitted) => Ok(admitted.continuity),
            other => Err(format!("unexpected end {other:?}").into()),
        }
    }

    fn listed() -> Facts {
        Facts::with(&[(RATS_START, 1), (RATS_STAGE, 1)])
    }

    fn list_view() -> QuestLog {
        QuestLog {
            view: Some(QuestLogView::List(vec![QuestListEntry {
                quest_index: RATS_INDEX,
                completed: false,
            }])),
            tracked: Vec::new(),
        }
    }

    #[test]
    fn domain_16_and_command_22_need_capability_16() -> TestResult {
        run(async {
            // Without capability 16: no domain 16, and command 22 is refused before any read.
            let authority = QuestAuthority::new(Some(listed()));
            let (end, frames) = serve(
                &authority,
                session(selected(&[]))?,
                &[query(1, &QuestLogQuery::List)?],
            )
            .await?;
            let mut expected = snapshot(None);
            expected.push(result(1, 1, CommandStatus::Rejected));
            assert_eq!(frames, expected);
            assert!(authority.reads.borrow().is_empty());
            assert_eq!(ended(end)?.quest_log, QuestLogContinuity::FRESH);

            // With it: the snapshot, then the list.
            let authority = QuestAuthority::new(Some(listed()));
            let (end, frames) = serve(
                &authority,
                session(selected(&[CAPABILITY_QUEST_LOG_V1]))?,
                &[query(1, &QuestLogQuery::List)?],
            )
            .await?;
            let mut expected = snapshot(Some((1, &QuestLog::default())));
            expected.push(result(1, 1, CommandStatus::Accepted));
            expected.push(delta(2, 2, &list_view()));
            assert_eq!(frames, expected);
            assert_eq!(*authority.reads.borrow(), [None, None]);
            assert_eq!(ended(end)?.quest_log.revision, 2);
            Ok(())
        })
    }

    #[test]
    fn refused_queries_are_rejected_with_an_empty_payload_and_change_nothing() -> TestResult {
        run(async {
            // A copy that is not loaded: an empty snapshot, and every query is REJECTED.
            let authority = QuestAuthority::new(None);
            let (end, frames) = serve(
                &authority,
                session(selected(&[CAPABILITY_QUEST_LOG_V1]))?,
                &[query(1, &QuestLogQuery::List)?],
            )
            .await?;
            let mut expected = snapshot(Some((1, &QuestLog::default())));
            expected.push(result(1, 1, CommandStatus::Rejected));
            assert_eq!(frames, expected);
            assert_eq!(ended(end)?.quest_log.revision, 1);

            // A malformed query (an empty oneof) and an unlisted quest.
            let authority = QuestAuthority::new(Some(listed()));
            let empty = encode_client_command(
                1,
                &ClientCommandValue {
                    command_id: 1,
                    command_type: COMMAND_TYPE_QUEST_LOG_QUERY,
                    payload: &[],
                },
            )?;
            let (end, frames) = serve(
                &authority,
                session(selected(&[CAPABILITY_QUEST_LOG_V1]))?,
                &[
                    empty,
                    query(
                        2,
                        &QuestLogQuery::Quest {
                            quest_index: TOMB_INDEX,
                        },
                    )?,
                ],
            )
            .await?;
            let mut expected = snapshot(Some((1, &QuestLog::default())));
            expected.push(result(1, 1, CommandStatus::Rejected));
            expected.push(result(2, 2, CommandStatus::Rejected));
            assert_eq!(frames, expected);
            let continuity = ended(end)?;
            assert_eq!(continuity.quest_log.revision, 1);
            assert!(continuity.quest_log.tracked.as_slice().is_empty());
            Ok(())
        })
    }

    #[test]
    fn a_third_query_in_one_second_is_rejected_also_after_a_reconnect() -> TestResult {
        run(async {
            let authority = QuestAuthority::new(Some(listed()));
            let track = QuestLogQuery::Track {
                quest_indexes: vec![RATS_INDEX],
            };
            let (end, frames) = serve(
                &authority,
                session(selected(&[CAPABILITY_QUEST_LOG_V1]))?,
                &[
                    query(1, &QuestLogQuery::List)?,
                    query(2, &track)?,
                    query(3, &QuestLogQuery::List)?,
                ],
            )
            .await?;
            let tracked = QuestLog {
                view: list_view().view,
                tracked: vec![rats_line("Hunt the rats.", false)],
            };
            let mut expected = snapshot(Some((1, &QuestLog::default())));
            expected.push(result(1, 1, CommandStatus::Accepted));
            expected.push(delta(2, 2, &list_view()));
            expected.push(result(3, 2, CommandStatus::Accepted));
            expected.push(delta(4, 3, &tracked));
            expected.push(result(5, 3, CommandStatus::Rejected));
            assert_eq!(frames, expected);
            // The over-rate query never read the copy.
            assert_eq!(authority.reads.borrow().len(), 3);
            let lost = ended(end)?;

            // The reconnect carries the window, the revision and the tracked set; the view
            // closed, so the snapshot shows only the tracker.
            let resumed = SessionContinuity {
                connection_generation: 1,
                next_command_id: 1,
                server_sequence: 0,
                ..lost
            };
            let (end, frames) = serve(
                &authority,
                session(resumed)?,
                &[query(1, &QuestLogQuery::List)?],
            )
            .await?;
            let reconnect = QuestLog {
                view: None,
                tracked: tracked.tracked.clone(),
            };
            let mut expected = snapshot(Some((4, &reconnect)));
            expected.push(result(1, 1, CommandStatus::Rejected));
            assert_eq!(frames, expected);
            let continuity = ended(end)?;
            assert_eq!(continuity.quest_log.revision, 4);
            assert_eq!(continuity.quest_log.tracked.as_slice(), [RATS_INDEX]);
            Ok(())
        })
    }

    #[test]
    fn a_committed_receipt_that_changes_the_tracker_sends_a_delta() -> TestResult {
        run(async {
            let authority = QuestAuthority::new(Some(listed()));
            let continuity = SessionContinuity {
                quest_log: QuestLogContinuity {
                    tracked: TrackedQuests::new(&[RATS_INDEX]).ok_or("tracked")?,
                    ..QuestLogContinuity::FRESH
                },
                ..selected(&[CAPABILITY_QUEST_LOG_V1])
            };
            let before = QuestLog {
                view: None,
                tracked: vec![rats_line("Hunt the rats.", false)],
            };
            let after = QuestLog {
                view: None,
                tracked: vec![rats_line("Hunt the rats.", true)],
            };
            let mut expected = snapshot(Some((1, &before)));
            expected.push(delta(1, 2, &after));
            let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 16);
            let wanted = expected.len();
            let read = async move {
                let mut frames = Vec::new();
                while frames.len() < wanted {
                    let mut length = [0_u8; 4];
                    client.read_exact(&mut length).await?;
                    let mut frame = vec![0_u8; u32::from_be_bytes(length) as usize];
                    client.read_exact(&mut frame).await?;
                    frames.push(frame);
                }
                drop(client);
                Ok::<_, std::io::Error>(frames)
            };
            // The snapshot reads version 1 at stage 1; a receipt then commits version 2, which
            // completes the mission before the first refresh.
            *authority.receipt.borrow_mut() =
                Some(Facts::with(&[(RATS_START, 1), (RATS_STAGE, 2)]));
            let reader = tokio::spawn(read);
            let end =
                serve_admitted(&mut server, session(continuity)?, &authority, IDLE_LIVENESS).await;
            assert_eq!(reader.await??, expected);
            assert_eq!(ended(end)?.quest_log.revision, 2);
            // The refresh named the version it had projected.
            assert_eq!(authority.reads.borrow()[..2], [None, Some(1)]);
            Ok(())
        })
    }
}
