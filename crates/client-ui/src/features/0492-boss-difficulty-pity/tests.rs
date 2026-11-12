use super::*;
use crate::boss_difficulty::{BossDifficultyPanel, apply_result, panel_text};
use oteryn_client_runtime::{
    boss_difficulty::{PityTracker, Reply, encode_reply, encode_reply_with_pity},
    raw_feature::{CommandStatus, GatedCommandResult},
};

fn window() -> Reply {
    Reply::Window {
        context: [1; 16],
        boss_key: "oteryn:moonsilver".into(),
        boss_creature_key: "oteryn:moonsilver".into(),
        content_revision: "r1".into(),
        selected_difficulty: 3,
        personal_highest: 2,
        unlocked_max: 4,
        maximum_difficulty: 25,
    }
}
fn tracker(active: bool, kills: u16, bonus: u16) -> PityTracker {
    PityTracker {
        kills_without_drop: kills,
        bonus_permille: bonus,
        tracker_active: active,
    }
}
fn result(payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload,
    }
}
fn panel(pity_selected: bool) -> BossDifficultyPanel {
    BossDifficultyPanel {
        selected: true,
        pity_selected,
        ..Default::default()
    }
}

#[test]
fn c0492_selected_windows_show_the_bad_luck_tracker() -> Result<(), &'static str> {
    let payload =
        encode_reply_with_pity(&window(), Some(&tracker(true, 37, 185))).map_err(|_| "encode")?;
    let mut p = panel(true);
    assert!(apply_result(&mut p, &result(payload)));
    assert_eq!(p.pity, Some(tracker(true, 37, 185)));
    let text = panel_text(&p);
    assert!(text.contains("Bad luck bonus: +18.5%"), "{text}");
    assert!(text.contains("Kills without drop: 37"), "{text}");
    Ok(())
}

#[test]
fn c0492_inactive_tracker_reads_inactive_and_has_no_bonus() -> Result<(), &'static str> {
    let payload =
        encode_reply_with_pity(&window(), Some(&tracker(false, 90, 0))).map_err(|_| "encode")?;
    let mut p = panel(true);
    assert!(apply_result(&mut p, &result(payload)));
    let text = panel_text(&p);
    assert!(text.contains("Bad luck tracker: inactive"), "{text}");
    assert!(!text.contains("Bad luck bonus"), "{text}");
    Ok(())
}

#[test]
fn c0492_unselected_session_keeps_the_legacy_window_and_rejects_tracker_fields()
-> Result<(), &'static str> {
    let legacy = encode_reply(&window()).map_err(|_| "encode")?;
    let mut p = panel(false);
    assert!(apply_result(&mut p, &result(legacy)));
    assert_eq!(p.pity, None);
    assert!(!panel_text(&p).contains("Bad luck"));
    let with =
        encode_reply_with_pity(&window(), Some(&tracker(true, 1, 5))).map_err(|_| "encode")?;
    assert!(decode(false, &with).is_none());
    assert_eq!(
        decode(true, &with).map(|(_, pity)| pity),
        Some(Some(tracker(true, 1, 5)))
    );
    Ok(())
}

#[test]
fn c0492_a_selected_session_still_takes_a_window_without_tracker() -> Result<(), &'static str> {
    let legacy = encode_reply(&window()).map_err(|_| "encode")?;
    let mut p = panel(true);
    p.pity = Some(tracker(true, 9, 9));
    assert!(apply_result(&mut p, &result(legacy)));
    assert_eq!(p.pity, None);
    Ok(())
}

#[test]
fn c0492_bonus_percent_formats_permille_with_one_decimal() {
    assert_eq!(bonus_percent(0), "+0.0%");
    assert_eq!(bonus_percent(5), "+0.5%");
    assert_eq!(bonus_percent(2000), "+200.0%");
}

#[test]
fn c0492_boss_window_text_shows_the_tracker_of_the_selected_boss() {
    use crate::boss_encounter::{BossEncounterPanel, browser_text};
    let mut p = BossEncounterPanel::default();
    p.fields[0] = "oteryn:moonsilver".into();
    p.difficulty_windows
        .insert("oteryn:moonsilver".into(), window());
    assert!(!browser_text(&p).contains("Bad luck"));
    p.pity_windows
        .insert("oteryn:moonsilver".into(), tracker(true, 12, 60));
    let text = browser_text(&p);
    assert!(text.contains("Bad luck bonus: +6.0%"), "{text}");
    assert!(text.contains("Kills without drop: 12"), "{text}");
}
