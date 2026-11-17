use oteryn_client_runtime::{
    cyclopedia_tracker::{TrackerEntry, TrackerSnapshot, encode_snapshot},
    raw_feature::{CommandStatus, GatedCommandResult},
};
use oteryn_client_ui::cyclopedia_tracker::{CyclopediaTrackerPanel, Intent, apply_result, request};
#[test]
fn controls_encode_exact_request_and_reply_and_refusal_are_visible() {
    let mut p = CyclopediaTrackerPanel {
        selected: true,
        creature_key: "oteryn:dragon".into(),
        ..Default::default()
    };
    let c = request(&mut p, Intent::Add).unwrap_or_else(|| unreachable!());
    assert_eq!(c.payload, b"\x01oteryn:dragon");
    let payload = encode_snapshot(&TrackerSnapshot {
        boss: false,
        entries: vec![TrackerEntry {
            creature_key: "oteryn:dragon".into(),
            kill_count: 4,
            thresholds: [5, 20, 60],
            completed: false,
        }],
    })
    .unwrap_or_default();
    let ok = GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    };
    assert!(apply_result(&mut p, &ok));
    assert_eq!(p.snapshot.as_ref().map(|v| v.entries.len()), Some(1));
    let refused = GatedCommandResult {
        status: CommandStatus::Rejected,
        ..ok
    };
    assert!(!apply_result(&mut p, &refused));
    assert!(p.feedback.is_some());
}

#[test]
fn tracker_widgets_round_trip_admitted_commands_and_keep_independent_state()
-> Result<(), &'static str> {
    use crate::combat_input::PlayCommand;
    use crate::play::{mailbox, play_channel};
    use oteryn_client_ui::cyclopedia_tracker::{
        TrackerSort, bestiary_rows, tracker_rows, tracker_selection,
    };
    let (link, mut commands, outbox) = play_channel();
    outbox.cyclopedia_tracker_initialize(true);
    {
        let mut m = mailbox::lock(&outbox.0);
        m.cyclopedia_tracker.sort = TrackerSort::Remaining;
        m.cyclopedia_tracker.hide_completed = true;
    }
    let entry = |key: &str, kills, completed| TrackerEntry {
        creature_key: key.into(),
        kill_count: kills,
        thresholds: [5, 10, 20],
        completed,
    };
    for (boss, kind, entries) in [
        (
            false,
            383,
            vec![
                entry("oteryn:rat", 9, false),
                entry("oteryn:bear", 18, false),
            ],
        ),
        (
            true,
            384,
            vec![
                entry("oteryn:boss_a", 20, true),
                entry("oteryn:boss_b", 4, false),
            ],
        ),
    ] {
        mailbox::lock(&outbox.0).cyclopedia_tracker.boss = boss;
        link.cyclopedia_tracker_request(Intent::Read);
        let Ok(PlayCommand::Feature(command)) = commands.try_recv() else {
            return Err("tracker command");
        };
        assert_eq!((command.capability, command.kind), (197, kind));
        assert!(command.payload.is_empty());
        outbox.cyclopedia_tracker_result(&GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: encode_snapshot(&TrackerSnapshot { boss, entries }).ok_or("snapshot")?,
        });
        assert!(!link.cyclopedia_tracker_panel().pending);
    }
    {
        let mut m = mailbox::lock(&outbox.0);
        m.cyclopedia_tracker.boss_sort = TrackerSort::Kills;
        m.cyclopedia_tracker.boss_search = "boss".into();
    }
    let p = link.cyclopedia_tracker_panel();
    assert_eq!(bestiary_rows(&p)[0].creature_key, "oteryn:rat");
    assert_eq!(tracker_rows(&p, true)[0].creature_key, "oteryn:boss_a");
    assert_eq!(tracker_selection(&p, true, "oteryn:boss_b"), Some(1));
    outbox.cyclopedia_tracker_push(
        encode_snapshot(&TrackerSnapshot {
            boss: false,
            entries: vec![],
        })
        .ok_or("push")?,
    );
    assert!(bestiary_rows(&link.cyclopedia_tracker_panel()).is_empty());
    assert_eq!(
        tracker_rows(&link.cyclopedia_tracker_panel(), true).len(),
        2
    );
    Ok(())
}
