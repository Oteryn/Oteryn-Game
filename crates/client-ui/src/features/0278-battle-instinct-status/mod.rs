//! Own-player Battle Instinct status confirmed by the server.
pub use oteryn_client_runtime::battle_instinct_status::*;
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::request;
pub use state::BattleInstinctStatusPanel;
pub use view::panel_text;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battle_instinct_panel_requires_a_confirmed_snapshot_and_preserves_it_on_failure()
    -> Result<(), &'static str> {
        let mut panel = BattleInstinctStatusPanel::default();
        assert!(request(&mut panel).is_none());
        panel.selected = true;
        assert_eq!(request(&mut panel), Some(Vec::new()));
        assert!(request(&mut panel).is_none());
        let status = BattleInstinctStatus {
            active: true,
            melee: 4,
            shielding: 24,
        };
        assert!(panel.finish(true, &status.encode().ok_or("encode")?));
        assert_eq!(panel.snapshot, Some(status));
        let text = panel_text(&panel);
        assert!(text.contains("Melee: +4"));
        assert!(text.contains("Shielding: +24"));
        assert!(request(&mut panel).is_some());
        assert!(!panel.finish(true, &[1]));
        assert_eq!(panel.snapshot, Some(status));
        assert!(request(&mut panel).is_some());
        assert!(!panel.finish(false, &[]));
        assert_eq!(panel.snapshot, Some(status));
        assert!(request(&mut panel).is_some());
        assert!(panel.finish(
            true,
            &BattleInstinctStatus::default().encode().ok_or("encode")?
        ));
        assert!(panel_text(&panel).contains("Inactive"));
        panel = BattleInstinctStatusPanel::default();
        assert!(panel.snapshot.is_none());
        Ok(())
    }
}
