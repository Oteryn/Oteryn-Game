//! W15d client replay: the quest journal lists captured before and after the guidance shrine use
//! enter the native quest reducer (`QuestState::from_snapshot` / `QuestState::apply`). The
//! shrine callback is one-way, so a refused use leaves the projection and revision unchanged and
//! a committed use changes the list; a stale or malformed list delta is refused without change.
use super::*;

type BoxError = Box<dyn std::error::Error>;

struct Capture {
    snapshot: Vec<u8>,
    before: Vec<u8>,
    after: Vec<u8>,
    expected: String,
}

fn list_log(entries: &[(u32, bool)]) -> QuestLog {
    QuestLog {
        map_marks: Vec::new(),
        view: Some(QuestLogView::List(
            entries
                .iter()
                .map(|(quest_index, completed)| QuestListEntry {
                    quest_index: *quest_index,
                    completed: *completed,
                })
                .collect(),
        )),
        tracked: vec![],
    }
}

fn capture() -> Result<Capture, BoxError> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok(Capture {
            snapshot: std::fs::read(root.join("w15d-snapshot.bin"))?,
            before: std::fs::read(root.join("w15d-before.bin"))?,
            after: std::fs::read(root.join("w15d-after.bin"))?,
            expected: std::fs::read_to_string(root.join("w15d-expected.txt"))?,
        });
    }
    let encode = |log: &QuestLog| encode_quest_log(log).map_err(|error| format!("{error:?}"));
    Ok(Capture {
        snapshot: encode(&QuestLog::default())?,
        before: encode(&list_log(&[(3, false)]))?,
        after: encode(&list_log(&[(3, false), (14, false)]))?,
        expected: "snapshot_revision=2\nuse=Committed\nlist_before=3:0\nbefore_base=2\nbefore_new=3\n\
                   before_sequence=41\nlist_after=3:0;14:0\nafter_base=3\nafter_new=4\nafter_sequence=43\n"
            .into(),
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, BoxError> {
    expected
        .lines()
        .find_map(|line| {
            line.strip_prefix(key)
                .and_then(|rest| rest.strip_prefix('='))
        })
        .ok_or_else(|| format!("w15d expected lacks {key}").into())
}

fn number(expected: &str, key: &str) -> Result<u64, BoxError> {
    Ok(value(expected, key)?.parse()?)
}

fn visible(state: &QuestState) -> String {
    state
        .list
        .iter()
        .map(|entry| format!("{}:{}", entry.quest_index, u8::from(entry.completed)))
        .collect::<Vec<_>>()
        .join(";")
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
fn e2e_w15d_shrine_use_quest_list_replays_into_the_quest_reducer() -> Result<(), BoxError> {
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
    state
        .apply(&delta(expected, "before", &capture.before)?)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(visible(&state), value(expected, "list_before")?);
    let before = state.clone();

    state
        .apply(&delta(expected, "after", &capture.after)?)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(visible(&state), value(expected, "list_after")?);
    assert_eq!(state.revision, Some(number(expected, "after_new")?));
    match value(expected, "use")? {
        "Committed" => assert_ne!(visible(&state), visible(&before)),
        "Rejected" => assert_eq!(state.list, before.list),
        other => return Err(format!("w15d unknown use outcome {other}").into()),
    }

    // A replayed delta (stale base revision) and a refused command result leave the list alone.
    let kept = state.clone();
    assert_eq!(
        state.apply(&delta(expected, "after", &capture.after)?),
        Err(QuestRefusal::RevisionMismatch)
    );
    state
        .record_result(CommandStatus::Rejected, &[])
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(state.refusal, Some(QuestRefusal::Rejected));
    assert_eq!(
        (&state.list, &state.log, state.revision),
        (&kept.list, &kept.log, kept.revision)
    );
    Ok(())
}
