use super::{Panel, panel_text, protocol::*};
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
fn selector_keeps_confirmed_totals_and_shows_only_the_selected_tool() {
    use super::Tool;
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    apply(
        &mut panel,
        &[
            AnalyserFact::Kill { race: None },
            AnalyserFact::Experience {
                raw: 25,
                gained: 30,
            },
        ],
    );
    panel.tool = Tool::Experience;
    let text = panel_text(&panel);
    assert!(text.contains("Experience: 25 raw / 30 gained"));
    assert!(!text.contains("Kills:"));
    panel.tool = Tool::Hunting;
    assert!(panel_text(&panel).contains("Kills: 1"));
    assert!(!panel_text(&panel).contains("Experience:"));
    reset(&mut panel);
    assert_eq!(panel.tool, Tool::Hunting);
    assert!(panel_text(&panel).contains("Kills: 0"));
    for tool in Tool::ALL {
        panel.tool = tool;
        assert!(panel_text(&panel).contains(tool.title()));
    }
}
