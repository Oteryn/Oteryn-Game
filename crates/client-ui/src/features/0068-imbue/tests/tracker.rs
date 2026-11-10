#[cfg(test)]
mod tracker_tests {
    use crate::imbue::*;
    #[test]
    fn low_time_alert_uses_confirmed_imbuements_and_clears_on_item_change() {
        let mut panel = ImbuePanel {
            item: Some(ItemHandle::MIN),
            window: Some(imbuement::ImbuementWindow {
                slot_count: 2,
                applied: vec![
                    imbuement::AppliedImbuement {
                        slot: 0,
                        family: 1,
                        tier: 1,
                        remaining_ms: 300_000,
                    },
                    imbuement::AppliedImbuement {
                        slot: 1,
                        family: 2,
                        tier: 1,
                        remaining_ms: 0,
                    },
                ],
                offers: vec![],
                clear_gold: 0,
            }),
            ..Default::default()
        };
        let text = panel_text(&panel);
        assert!(text.contains("LOW TIME"));
        assert!(text.contains("EXPIRED"));
        panel.select_item(ItemHandle::MIN.saturating_add(1));
        assert!(!panel_text(&panel).contains("LOW TIME"));
    }
}

#[cfg(test)]
mod time_filter_tests {
    use crate::imbue::*;
    #[test]
    fn tracker_time_filter_keeps_only_confirmed_slots_within_limit() {
        let mut panel = ImbuePanel {
            window: Some(imbuement::ImbuementWindow {
                slot_count: 2,
                applied: vec![
                    imbuement::AppliedImbuement {
                        slot: 0,
                        family: 1,
                        tier: 1,
                        remaining_ms: 299_000,
                    },
                    imbuement::AppliedImbuement {
                        slot: 1,
                        family: 2,
                        tier: 1,
                        remaining_ms: 900_000,
                    },
                ],
                offers: vec![],
                clear_gold: 0,
            }),
            ..Default::default()
        };
        panel.tracker_minutes = "5".into();
        let text = tracker_text(&panel);
        assert!(text.contains("Slot 1"));
        assert!(!text.contains("Slot 2"));
        panel.tracker_minutes = "0".into();
        assert!(tracker_text(&panel).contains("Slot 2"));
        panel.tracker_minutes = "bad".into();
        assert!(tracker_text(&panel).contains("Enter minutes"));
    }
}
