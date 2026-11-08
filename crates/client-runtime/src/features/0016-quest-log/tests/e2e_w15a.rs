//! W15a client replay: the real quest journal payloads (join snapshot, `List`, `Quest` line and
//! `Track` deltas) enter the native quest reducer through `QuestState::from_snapshot` and
//! `QuestState::apply`; the refused query for an unlisted quest leaves the state unchanged.
use super::*;

type BoxError = Box<dyn std::error::Error>;

struct Capture {
    snapshot: Vec<u8>,
    list: Vec<u8>,
    line: Vec<u8>,
    track: Vec<u8>,
    expected: String,
}

fn unescape(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
}

fn fixture_line(index: u32, done: bool) -> QuestLine {
    QuestLine {
        quest_index: index,
        completed: false,
        missions: vec![
            QuestMission {
                mission_index: 0,
                name: "Dormovo's Amulet".into(),
                text: "Bring the amulet.\nSpeak with Morris.".into(),
                done,
            },
            QuestMission {
                mission_index: 1,
                name: "Dawnfire".into(),
                text: "Gather herbs.".into(),
                done: false,
            },
        ],
    }
}

fn line_text(prefix: &str, line: &QuestLine) -> String {
    let mut text = format!(
        "{prefix}={}\t{}\t{}\n",
        line.quest_index,
        u8::from(line.completed),
        line.missions.len()
    );
    for mission in &line.missions {
        text.push_str(&format!(
            "{prefix}.mission={}\t{}\t{}\t{}\n",
            mission.mission_index,
            u8::from(mission.done),
            escape(&mission.name),
            escape(&mission.text)
        ));
    }
    text
}

fn capture() -> Result<Capture, BoxError> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok(Capture {
            snapshot: std::fs::read(root.join("w15a-snapshot.bin"))?,
            list: std::fs::read(root.join("w15a-list.bin"))?,
            line: std::fs::read(root.join("w15a-line.bin"))?,
            track: std::fs::read(root.join("w15a-track.bin"))?,
            expected: std::fs::read_to_string(root.join("w15a-expected.txt"))?,
        });
    }
    let encode = |log: &QuestLog| encode_quest_log(log).map_err(|error| format!("{error:?}"));
    let line = fixture_line(7, true);
    let list = QuestLog {
        view: Some(QuestLogView::List(vec![
            QuestListEntry {
                quest_index: 7,
                completed: false,
            },
            QuestListEntry {
                quest_index: 12,
                completed: true,
            },
        ])),
        tracked: vec![],
    };
    let mut expected = String::from(
        "snapshot_revision=4\nlist_base=4\nlist_new=5\nlist_sequence=19\nline_base=5\nline_new=6\nline_sequence=21\ntrack_base=6\ntrack_new=7\ntrack_sequence=23\n\
         list=7\t0\nlist=12\t1\n",
    );
    expected.push_str(&line_text("line", &line));
    expected.push_str(&line_text("tracked", &line));
    expected.push_str("invalid=13\nrefused=Rejected\n");
    Ok(Capture {
        snapshot: encode(&QuestLog::default())?,
        list: encode(&list)?,
        line: encode(&QuestLog {
            view: Some(QuestLogView::Line(line.clone())),
            tracked: vec![],
        })?,
        track: encode(&QuestLog {
            view: None,
            tracked: vec![line],
        })?,
        expected,
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, BoxError> {
    expected
        .lines()
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.strip_prefix('='))
        })
        .ok_or_else(|| format!("w15a expected lacks {key}").into())
}

fn number(expected: &str, key: &str) -> Result<u64, BoxError> {
    Ok(value(expected, key)?.parse()?)
}

fn expected_line(expected: &str, prefix: &str) -> Result<QuestLine, BoxError> {
    let head: Vec<&str> = value(expected, prefix)?.split('\t').collect();
    let [index, completed, count] = head.as_slice() else {
        return Err(format!("w15a {prefix} head malformed").into());
    };
    let key = format!("{prefix}.mission=");
    let missions = expected
        .lines()
        .filter_map(|line| line.strip_prefix(key.as_str()))
        .map(|rest| {
            let fields: Vec<&str> = rest.split('\t').collect();
            let [mission, done, name, text] = fields.as_slice() else {
                return Err(BoxError::from("w15a mission row malformed"));
            };
            Ok(QuestMission {
                mission_index: mission.parse()?,
                name: unescape(name),
                text: unescape(text),
                done: *done == "1",
            })
        })
        .collect::<Result<Vec<_>, BoxError>>()?;
    assert_eq!(missions.len(), count.parse::<usize>()?);
    Ok(QuestLine {
        quest_index: index.parse()?,
        completed: *completed == "1",
        missions,
    })
}

