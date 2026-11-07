use super::*;
use crate::world_event::WorldEventPanel;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

#[test]
fn seasonal_observation_requires_negotiation() {
    let result = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: vec![34, 8, 8, 1, 16, 1, 24, 1, 32, 2],
    };
    let mut panel = WorldEventPanel {
        selected: true,
        ..Default::default()
    };
    apply_result(&mut panel, &result);
    assert!(panel.state.is_none());
    panel.seasonal_selected = true;
    apply_result(&mut panel, &result);
    assert!(panel_text(&panel).contains("Halloween: active"));
    assert!(!panel_text(&panel).contains("No active world event"));
    assert!(!panel_text(&panel).contains("Reset epoch"));
}

#[test]
fn cake_progress_presentation_retains_bonus_and_labels_pending_next_save()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::world_event::{
        CakeProgress, WorldEventState, encode_world_event_state,
    };
    let mut state = WorldEventState {
        reset_epoch: 0,
        active: false,
        announcement: String::new(),
        seasonal: Vec::new(),
        cake: Some(CakeProgress {
            stage: 0,
            substances: 200,
            golems: 200,
            layer: 4,
            layer_progress: 1116,
            layer_goal: 1116,
            occurrence_start: 1_799_000_000,
            occurrence_end: 1_799_518_400,
            window_end: None,
            bonus_until: Some(1_800_604_800),
            observed_unix: 1_800_100_000,
        }),
    };
    let mut result = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: encode_world_event_state(&state).map_err(|error| format!("{error:?}"))?,
    };
    let mut panel = WorldEventPanel {
        selected: true,
        ..Default::default()
    };
    apply_result(&mut panel, &result);
    assert!(panel.state.is_none()); // Calendar may be empty; Cake still requires371.
    panel.seasonal_selected = true;
    apply_result(&mut panel, &result);
    let text = panel_text(&panel);
    assert!(text.contains("A Piece of Cake: event finished"));
    assert!(text.contains("Premium experience +50%; bonus regeneration +25%"));
    assert!(text.contains("Server observation:"));
    assert!(!text.contains("seconds remaining"));
    state
        .cake
        .as_mut()
        .ok_or("fixture Cake progress")?
        .observed_unix = 1_799_999_999;
    result.payload = encode_world_event_state(&state).map_err(|error| format!("{error:?}"))?;
    apply_result(&mut panel, &result);
    assert!(panel_text(&panel).contains("pending the next server save"));
    result.status = CommandStatus::DuplicateReplay;
    apply_result(&mut panel, &result);
    assert!(panel_text(&panel).contains("pending the next server save"));
    assert!(!panel_text(&panel).contains("World bonus active"));
    assert!(!panel_text(&panel).contains("Premium experience +50%"));
    state
        .cake
        .as_mut()
        .ok_or("fixture Cake progress")?
        .observed_unix = 1_800_604_800;
    result.payload = encode_world_event_state(&state).map_err(|error| format!("{error:?}"))?;
    apply_result(&mut panel, &result);
    let expired = panel_text(&panel);
    assert!(expired.contains("A Piece of Cake: event finished"));
    assert!(expired.contains("Earned world bonus ended"));
    assert!(!expired.contains("World bonus active"));
    assert!(!expired.contains("Premium experience +50%"));
    state
        .cake
        .as_mut()
        .ok_or("fixture Cake progress")?
        .observed_unix = 1_799_999_999;
    state.cake.as_mut().ok_or("fixture Cake progress")?.stage = 2;
    state
        .cake
        .as_mut()
        .ok_or("fixture Cake progress")?
        .window_end = Some(1_800_001_799);
    result.payload = encode_world_event_state(&state).map_err(|error| format!("{error:?}"))?;
    apply_result(&mut panel, &result);
    let text = panel_text(&panel);
    assert!(text.contains("cake bridge is open"));
    assert!(text.contains("Cake layer 4/4: 1116/1116 bites"));
    assert!(text.contains("Layer deadline:"));
    Ok(())
}
