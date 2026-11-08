use super::{
    Panel, Tool, panel_text,
    protocol::{AnalyserFact, apply_at, recover, reset},
};
#[test]
fn kill_goals_count_only_credited_kills_and_preserve_confirmed_task_progress()
-> Result<(), &'static str> {
    use crate::task_board::{TaskBoardPanel, task_board};
    let mut panel = Panel {
        selected: true,
        tool: Tool::Kills,
        ..Default::default()
    };
    let kill = AnalyserFact::Kill {
        race: std::num::NonZeroU32::new(7),
    };
    apply_at(
        &mut panel,
        &[
            kill.clone(),
            kill,
            AnalyserFact::KillLoot {
                race: std::num::NonZeroU32::new(7),
                corpse_items: vec![],
                gold: 1,
            },
        ],
        0,
    );
    assert!(super::kills::goal(&mut panel, "7", Some("3")));
    assert!(!super::kills::goal(&mut panel, "0", Some("3")));
    assert!(!super::kills::goal(&mut panel, "999", Some("3")));
    assert!(super::kills::goal(&mut panel, "7", Some("3")));
    assert!(panel_text(&panel).contains("Race #7: 2 / 3 · 1 remaining"));
    let mut board = TaskBoardPanel {
        selected: true,
        state: Some(task_board::TaskBoardReply {
            refused: false,
            selected: 1,
            kills: 4,
            points: 0,
            bounty_points: 0,
            talisman_levels: [0; 4],
            talisman_bonuses: [0; 4],
            talisman_next_costs: [0; 4],
            tasks: vec![task_board::TaskBoardTask {
                race: "oteryn:creature.tracker_fox".into(),
                required: 20,
                claimed_at_unix: 0,
            }],
        }),
        ..Default::default()
    };
    panel.task_progress = super::kills::task_rows(&board);
    apply_at(
        &mut panel,
        &[AnalyserFact::Kill {
            race: std::num::NonZeroU32::new(7),
        }],
        1,
    );
    let text = panel_text(&panel);
    assert!(text.contains("target reached"));
    assert!(text.contains("Hunting task · tracker fox: 4/20 · active"));
    if let Some(state) = &mut board.state {
        state.refused = true;
    }
    assert!(super::kills::task_rows(&board).is_none());
    reset(&mut panel);
    assert_eq!(panel.kill_goals.get(&7), Some(&3));
    assert!(panel_text(&panel).contains("Race #7: 0 / 3 · 3 remaining"));
    assert!(panel_text(&panel).contains("tracker fox: 4/20"));
    recover(&mut panel);
    assert!(panel.kill_goals.is_empty() && panel.task_progress.is_none());
    Ok(())
}
