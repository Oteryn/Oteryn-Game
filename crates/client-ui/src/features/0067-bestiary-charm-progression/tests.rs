use super::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
#[test]
fn pc01_charm_progression_shortcut_and_request_gate() -> Result<(), Box<dyn std::error::Error>> {
    let code = KeyCode::new(6)?;
    let modifiers = Modifiers::one(Modifier::Control).with(Modifier::Alt);
    let event = |state, modifiers, repeat| NormalizedInputEvent::Key {
        code,
        state,
        modifiers,
        repeat,
    };
    assert!(open_key(&[event(ButtonState::Pressed, modifiers, false)]));
    assert!(!open_key(&[event(ButtonState::Pressed, modifiers, true)]));
    assert!(!open_key(&[event(ButtonState::Released, modifiers, false)]));
    assert!(!open_key(&[event(
        ButtonState::Pressed,
        modifiers.with(Modifier::Shift),
        false
    )]));
    assert!(!open_key(&[event(
        ButtonState::Pressed,
        Modifiers::NONE,
        false
    )]));
    let mut panel = Panel::default();
    assert!(!request(&mut panel, 1));
    assert!(panel_text(&panel).contains("Unavailable"));
    panel.selected = true;
    assert!(request(&mut panel, 1));
    assert!(!request(&mut panel, 2));
    assert_eq!(panel.pending_kind, 1);
    assert!(panel_text(&panel).contains("Waiting"));
    panel.failed("The session has ended");
    assert!(!panel.pending);
    assert!(panel_text(&panel).contains("ended"));
    Ok(())
}

#[test]
fn bestiary_browser_filters_confirmed_stages_and_selects_without_index_entry()
-> Result<(), &'static str> {
    use oteryn_client_runtime::charm_progression::{
        CharmProgressionOutcome, CharmProgressionReply, bestiary::BestiaryRaceProgress,
        charm::CharmView,
    };
    let mut panel = Panel {
        selected: true,
        reply: Some(CharmProgressionReply {
            outcome: CharmProgressionOutcome::Query,
            fee_gold: 0,
            charms: CharmView::default(),
            bestiary: vec![
                BestiaryRaceProgress {
                    race: std::num::NonZeroU32::new(1).ok_or("race")?,
                    kill_count: 3,
                    kill_thresholds: [5, 10, 20],
                },
                BestiaryRaceProgress {
                    race: std::num::NonZeroU32::new(2).ok_or("race")?,
                    kill_count: 20,
                    kill_thresholds: [5, 10, 20],
                },
            ],
        }),
        ..Default::default()
    };
    panel.race_filter = RaceFilter::Complete;
    panel.select_race(false);
    assert_eq!(panel.fields[1], "2");
    assert!(bestiary_text(&panel).contains("Stage 3: 20/20"));
    assert!(!bestiary_text(&panel).contains("Race 1:"));
    panel.race_filter = RaceFilter::InProgress;
    panel.select_race(false);
    assert_eq!(panel.fields[1], "1");
    assert!(bestiary_text(&panel).contains("Stage 1: 3/5"));
    panel.pending = true;
    panel.select_race(true);
    assert_eq!(panel.fields[1], "1");
    Ok(())
}
