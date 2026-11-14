//! Server-event fixtures for the bound frontend tests.
use crate::play::{Outbox, mailbox::lock};
impl Outbox {
    pub(crate) fn bevy_browse_field_selected(&self, selected: bool) {
        self.browse_field_initialize(selected);
    }
    pub(crate) fn bevy_browse_field_data(&self) -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_session::{
            ItemEntry,
            browse_field::BrowseFieldPage,
            container_tree::{ContainerView, ContainerViews},
            item_view::WorldTilePosition,
        };
        let item = ItemEntry {
            handle: 1.try_into()?,
            item_definition_ref: 42.try_into()?,
            count: 3.try_into()?,
            sub_type: 0,
        };
        let views = ContainerViews {
            views: vec![ContainerView {
                view_id: 3,
                container_handle: 50.try_into()?,
                parent_handle: None,
                capacity: 30,
                entries: vec![item],
                browse: Some(BrowseFieldPage {
                    position: WorldTilePosition {
                        x: 100,
                        y: 200,
                        floor: -7,
                    },
                    index: 0,
                    total: 1,
                }),
            }],
        };
        let bytes = oteryn_session::container_tree::encode_container_views_with_house_capacity(
            &views, true, false,
        )
        .map_err(|e| format!("{e:?}"))?;
        self.browse_field_views(&bytes);
        self.browse_field_inventory(oteryn_session::CharacterInventory {
            entries: vec![ItemEntry {
                handle: 2.try_into()?,
                item_definition_ref: 99.try_into()?,
                count: 1.try_into()?,
                sub_type: 0,
            }],
            ..Default::default()
        });
        Ok(())
    }
    pub(crate) fn bevy_browse_field_reply(&self) {
        self.browse_field_moved(None);
        lock(&self.0).world_context.pending = false;
    }
    pub(crate) fn bevy_browse_field_change_target(&self) {
        self.push(vec![oteryn_session::SessionEvent::ContainerViews(
            oteryn_session::AppliedDelta {
                server_sequence: 2,
                base_revision: 1,
                new_revision: 2,
                value: Default::default(),
            },
        )]);
    }
}

#[test]
fn browse_field_owner_rejects_foreign_closed_changed_duplicate_and_modal_bindings()
-> Result<(), Box<dyn std::error::Error>> {
    use super::bound::Action;
    use crate::play::play_channel;
    let (link, mut rx, outbox) = play_channel();
    outbox.bevy_browse_field_selected(true);
    link.browse_field_set_open_bound(true);
    outbox.bevy_browse_field_data()?;
    let binding = link.browse_field_binding();
    let fields = ["100", "200", "-7", "3", "0", "0", "0"].map(str::to_owned);
    let (other, mut other_rx, other_out) = play_channel();
    other_out.bevy_browse_field_selected(true);
    other.browse_field_set_open_bound(true);
    other_out.bevy_browse_field_data()?;
    assert!(!other.browse_field_dispatch_bound(&binding, Action::Item(105), &fields, true));
    assert!(other_rx.try_recv().is_err());
    assert!(!link.browse_field_dispatch_bound(&binding, Action::Item(106), &fields, false));
    assert!(rx.try_recv().is_err());
    assert_eq!(link.browse_field_panel(), binding.panel);
    assert!(link.browse_field_dispatch_bound(&binding, Action::Item(109), &fields, true));
    assert!(!link.browse_field_dispatch_bound(&binding, Action::Item(109), &fields, true));
    assert!(matches!(
        rx.try_recv()?,
        crate::combat_input::PlayCommand::WorldContext(_)
    ));
    assert!(rx.try_recv().is_err());
    outbox.bevy_browse_field_reply();
    let binding = link.browse_field_binding();
    outbox.bevy_browse_field_change_target();
    let changed = link.browse_field_panel();
    assert!(!link.browse_field_dispatch_bound(&binding, Action::Item(105), &fields, true));
    assert_eq!(link.browse_field_panel(), changed);
    outbox.bevy_browse_field_data()?;
    let binding = link.browse_field_binding();
    let modal = link.bevy_server_modal_fixture();
    assert!(!link.browse_field_dispatch_bound(&binding, Action::Open, &fields, true));
    assert_eq!(link.browse_field_panel(), binding.panel);
    drop(modal);
    assert!(link.browse_field_dispatch_bound(&binding, Action::Close, &fields, true));
    assert!(!link.browse_field_dispatch_bound(&binding, Action::Open, &fields, true));
    link.browse_field_set_open_bound(true);
    outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    assert!(!link.browse_field_dispatch_bound(&binding, Action::Open, &fields, true));
    assert!(!link.browse_field_set_open_bound(true));
    assert!(rx.try_recv().is_err());
    Ok(())
}

// Share the production presenter with the pre-existing native assertions: loading
// native.rs a second time in this test target is rejected by Clippy's duplicate_mod.
#[cfg(not(windows))]
pub(in crate::play) mod native {
    use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
    use oteryn_client_ui::browse_field::BrowseFieldPanel;
    pub(in crate::play) fn enabled(panel: &BrowseFieldPanel) -> bool {
        super::super::native::enabled(panel)
    }
    pub(in crate::play) fn submit(
        panel: &mut BrowseFieldPanel,
        seeking: bool,
        fields: [&str; 5],
    ) -> Option<ServerMechanicCommand> {
        super::super::native::submit(panel, seeking, fields)
    }
    pub(in crate::play) fn result_text(panel: &BrowseFieldPanel) -> String {
        super::super::native::result_text(panel)
    }
    pub(in crate::play) fn item_command(
        panel: &mut BrowseFieldPanel,
        action: usize,
        field_row: &str,
        bag_row: &str,
    ) -> Option<crate::combat_input::PlayCommand> {
        super::super::native::item_command(panel, action, field_row, bag_row)
    }
}