fn delta(expected: &str, stage: &str, payload: &[u8]) -> Result<AppliedDelta<QuestLog>, BoxError> {
    Ok(AppliedDelta {
        server_sequence: number(expected, &format!("{stage}_sequence"))?,
        base_revision: number(expected, &format!("{stage}_base"))?,
        new_revision: number(expected, &format!("{stage}_new"))?,
        value: decode_quest_log(payload).map_err(|error| format!("{stage}: {error:?}"))?,
    })
}

#[test]
fn e2e_w15a_quest_journal_payloads_replay_into_the_quest_reducer() -> Result<(), BoxError> {
    let capture = capture()?;
    let expected = capture.expected.as_str();
    let snapshot = GatedSnapshot {
        domain_id: STATE_DOMAIN_QUEST_LOG,
        snapshot_type: SNAPSHOT_TYPE_QUEST_LOG_V1,
        revision: number(expected, "snapshot_revision")?,
        payload: capture.snapshot.clone(),
    };
    let mut state = QuestState::from_snapshot(&[CAPABILITY_QUEST_LOG_V1], Some(&snapshot))
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(state.revision, Some(number(expected, "snapshot_revision")?));

    let listed = delta(expected, "list", &capture.list)?;
    let list_new = listed.new_revision;
    state.apply(&listed).map_err(|error| format!("{error:?}"))?;
    let want_list = expected
        .lines()
        .filter_map(|line| line.strip_prefix("list="))
        .map(|rest| {
            let (index, completed) = rest.split_once('\t').ok_or("w15a list row malformed")?;
            Ok(QuestListEntry {
                quest_index: index.parse()?,
                completed: completed == "1",
            })
        })
        .collect::<Result<Vec<_>, BoxError>>()?;
    assert_eq!(state.list, want_list);
    assert_eq!(state.revision, Some(list_new));

    // The native request builder only lets a listed quest through, so the Quest query that
    // produced the captured line is exactly the first list entry.
    let want_line = expected_line(expected, "line")?;
    assert_eq!(want_line.quest_index, want_list[0].quest_index);
    assert!(
        state
            .command(&QuestLogQuery::Quest {
                quest_index: want_line.quest_index
            })
            .is_ok()
    );
    state
        .apply(&delta(expected, "line", &capture.line)?)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(state.log.view, Some(QuestLogView::Line(want_line.clone())));
    let entry = state
        .list
        .iter()
        .find(|entry| entry.quest_index == want_line.quest_index)
        .ok_or("listed quest vanished")?;
    assert_eq!(entry.completed, want_line.completed);

    let want_tracked = expected_line(expected, "tracked")?;
    state
        .apply(&delta(expected, "track", &capture.track)?)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(state.log.tracked, vec![want_tracked]);
    assert_eq!(state.revision, Some(number(expected, "track_new")?));

    // Refusal: the unlisted quest cannot even be requested natively, the server's Rejected reply
    // is recorded, and neither changes the projection or revision.
    let before = state.clone();
    let invalid = u32::try_from(number(expected, "invalid")?)?;
    assert_eq!(
        state.command(&QuestLogQuery::Quest {
            quest_index: invalid
        }),
        Err(QuestRefusal::UnknownQuest)
    );
    assert_eq!(value(expected, "refused")?, "Rejected");
    state
        .record_result(CommandStatus::Rejected, &[])
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(state.refusal, Some(QuestRefusal::Rejected));
    assert_eq!(
        (&state.list, &state.log, state.revision),
        (&before.list, &before.log, before.revision)
    );
    // A stale delta (old base revision) is refused and leaves the state unchanged.
    let stale = delta(expected, "list", &capture.list)?;
    assert_eq!(state.apply(&stale), Err(QuestRefusal::RevisionMismatch));
    assert_eq!(
        (&state.list, &state.log, state.revision),
        (&before.list, &before.log, before.revision)
    );
    Ok(())
}
