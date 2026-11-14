use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub fn container_sort_panel(&self) -> oteryn_client_ui::container_sort::Panel {
        lock(&self.mailbox).properties.container_sort.clone()
    }
}
#[cfg(windows)]
pub(in crate::play) fn open_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Container sorting",
            labels: &[
                "Open container view (0-15)",
                "Sort key: 1 name, 2 weight, 3 expiry, 4 stack",
                "Descending: 0 no, 1 yes",
                "Recursive: 0 no, 1 yes",
            ],
            buttons: &["Sort"],
            snapshot: |mailbox| {
                let panel = &mailbox.properties.container_sort;
                (
                    panel.open,
                    panel.pending,
                    panel.fields.to_vec(),
                    oteryn_client_ui::container_sort::panel_text(panel),
                )
            },
            set_open: |mailbox, value| mailbox.properties.container_sort.open = value,
            submit: |mailbox, commands, action, values| {
                let panel = &mut mailbox.properties.container_sort;
                if panel.pending {
                    return;
                }
                for (index, value) in values.into_iter().enumerate() {
                    super::input::edit(panel, index, value);
                }
                let intent = match action {
                    0 => oteryn_client_ui::container_sort::Intent::Sort,
                    _ => return,
                };
                super::input::submit(mailbox, commands, intent);
            },
        },
    );
}

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "tests.rs"]
mod tests;

/// A presentation snapshot bound to this exact retained session and its real container views.
#[cfg(feature = "bevy-shell")]
#[derive(Clone, Debug)]
pub struct ContainerSortingSnapshot {
    pub panel: oteryn_client_ui::container_sort::Panel,
    pub views: Option<oteryn_session::container_tree::ContainerViews>,
    session: crate::play::mailbox::SharedMailbox,
}
#[cfg(feature = "bevy-shell")]
impl PartialEq for ContainerSortingSnapshot {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.session, &other.session)
            && self.panel == other.panel
            && self.views == other.views
    }
}
#[cfg(feature = "bevy-shell")]
impl Eq for ContainerSortingSnapshot {}
#[cfg(feature = "bevy-shell")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContainerSortingControl {
    Close,
    Edit(usize, String),
    Sort { confirmed: bool },
}
#[cfg(feature = "bevy-shell")]
impl PlayLink {
    pub fn container_sorting_snapshot(&self) -> ContainerSortingSnapshot {
        let mailbox = lock(&self.mailbox);
        ContainerSortingSnapshot {
            panel: mailbox.properties.container_sort.clone(),
            views: mailbox.containers.views.clone(),
            session: std::sync::Arc::clone(&self.mailbox),
        }
    }
    /// Validate and dispatch under the same owner lock; no local inventory mutation.
    pub fn container_sorting_dispatch_bound(
        &self,
        expected: &ContainerSortingSnapshot,
        control: ContainerSortingControl,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !std::sync::Arc::ptr_eq(&self.mailbox, &expected.session)
            || self.commands.is_closed()
            || mailbox.inventory_shutdown
            || mailbox.custom_hotkeys_ended
            || mailbox.server_modal.is_active()
            || mailbox.properties.container_sort != expected.panel
            || mailbox.containers.views != expected.views
            || !expected.panel.open
        {
            return false;
        }
        if control == ContainerSortingControl::Close {
            mailbox.properties.container_sort.open = false;
            return true;
        }
        if !expected.panel.selected || expected.panel.pending || mailbox.containers.pending {
            return false;
        }
        match control {
            ContainerSortingControl::Edit(index, value) => {
                if index >= 4 || value.len() > 7996 || value.contains('\0') {
                    return false;
                }
                super::input::edit(&mut mailbox.properties.container_sort, index, value);
                true
            }
            ContainerSortingControl::Sort { confirmed } => {
                if !confirmed {
                    return false;
                }
                let Ok(command) = super::protocol::command(
                    &expected.panel,
                    oteryn_client_ui::container_sort::Intent::Sort,
                ) else {
                    return false;
                };
                let Some(view) = command.payload.first() else {
                    return false;
                };
                if !expected.views.as_ref().is_some_and(|views| {
                    views
                        .views
                        .iter()
                        .any(|current| current.view_id == *view && current.browse.is_none())
                }) {
                    return false;
                }
                super::input::submit(
                    &mut mailbox,
                    &self.commands,
                    oteryn_client_ui::container_sort::Intent::Sort,
                );
                mailbox.properties.container_sort.pending
            }
            ContainerSortingControl::Close => false,
        }
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
impl PlayLink {
    pub(crate) fn container_sorting_test_initialize(
        &self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        tests::initialize(self)?;
        lock(&self.mailbox).properties.container_sort.summary = "Server sort summary".into();
        Ok(())
    }
}
