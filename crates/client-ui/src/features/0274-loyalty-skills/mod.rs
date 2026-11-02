//! Native skills window: trained and loyalty-adjusted levels from the server.
pub use oteryn_client_runtime::loyalty_skills::*;
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::request;
pub use state::LoyaltySkillsPanel;
pub use view::panel_text;

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> LoyaltySkills {
        LoyaltySkills {
            points: 2_000,
            bonus_percent: 25,
            title: Some("Warrior of Tibia".into()),
            skills: [SkillLevel {
                base: 100,
                with_loyalty: 102,
            }; 8],
        }
    }

    #[test]
    fn loyalty_skills_panel_requires_selection_and_a_server_reply() -> Result<(), &'static str> {
        let mut panel = LoyaltySkillsPanel::default();
        assert!(request(&mut panel).is_none());
        panel.selected = true;
        assert_eq!(request(&mut panel), Some(Vec::new()));
        assert!(request(&mut panel).is_none());
        assert!(panel.snapshot.is_none());
        let snapshot = snapshot();
        assert!(panel.finish(true, &snapshot.encode().ok_or("encode")?));
        assert_eq!(panel.snapshot, Some(snapshot));
        assert!(!panel.pending);
        let text = panel_text(&panel);
        assert!(text.contains("Warrior of Tibia"));
        assert!(text.contains("25%"));
        assert!(text.contains("Magic: 100 / 102"));
        assert!(text.contains("Fishing: 100 / 102"));
        Ok(())
    }

    #[test]
    fn loyalty_skills_refusal_and_invalid_reply_preserve_last_confirmed_state()
    -> Result<(), &'static str> {
        let snapshot = snapshot();
        let mut panel = LoyaltySkillsPanel {
            selected: true,
            ..LoyaltySkillsPanel::default()
        };
        assert!(request(&mut panel).is_some());
        assert!(panel.finish(true, &snapshot.encode().ok_or("encode")?));
        assert!(request(&mut panel).is_some());
        assert!(!panel.finish(false, &[]));
        assert_eq!(panel.snapshot, Some(snapshot.clone()));
        assert!(panel.error.is_some());
        assert!(request(&mut panel).is_some());
        assert!(!panel.finish(true, &[0]));
        assert_eq!(panel.snapshot, Some(snapshot));
        assert!(!panel.pending);
        Ok(())
    }
}
