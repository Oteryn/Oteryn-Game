use super::*;

#[test]
fn fill_clamps_and_survives_zero_max() {
    assert!((fill_px(50, 100) - 60.0).abs() < f32::EPSILON);
    assert!((fill_px(500, 100) - BAR_WIDTH_PX).abs() < f32::EPSILON);
    assert!(fill_px(5, 0).abs() < f32::EPSILON);
}

#[test]
fn bars_draw_track_and_fill_per_vital_and_nothing_without_vitals() -> Result<(), BatchError> {
    let atlas = AtlasImage::new(2, 3, 1, vec![0; 3 * 2 * 2 * 4])?;
    let cells = (0, 1, 2);
    assert!(vitals_bars(&atlas, None, [0.0, 0.0], cells)?.is_empty());
    let vitals = ActorVitals {
        health: 75,
        max_health: 150,
        mana: 30,
        max_mana: 30,
        ..ActorVitals::default()
    };
    let quads = vitals_bars(&atlas, Some(&vitals), [8.0, 8.0], cells)?;
    assert_eq!(quads.len(), 4);
    assert!((quads[1].size[0] - 60.0).abs() < f32::EPSILON);
    assert!((quads[3].size[0] - BAR_WIDTH_PX).abs() < f32::EPSILON);
    assert!(quads[2].position[1] > quads[0].position[1]);
    Ok(())
}

#[test]
fn hud_shows_current_max_and_bounded_bars() {
    assert_eq!(vitals_text(None), "HP —   MP —");
    let vitals = ActorVitals {
        health: 50,
        max_health: 100,
        mana: 5,
        max_mana: 0,
        ..ActorVitals::default()
    };
    assert_eq!(
        vitals_text(Some(&vitals)),
        "HP [█████░░░░░] 50/100   MP [░░░░░░░░░░] 5/0"
    );
}

#[test]
fn hud_clamps_overfull_and_handles_largest_vitals() {
    let vitals = ActorVitals {
        health: u32::MAX,
        max_health: 1,
        mana: u32::MAX,
        max_mana: u32::MAX,
        ..ActorVitals::default()
    };
    assert_eq!(
        vitals_text(Some(&vitals)),
        "HP [██████████] 4294967295/1   MP [██████████] 4294967295/4294967295"
    );
}
