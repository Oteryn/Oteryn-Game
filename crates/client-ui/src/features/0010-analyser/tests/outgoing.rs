use super::{
    Panel, Tool,
    protocol::{apply_at, reset},
};
#[test]
fn outgoing_views_show_own_rates_types_and_charts_without_mixing_metrics()
-> Result<(), &'static str> {
    let facts = oteryn_client_runtime::analyser::decode_analyser_facts(&[
        10, 8, 34, 6, 8, 1, 16, 120, 24, 2, 10, 6, 34, 4, 8, 2, 16, 60,
    ])
    .map_err(|_| "impact wire")?;
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    apply_at(&mut panel, &facts, 0);
    let damage = super::view::outgoing_text(&panel, 60, Tool::Damage);
    assert!(damage.lines().any(|line| line == "DPS: 0.00"));
    assert!(
        damage
            .lines()
            .any(|line| line == "Session average DPS: 2.00")
    );
    for expected in ["Damage per minute", "Fire: 120", "100.00%"] {
        assert!(damage.contains(expected), "{damage}");
    }
    assert!(!damage.contains("HPS:"));
    let healing = super::view::outgoing_text(&panel, 60, Tool::Healing);
    assert!(healing.lines().any(|line| line == "HPS: 0.00"));
    assert!(
        healing
            .lines()
            .any(|line| line == "Session average HPS: 1.00")
    );
    for expected in ["Healing per minute", "Healing: 60"] {
        assert!(healing.contains(expected), "{healing}");
    }
    assert!(!healing.contains("DPS:") && !healing.contains("Fire:"));
    for second in 1..=20 {
        apply_at(&mut panel, &facts, second);
    }
    let steady = super::view::impact_text(&panel, 20);
    assert!(
        steady
            .lines()
            .any(|line| line == "DPS: 120.00 | HPS: 60.00"),
        "{steady}"
    );
    let damage = super::view::outgoing_text(&panel, 20, Tool::Damage);
    assert!(damage.lines().any(|line| line == "DPS: 120.00"), "{damage}");
    let healing = super::view::outgoing_text(&panel, 20, Tool::Healing);
    assert!(
        healing.lines().any(|line| line == "HPS: 60.00"),
        "{healing}"
    );
    reset(&mut panel);
    assert!(
        super::view::outgoing_text(&panel, 0, Tool::Healing)
            .contains("No confirmed measurements yet")
    );
    Ok(())
}
