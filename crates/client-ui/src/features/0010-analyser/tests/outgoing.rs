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
    for expected in ["DPS: 2.00", "Damage per minute", "Fire: 120", "100.00%"] {
        assert!(damage.contains(expected), "{damage}");
    }
    assert!(!damage.contains("HPS:"));
    let healing = super::view::outgoing_text(&panel, 60, Tool::Healing);
    for expected in ["HPS: 1.00", "Healing per minute", "Healing: 60"] {
        assert!(healing.contains(expected), "{healing}");
    }
    assert!(!healing.contains("DPS:") && !healing.contains("Fire:"));
    reset(&mut panel);
    assert!(
        super::view::outgoing_text(&panel, 0, Tool::Healing)
            .contains("No confirmed measurements yet")
    );
    Ok(())
}
