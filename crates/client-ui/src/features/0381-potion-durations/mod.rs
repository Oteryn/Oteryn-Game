//! Current potion timers from the server's active-effect projection.
pub use oteryn_client_runtime::potion_durations::*;
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::request;
pub use state::PotionDurationsPanel;
pub use view::{panel_text, panel_text_at};
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    #[test]
    fn nx8_potion_durations_play_receives_all_sources_counts_down_and_clears_expiry()
    -> Result<(), &'static str> {
        let mut panel = PotionDurationsPanel::default();
        assert!(request(&mut panel).is_none());
        panel.selected = true;
        assert_eq!(request(&mut panel), Some(Vec::new()));
        assert!(request(&mut panel).is_none());
        let snapshot = PotionDurations {
            active: vec![
                PotionDuration {
                    source: 1,
                    item_id: 36727,
                    percent: 0,
                    remaining_ms: 3_600_000,
                },
                PotionDuration {
                    source: 2,
                    item_id: 35563,
                    percent: 0,
                    remaining_ms: 60_000,
                },
                PotionDuration {
                    source: 3,
                    item_id: 0,
                    percent: 50,
                    remaining_ms: 120_000,
                },
                PotionDuration {
                    source: 4,
                    item_id: 0,
                    percent: 25,
                    remaining_ms: 30_000,
                },
            ],
        };
        let now = Instant::now();
        assert!(panel.finish_at(true, &snapshot.encode().ok_or("encode")?, now));
        let later = now + std::time::Duration::from_secs(31);
        let text = panel_text_at(&panel, later);
        assert!(text.contains("Wealth Duplex: 59:29"));
        assert!(text.contains("Magic shield potion: 00:29"));
        assert!(text.contains("Store XP boost +50%: 01:29"));
        assert!(!text.contains("Daily XP boost"));
        assert!(request(&mut panel).is_some());
        assert!(!panel.finish_at(false, &[], later));
        assert_eq!(panel.snapshot, Some(snapshot));
        assert_eq!(panel.received_at, Some(now));
        assert!(panel.finish_at(
            true,
            &PotionDurations::default().encode().ok_or("empty")?,
            later
        ));
        assert!(panel_text_at(&panel, later).contains("No active potion effects"));
        Ok(())
    }
}
