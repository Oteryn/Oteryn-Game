//! Own party shared-experience state and server-confirmed party bonuses.
pub use oteryn_client_runtime::party_effects::*;
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::request;
pub use state::PartyEffectsPanel;
pub use view::panel_text;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn party_effects_panel_consumes_only_valid_server_reply_and_retains_on_refusal()
    -> Result<(), &'static str> {
        let mut panel = PartyEffectsPanel::default();
        assert!(request(&mut panel).is_none());
        panel.selected = true;
        assert_eq!(request(&mut panel), Some(Vec::new()));
        assert!(request(&mut panel).is_none());
        let value = PartyEffects {
            in_party: true,
            shared_enabled: true,
            shared_active: false,
            eligible: false,
            bonuses: [6, 12, 6, 12, 50],
            party_spell_remaining_ms: [0, 120000, 0, 0, 0],
        };
        assert!(panel.finish(true, &value.encode().ok_or("encode")?));
        let text = panel_text(&panel);
        assert!(text.contains("Enabled, waiting for eligibility"));
        assert!(text.contains("Damage reduction: 6%"));
        assert!(text.contains("Mantra: +50"));
        assert!(text.contains("Protect Party: 120s"));
        assert!(request(&mut panel).is_some());
        assert!(!panel.finish(false, &[]));
        assert_eq!(panel.snapshot, Some(value));
        assert!(request(&mut panel).is_some());
        assert!(!panel.finish(true, &[0]));
        assert_eq!(panel.snapshot, Some(value));
        Ok(())
    }
}
