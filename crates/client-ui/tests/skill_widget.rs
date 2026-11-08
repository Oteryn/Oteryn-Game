use oteryn_client_ui::skill_progress::{SkillProgressPanel, SkillProgressSnapshot, SkillProgress, request};

#[test]
fn t031_skill_widget_projects_confirmed_progress_and_configures_rows() -> Result<(), &'static str> {
    let mut panel = SkillProgressPanel { selected: true, ..Default::default() };
    assert!(panel.gauges().is_empty());
    assert!(request(&mut panel).is_some());
    let snapshot = SkillProgressSnapshot { skills: [SkillProgress { level: 42, progress: 1, required: 2, percent: 5000 }; 8] };
    assert!(panel.finish(true, &snapshot.encode().ok_or("encode")?));
    assert_eq!(panel.gauges().len(), 8);
    assert_eq!(panel.gauges()[3], ("Sword", 5000, true));
    panel.toggle_skill(3);
    assert_eq!(panel.gauges()[3], ("Sword", 5000, false));
    assert!(!oteryn_client_ui::skill_progress::panel_text(&panel).contains("Sword:"));
    panel.toggle_skill(8); // Invalid controls never change the valid mask.
    assert_eq!(panel.gauges()[3], ("Sword", 5000, false));
    Ok(())
}
