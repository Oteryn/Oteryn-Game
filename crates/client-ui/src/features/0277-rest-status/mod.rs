//! Server-owned resting-area bonuses and remaining food time.
pub use oteryn_client_runtime::rest_status::*;
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::request;
pub use state::RestStatusPanel;
pub use view::panel_text;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rest_status_panel_consumes_server_state_and_clears_on_refusal() -> Result<(), &'static str> {
        let mut panel = RestStatusPanel::default();
        assert!(request(&mut panel).is_none());
        panel.selected = true;
        assert_eq!(request(&mut panel), Some(Vec::new()));
        assert!(request(&mut panel).is_none());
        let status = RestStatus {
            inside_resting_area: true,
            streak: 7,
            food_remaining_ms: 180_000,
            health_multiplier: 2,
            mana_multiplier: 2,
            stamina: true,
            soul: true,
        };
        assert!(panel.finish(true, &status.encode().ok_or("encode")?));
        assert_eq!(panel.snapshot, Some(status));
        assert!(panel_text(&panel).contains("Food: 180 s"));
        assert!(panel_text(&panel).contains("Rest health bonus: double"));
        assert!(request(&mut panel).is_some());
        assert!(!panel.finish(false, &[]));
        assert!(panel.snapshot.is_none());
        assert!(panel.error.is_some());
        assert!(request(&mut panel).is_some());
        assert!(!panel.finish(true, &[0]));
        assert!(panel.snapshot.is_none());
        panel.clear();
        assert_eq!(panel, RestStatusPanel::default());
        Ok(())
    }
}
