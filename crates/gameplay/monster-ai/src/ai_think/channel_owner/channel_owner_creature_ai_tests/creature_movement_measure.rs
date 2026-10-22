//! Packet §1.7: on-demand evidence, never a CI timing gate.
use super::*;

#[test]
#[ignore = "RL-17 host measurement; run explicitly with --ignored --nocapture"]
fn creature_movement_rl17_integrated_window_p99() {
    use crate::movement::speed::{EngineeringGroundSpeed, creature_speed_fixture::Native};
    let mut channel = Channel::with_capacity(1025);
    for _ in 0..1024 {
        let actor = channel.admit(at(64, 64), 0);
        channel.table.entry_mut(actor).expect("entry").state.spawn = Some(at(114, 114));
    }
    let blocking: BTreeSet<_> = (80..=90)
        .map(|y| crate::content::LogicalCell { x: 90, y, z: 7 })
        .chain([crate::content::LogicalCell {
            x: 114,
            y: 114,
            z: 7,
        }])
        .collect();
    let players = [];
    let sources: Vec<_> = channel
        .runtime
        .visible_entities()
        .creatures
        .iter()
        .map(|creature| Native {
            actor: creature.actor,
            base: 67,
            pin: channel.runtime.content_pin().server_artifact_digest(),
        })
        .collect();
    let navigation = channel.runtime.content_pin().map_revision_digest();
    let mut elapsed = Vec::new();
    for window in 0..200_u64 {
        let now = window * 1_000_000;
        // Failed walk backs idle until wake; this is the next accepted wake input, outside
        // the measured window, followed by 1,024 due thinks and the shared path drain.
        channel
            .table
            .wake_indexed(&[at(64, 64)], |_| Some(at(64, 64)), now);
        let start = std::time::Instant::now();
        let reports = channel.table.run_due_thinks(
            &mut channel.runtime,
            now,
            &channel.root,
            &channel.revisions,
            &players,
        );
        assert_eq!(reports.len(), 1024);
        channel.table.drive_movement(
            &mut channel.runtime,
            &channel.cells,
            &blocking,
            navigation,
            &sources,
            &EngineeringGroundSpeed,
            &channel.root,
            now,
            &mut channel.path_budget,
            false,
        );
        elapsed.push(start.elapsed());
        assert_eq!(
            channel.path_budget.charged_work(),
            65_536,
            "64 full 1,024-expansion searches"
        );
    }
    elapsed.sort_unstable();
    eprintln!(
        "RL-17 integrated 1,024 thinks + 64 full searches; 200 windows; p99={:?}",
        elapsed[197]
    );
}
