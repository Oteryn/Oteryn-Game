use super::{Panel, Tool, panel_text, protocol::*, select_tool};
#[test]
fn confirmed_state_is_visible() -> Result<(), &'static str> {
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    apply(
        &mut panel,
        &[
            AnalyserFact::Kill { race: None },
            AnalyserFact::KillLoot {
                race: None,
                corpse_items: vec![],
                gold: 7,
            },
            AnalyserFact::KillLoot {
                race: None,
                corpse_items: vec![],
                gold: 3,
            },
            AnalyserFact::Experience { raw: 8, gained: 0 },
            AnalyserFact::Dropped(std::num::NonZeroU32::new(2).ok_or("count")?),
        ],
    );
    assert_eq!(
        (
            panel.kills,
            panel.gold,
            panel.raw_xp,
            panel.gained_xp,
            panel.dropped
        ),
        (1, 10, 8, 0, 2)
    );
    assert!(panel_text(&panel).contains("Kills: 1"));
    reset(&mut panel);
    assert!(panel.selected);
    assert_eq!(panel.gold, 0);
    panel.selected = false;
    apply(&mut panel, &[AnalyserFact::Kill { race: None }]);
    assert_eq!(panel.kills, 0);
    Ok(())
}

#[test]
fn analytics_selector_opens_independent_tools_over_confirmed_totals() {
    let mut panel = Panel {
        selected: true,
        kills: 2,
        gained_xp: 30,
        ..Default::default()
    };
    assert!(select_tool(&mut panel, "xp"));
    assert!(panel_text(&panel).starts_with("XP Analyser"));
    assert!(!panel_text(&panel).contains("Kills:"));
    assert!(select_tool(&mut panel, "kills"));
    assert!(panel_text(&panel).contains("Kills: 2"));
    assert_eq!(panel.gained_xp, 30);
    assert!(!select_tool(&mut panel, "unknown"));
    assert_eq!(panel.tool, Tool::Kills);
}
