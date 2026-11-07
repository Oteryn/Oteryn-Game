use super::*;
use oteryn_client_runtime::world_event::{
    CakeProgress, SeasonalEventKind, SeasonalEventState, WorldEventState,
};

#[test]
fn cake_bonus_panel_keeps_earned_reward_after_cleanup_and_labels_observation_time()
-> Result<(), Box<dyn std::error::Error>> {
    let bonus_start = 1_800_000_000;
    let bonus_end = bonus_start + 604800;
    let mut state = WorldEventState {
        reset_epoch: 0,
        active: false,
        announcement: String::new(),
        seasonal: Vec::new(),
        cake: None,
    };
    state.seasonal = vec![SeasonalEventState {
        event: SeasonalEventKind::BirthdayCake,
        start_unix: 1_820_000_000,
        end_unix: 1_820_518_400,
        active: false,
    }];
    state.cake = Some(CakeProgress {
        stage: 0,
        substances: 200,
        golems: 200,
        layer: 4,
        layer_progress: 1116,
        layer_goal: 1116,
        occurrence_start: 1_799_000_000,
        occurrence_end: 1_799_518_400,
        window_end: None,
        bonus_until: Some(bonus_end),
        observed_unix: bonus_start + 1,
    });
    let mut panel = WorldEventPanel {
        selected: true,
        seasonal_selected: true,
        state: Some(state),
        ..Default::default()
    };
    let text = panel_text(&panel);
    assert!(text.contains("event finished"));
    assert!(text.contains("World bonus active at server observation"));
    assert!(text.contains("Premium experience +50%; bonus regeneration +25%"));
    assert!(text.contains("Server observation:"));
    assert!(text.contains("UTC"));
    // A replay is a past server observation, so no locally restarted countdown.
    assert!(!text.contains("remaining"));
    let cake = panel
        .state
        .as_mut()
        .and_then(|state| state.cake.as_mut())
        .ok_or("fixture Cake progress")?;
    cake.observed_unix = bonus_end;
    let expired = panel_text(&panel);
    assert!(expired.contains("Earned world bonus ended"));
    assert!(!expired.contains("World bonus active"));
    Ok(())
}

#[test]
fn cake_bonus_panel_does_not_claim_benefits_before_next_server_save() {
    let mut state = WorldEventState {
        reset_epoch: 0,
        active: false,
        announcement: String::new(),
        seasonal: Vec::new(),
        cake: None,
    };
    state.cake = Some(CakeProgress {
        stage: 3,
        substances: 200,
        golems: 200,
        layer: 4,
        layer_progress: 1116,
        layer_goal: 1116,
        occurrence_start: 1_799_000_000,
        occurrence_end: 1_800_518_400,
        window_end: None,
        bonus_until: Some(1_800_604_800),
        observed_unix: 1_799_999_999,
    });
    let text = panel_text(&WorldEventPanel {
        selected: true,
        seasonal_selected: true,
        state: Some(state),
        ..Default::default()
    });
    assert!(text.contains("pending the next server save: begins"));
    assert!(!text.contains("World bonus active"));
}

#[test]
fn srv7_cake_layer_displays_server_qualified_food_goal() {
    let mut state = WorldEventState {
        reset_epoch: 0,
        active: false,
        announcement: String::new(),
        seasonal: Vec::new(),
        cake: None,
    };
    state.cake = Some(CakeProgress {
        stage: 2,
        substances: 200,
        golems: 200,
        layer: 1,
        layer_progress: 1066,
        layer_goal: 1066,
        occurrence_start: 1,
        occurrence_end: 2,
        window_end: None,
        bonus_until: None,
        observed_unix: 1,
    });
    let panel = WorldEventPanel {
        selected: true,
        seasonal_selected: true,
        state: Some(state),
        ..Default::default()
    };
    let text = panel_text(&panel);
    assert!(
        text.contains("1066/1066 bites"),
        "native panel must display authoritative source food goal: {text}"
    );
}
