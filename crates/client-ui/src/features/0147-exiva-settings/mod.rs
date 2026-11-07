pub use oteryn_client_runtime::exiva::*;
mod input;
mod protocol;
mod state;
mod view;
pub use input::open_key;
pub use protocol::request;
pub use state::ExivaPanel;
pub use view::panel_text;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exiva_editor_requires_selection_and_authoritative_refresh() -> Result<(), &'static str> {
        let mut panel = ExivaPanel::default();
        assert!(request(&mut panel, vec![5]).is_none());
        panel.selected = true;
        assert_eq!(request(&mut panel, vec![5]), Some(vec![5]));
        assert!(request(&mut panel, vec![6, b'A', b'l']).is_none());
        let list = PlayerList {
            modes: 2,
            players: vec![([3; 16], "Al".into())],
        };
        panel.pending = false;
        assert!(panel.apply(&list.encode().ok_or("list")?));
        assert!(panel_text(&panel).contains("Al"));
        let mut remove = vec![2];
        remove.extend([3; 16]);
        assert!(request(&mut panel, remove).is_some());
        assert_eq!(panel.list, Some(list)); // no optimistic local removal
        panel.pending = false;
        assert!(!panel.apply(&[0]));
        assert!(panel.error.is_some());
        assert!(request(&mut panel, vec![6]).is_none());
        Ok(())
    }
}
