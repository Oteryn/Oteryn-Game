use super::*;

#[test]
fn status_order_settings_reach_confirmed_panels_and_survive_restart()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{ReferencePreferences, SettingsState};
    use oteryn_client_ui::{condition_status as conditions, skill_progress as skills};
    let path = std::env::temp_dir().join(format!("cgopt-status-order-{}.json", std::process::id()));
    let mut settings = SettingsState::new(Some(path.clone()));
    let mut json = serde_json::to_value(ReferencePreferences::default())?;
    json["skill_order"] = serde_json::json!([7, 6, 5, 4, 3, 2, 1, 0]);
    json["condition_order"] = serde_json::json!((1..=27).rev().collect::<Vec<_>>());
    let mut preferences: ReferencePreferences = serde_json::from_value(json)?;
    assert!(preferences.move_status_entry(true, 0, true));
    assert!(preferences.move_status_entry(true, 1, false));
    assert!(preferences.move_status_entry(false, 0, true));
    assert!(preferences.move_status_entry(false, 1, false));
    assert!(!preferences.move_status_entry(false, 0, false));
    assert!(!preferences.move_status_entry(true, 7, true));
    preferences.status_panel_visible = false;
    settings.apply(preferences);
    let (link, _, outbox) = play_channel();
    outbox.skill_progress_initialize(true);
    outbox.condition_status_initialize(true);
    settings.sync_local_preferences(&link);
    assert!(skills::panel_text(&link.skill_progress_panel()).contains("Loading"));
    let result = |payload| oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload,
    };
    let snapshot = skills::SkillProgressSnapshot {
        skills: [skills::SkillProgress {
            level: 42,
            progress: 1,
            required: 2,
            percent: 5000,
        }; 8],
    };
    outbox.skill_progress_result(&result(snapshot.encode().ok_or("encode skills")?));
    let status = conditions::ConditionStatus {
        conditions: vec![
            conditions::ActiveCondition {
                icon: 1,
                sub_id: 0,
                remaining_ms: 1_001,
            },
            conditions::ActiveCondition {
                icon: 13,
                sub_id: 0,
                remaining_ms: 5_000,
            },
        ],
    };
    outbox.condition_status_result(&result(status.encode().ok_or("encode conditions")?));
    let panel = link.skill_progress_panel();
    let text = skills::panel_text(&panel);
    assert!(text.find("Fishing:").ok_or("Fishing")? < text.find("Magic:").ok_or("Magic")?);
    assert_eq!(panel.gauges().first().map(|row| row.0), Some("Fishing"));
    let mut hidden = panel.clone();
    hidden.toggle_skill(0);
    assert!(!skills::panel_text(&hidden).contains("Fishing:"));
    assert!(skills::panel_text(&hidden).contains("Magic: 42"));
    let text = conditions::panel_text(&link.condition_status_panel());
    assert!(text.find("Haste:").ok_or("Haste")? < text.find("Poison:").ok_or("Poison")?);
    assert!(text.contains("Poison: 2s"));
    assert_eq!(link.skill_progress_panel().snapshot, Some(snapshot));
    assert_eq!(link.condition_status_panel().snapshot, Some(status));
    let restarted = SettingsState::new(Some(path.clone()));
    assert_eq!(restarted.preferences, settings.preferences);
    let mut invalid = serde_json::to_value(&settings.preferences)?;
    invalid["skill_order"] = serde_json::json!([0, 0, 1, 2, 3, 4, 5, 6]);
    let before = settings.preferences.clone();
    settings.apply(serde_json::from_value::<ReferencePreferences>(invalid)?);
    assert_eq!(settings.preferences, before);
    std::fs::remove_file(path)?;
    Ok(())
}
