use super::*;
use crate::play::{mailbox::lock, play_channel};
type TestResult = Result<(), Box<dyn std::error::Error>>;
pub(super) fn initialize(link: &PlayLink) -> TestResult {
    let mut mailbox = lock(&link.mailbox);
    mailbox.properties.container_sort.selected = true;
    mailbox.properties.container_sort.open = true;
    mailbox.properties.container_sort.fields = ["3", "2", "1", "0"].map(str::to_owned);
    mailbox.containers.views = Some(oteryn_session::container_tree::ContainerViews {
        views: vec![oteryn_session::container_tree::ContainerView {
            view_id: 3,
            container_handle: 42.try_into()?,
            parent_handle: None,
            capacity: 20,
            entries: vec![],
            browse: None,
        }],
    });
    Ok(())
}
#[test]
fn container_sorting_window_shows_server_data_only() -> TestResult {
    let (link, _, _) = play_channel();
    initialize(&link)?;
    let snapshot = link.container_sorting_snapshot();
    assert_eq!(snapshot.panel, link.container_sort_panel());
    assert_eq!(snapshot.views, link.container_panel().views);
    assert_eq!(
        snapshot.views.as_ref().ok_or("views")?.views[0].container_handle,
        42.try_into()?
    );
    Ok(())
}
#[test]
fn container_sorting_actions_are_bound_and_confirmed() -> TestResult {
    let (link, mut commands, _) = play_channel();
    initialize(&link)?;
    let snapshot = link.container_sorting_snapshot();
    assert!(!link.container_sorting_dispatch_bound(
        &snapshot,
        ContainerSortingControl::Sort { confirmed: false }
    ));
    assert_eq!(snapshot, link.container_sorting_snapshot());
    assert!(commands.try_recv().is_err());
    assert!(link.container_sorting_dispatch_bound(
        &snapshot,
        ContainerSortingControl::Sort { confirmed: true }
    ));
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("feature command".into());
    };
    assert_eq!(command.payload, [3, 2, 1, 0]);
    assert_eq!(command.capability, 146);
    assert_eq!(command.kind, 168);
    assert_eq!(link.container_sort_panel().summary, snapshot.panel.summary);
    assert_eq!(link.container_panel().views, snapshot.views);
    assert!(link.container_sort_panel().pending);
    assert!(!link.container_sorting_dispatch_bound(
        &snapshot,
        ContainerSortingControl::Sort { confirmed: true }
    ));
    assert!(commands.try_recv().is_err());
    let result = oteryn_client_runtime::raw_feature::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: vec![1],
    };
    super::super::protocol::apply(
        &mut lock(&link.mailbox).properties.container_sort,
        &command,
        &result,
    );
    assert!(!link.container_sort_panel().pending);
    assert!(
        link.container_sort_panel()
            .summary
            .contains("Container sorted")
    );
    assert_eq!(link.container_panel().views, snapshot.views);
    Ok(())
}
#[test]
fn container_sorting_rejected_when_session_replaced_or_target_changed() -> TestResult {
    let (link, mut commands, _) = play_channel();
    initialize(&link)?;
    let snapshot = link.container_sorting_snapshot();
    let (replacement, mut replacement_commands, _) = play_channel();
    initialize(&replacement)?;
    let replacement_snapshot = replacement.container_sorting_snapshot();
    assert!(!replacement.container_sorting_dispatch_bound(
        &snapshot,
        ContainerSortingControl::Sort { confirmed: true }
    ));
    assert_eq!(
        replacement_snapshot,
        replacement.container_sorting_snapshot()
    );
    assert!(replacement_commands.try_recv().is_err());
    lock(&link.mailbox).properties.container_sort.open = false;
    assert!(!link.container_sorting_dispatch_bound(
        &snapshot,
        ContainerSortingControl::Sort { confirmed: true }
    ));
    lock(&link.mailbox).properties.container_sort.open = true;
    lock(&link.mailbox)
        .containers
        .views
        .as_mut()
        .ok_or("views")?
        .views[0]
        .container_handle = 43.try_into()?;
    let current = link.container_sorting_snapshot();
    assert!(!link.container_sorting_dispatch_bound(
        &snapshot,
        ContainerSortingControl::Sort { confirmed: true }
    ));
    assert_eq!(current, link.container_sorting_snapshot());
    let modal = link.bevy_server_modal_fixture();
    assert!(!link.container_sorting_dispatch_bound(
        &current,
        ContainerSortingControl::Sort { confirmed: true }
    ));
    assert_eq!(current, link.container_sorting_snapshot());
    drop(modal);
    lock(&link.mailbox).inventory_shutdown = true;
    let ended = link.container_sorting_snapshot();
    assert!(!link.container_sorting_dispatch_bound(
        &ended,
        ContainerSortingControl::Sort { confirmed: true }
    ));
    assert_eq!(ended, link.container_sorting_snapshot());
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn container_sorting_window_registered() {
    let modules = include_str!("../../bevy_shell/game_ui/mod.rs");
    let toolbar = include_str!("../../bevy_shell/ui.rs");
    assert!(modules.contains("mod container_sorting;"));
    assert!(modules.contains("container_sorting::install(app)"));
    assert!(toolbar.contains("shell.container_sorting"));
}

#[test]
fn container_sorting_rejects_closed_dispatcher_without_state_change() -> TestResult {
    let (link, commands, _) = play_channel();
    initialize(&link)?;
    let snapshot = link.container_sorting_snapshot();
    drop(commands);
    assert!(!link.container_sorting_dispatch_bound(
        &snapshot,
        ContainerSortingControl::Sort { confirmed: true },
    ));
    assert_eq!(snapshot, link.container_sorting_snapshot());
    Ok(())
}
