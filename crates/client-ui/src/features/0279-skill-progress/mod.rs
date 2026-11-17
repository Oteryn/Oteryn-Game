//! Server-owned skill counters and progress bars, including magic level.
pub use oteryn_client_runtime::skill_progress::*;
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::request;
pub use state::{SKILL_LABELS, SkillProgressPanel};
pub use view::panel_text;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_progress_panel_waits_for_server_then_displays_all_bars() -> Result<(), &'static str> {
        let mut panel = SkillProgressPanel::default();
        assert!(request(&mut panel).is_none());
        panel.selected = true;
        assert!(request(&mut panel).is_some());
        assert!(request(&mut panel).is_none());
        let snapshot = SkillProgressSnapshot {
            skills: std::array::from_fn(|family| SkillProgress {
                level: 10,
                progress: family as u64 * 10,
                required: 100,
                percent: family as u16 * 1_000,
            }),
        };
        assert!(panel.finish(true, &snapshot.encode().ok_or("encode")?));
        let text = panel_text(&panel);
        assert!(text.contains("Magic: 10 [----------] 0.00% (0/100)"));
        assert!(text.contains("Sword: 10 [###-------] 30.00% (30/100)"));
        assert!(text.contains("Fishing: 10 [#######---] 70.00% (70/100)"));
        assert!(request(&mut panel).is_some());
        assert!(!panel.finish(false, &[]));
        assert_eq!(panel.snapshot, Some(snapshot));
        assert!(request(&mut panel).is_some());
        assert!(!panel.finish(true, &[0]));
        assert_eq!(panel.snapshot, Some(snapshot));
        assert!(!panel.pending);
        Ok(())
    }
}
