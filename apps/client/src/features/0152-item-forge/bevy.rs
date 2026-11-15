//! Snapshot-fenced forwarding to the retained Forge controller; no new wire operations.
use super::super::input::controller::{self, Action};
use crate::{
    combat_input::PlayCommand,
    play::{PlayLink, mailbox::lock},
};
use oteryn_client_runtime::{cui3::item_forge::Operation, forge_preview as wire};
use oteryn_client_ui::item_forge::ItemForgePanel as Panel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ForgeControl {
    Close,
    Operation(Operation),
    Convergence,
    First(u64),
    Second(u64),
    Boost,
    Protect,
    Quote,
    Submit,
    Refresh,
    Next,
    Convert(Operation),
    Confirm(u64),
    Dismiss(u64),
}

pub(crate) fn forge_action_enabled(panel: &Panel) -> bool {
    if !panel.open
        || !panel.editing()
        || panel.preview.pending.is_some()
        || !panel.verified_quote()
        || panel.first_item == panel.second_item
    {
        return false;
    }
    let Some(quote) = &panel.preview.quote else {
        return false;
    };
    let gold = quote
        .resources
        .carried_gold
        .saturating_add(if quote.resources.bank_spendable {
            quote.resources.bank_gold
        } else {
            0
        });
    gold >= quote.cost.gold
        && quote.resources.dust >= quote.cost.dust
        && quote.resources.cores >= u64::from(quote.cost.cores)
        && [panel.first_item, panel.second_item]
            .into_iter()
            .all(|handle| {
                panel
                    .preview
                    .item(handle)
                    .is_some_and(|item| !item.container)
            })
}

impl PlayLink {
    pub(crate) fn forge_dispatch_bound(&self, expected: &Panel, control: ForgeControl) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.item_forge;
        if panel != expected || !panel.open {
            return false;
        }
        if control == ForgeControl::Close {
            panel.open = false;
            return true;
        }
        if !panel.selected || panel.pending {
            return false;
        }
        let action = match control {
            ForgeControl::Close => return false,
            ForgeControl::Operation(operation) => {
                if !panel.editing() || !operation.requires_pair() {
                    return false;
                }
                panel.first_item = 0;
                panel.second_item = 0;
                Action::Operation(operation)
            }
            ForgeControl::Convergence => {
                if !panel.editing() {
                    return false;
                }
                let operation = match panel.operation {
                    Operation::Fusion => Operation::ConvergenceFusion,
                    Operation::ConvergenceFusion => Operation::Fusion,
                    Operation::Transfer => Operation::ConvergenceTransfer,
                    Operation::ConvergenceTransfer => Operation::Transfer,
                    _ => return false,
                };
                panel.first_item = 0;
                panel.second_item = 0;
                // Convergence cannot retain normal-fusion core options.
                panel.boost = false;
                panel.protect = false;
                Action::Operation(operation)
            }
            ForgeControl::First(id) => Action::First(id),
            ForgeControl::Second(id) => Action::Second(id),
            ForgeControl::Boost => Action::Cores {
                boost: !panel.boost,
                protect: panel.protect,
            },
            ForgeControl::Protect => Action::Cores {
                boost: panel.boost,
                protect: !panel.protect,
            },
            ForgeControl::Quote => Action::Quote,
            ForgeControl::Submit => {
                if !forge_action_enabled(panel) {
                    return false;
                }
                Action::Forge
            }
            ForgeControl::Refresh => Action::Refresh,
            ForgeControl::Next => Action::Next,
            ForgeControl::Convert(operation) => Action::Legacy(operation),
            ForgeControl::Confirm(serial) => Action::ConfirmResource(serial),
            ForgeControl::Dismiss(serial) => Action::DismissResource(serial),
        };
        controller::activate(&mut mailbox, &self.commands, action);
        true
    }

    /// Refresh only the read model after a deadline. Preserve the unresolved paid occurrence,
    /// pending flag and submitted request so late results still reconcile with their attempt.
    pub(crate) fn forge_timeout_refresh(&self, expected: &Panel) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.item_forge;
        if panel != expected || !panel.open || !panel.selected || !panel.pending {
            return false;
        }
        panel.preview.invalidate();
        panel.preview.refresh = false;
        panel.preview.items.clear();
        panel.preview.page = None;
        panel.feedback = Some("No result after 5 seconds. Refreshing current items; the original attempt is still pending.".into());
        if !panel.preview.selected {
            return true;
        }
        let request = wire::Request {
            nonce: panel.preview.nonce,
            selection: wire::Selection::Browse { after: 0 },
        };
        let Some(payload) = request.encode() else {
            return false;
        };
        panel.preview.pending = Some(request);
        if self
            .commands
            .send(PlayCommand::Feature(
                oteryn_client_runtime::raw_feature::FeatureCommand {
                    capability: wire::CAPABILITY_FORGE_PREVIEW_V1,
                    kind: wire::COMMAND_TYPE_FORGE_PREVIEW,
                    payload,
                },
            ))
            .is_err()
        {
            panel.preview.pending = None;
            panel.preview.feedback = Some("The session has ended".into());
        }
        true
    }
}

#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_forge_test_panel(&self, panel: Panel) {
        lock(&self.0).item_forge = panel;
    }
    pub(crate) fn bevy_forge_test_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        self.item_forge_result(response);
    }
}
