use super::*;
use oteryn_client_runtime::boss_encounter::Snapshot;
#[test]
fn boss_browser_search_categories_and_detail_selection_use_server_records() {
    let mut p = BossEncounterPanel {
        selected: true,
        snapshot: Some(Snapshot {
            now: 100,
            progress: vec![("oteryn:dragon".into(), 7), ("oteryn:rat".into(), 1)],
            cooldowns: vec![("oteryn:dragon".into(), 200)],
            ..Default::default()
        }),
        ..Default::default()
    };
    p.browser_search = " DRAGON ".into();
    assert_eq!(boss_records(&p), ["oteryn:dragon"]);
    assert!(select_boss(&mut p, true));
    assert_eq!(p.fields[0], "oteryn:dragon");
    assert!(browser_text(&p).contains("7 kills"));
    assert!(browser_text(&p).contains("100 ms"));
    p.browser_search.clear();
    p.browser_category = BossCategory::Cooldown;
    assert_eq!(boss_records(&p).len(), 1);
    p.pending = true;
    assert!(!select_boss(&mut p, false));
}
use super::*;

#[test]
fn bosstiary_browser_filters_and_selects_confirmed_bosses() {
    let mut panel = BossEncounterPanel {
        snapshot: Some(oteryn_client_runtime::boss_encounter::Snapshot {
            progress: vec![("boss:dragon".into(), 12), ("boss:rat".into(), 2)],
            ..Default::default()
        }),
        ..Default::default()
    };
    panel.boss_search = "DRAGON".into();
    panel.select_boss(false);
    assert_eq!(panel.fields[0], "boss:dragon");
    assert!(panel_text(&panel).contains("boss:dragon: 12 kills"));
    assert!(!panel_text(&panel).contains("boss:rat: 2 kills"));
    panel.pending = true;
    panel.boss_search = "rat".into();
    panel.select_boss(false);
    assert_eq!(panel.fields[0], "boss:dragon");
}

#[test]
fn boss_slot_selector_uses_server_unlocks_and_preserves_confirmed_bonus() -> Result<(), &'static str>
{
    use oteryn_client_runtime::boss_encounter::bosstiary_slots::BossSlotReply;
    let mut panel = BossEncounterPanel {
        selected: true,
        ..Default::default()
    };
    let reply = BossSlotReply {
        status: 0,
        unlocked: 2,
        bonus: 25,
        points: 600,
        remove_fee: 1000,
        ready_at_ms: 900,
        bosses: [String::new(), "boss:dragon".into()],
    };
    assert!(request(&mut panel, Intent::SlotsQuery).is_some());
    assert!(apply_result(
        &mut panel,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: reply.encode().ok_or("encode")?,
        }
    ));
    panel.select_slot();
    assert_eq!(panel.fields[1], "2");
    assert_eq!(panel.slots.as_ref(), Some(&reply));
    let text = panel_text(&panel);
    assert!(text.contains("Slot 1: Locked"));
    assert!(text.contains("Slot 2: boss:dragon"));
    assert!(text.contains("Confirmed bonus: 25"));
    assert!(request(&mut panel, Intent::SlotsQuery).is_some());
    assert!(!apply_result(
        &mut panel,
        &GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload: vec![255],
        }
    ));
    assert_eq!(panel.slots.as_ref(), Some(&reply));
    Ok(())
}
