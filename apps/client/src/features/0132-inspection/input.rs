#[path = "input/catalogue.rs"]
pub(in crate::play) mod catalogue;
#[path = "input/related.rs"]
mod related;
use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
    pub(in crate::play) fn sync_inspection_permission(
        &self,
        panel: &mut oteryn_client_ui::inspection::Panel,
        allow: bool,
    ) {
        use oteryn_client_ui::inspection as wire;
        if !panel.selected
            || panel.pending
            || (panel.allow_inspection == allow && panel.permission_retry_at.is_none())
            || panel
                .permission_retry_at
                .is_some_and(|(previous, at)| previous == allow && at > std::time::Instant::now())
        {
            return;
        }
        let request = wire::Request::Permission {
            action: if allow { 4 } else { 5 },
            target: None,
        };
        let Ok(payload) = wire::encode_request(request) else {
            return;
        };
        let command = oteryn_client_runtime::server_mechanic::ServerMechanicCommand {
            capability: wire::CAPABILITY_INSPECTION_V1,
            command_type: wire::COMMAND_TYPE_INSPECTION,
            payload,
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::ServerMechanic(command))
            .is_ok()
        {
            panel.permission_pending = Some(allow);
            panel.pending = true;
        } else {
            panel.feedback = tr!("Session ended").into();
            panel.permission_retry_at = Some((
                allow,
                std::time::Instant::now() + std::time::Duration::from_secs(5),
            ));
        }
    }
    pub fn inspect(&self, request: oteryn_client_ui::inspection::Request) -> bool {
        let command = oteryn_client_ui::inspection::request(
            &mut lock(&self.mailbox).discovery.inspection,
            request,
        );
        command.is_some_and(|command| self.discovery_send(command))
    }
    pub fn inspect_selected_item(&self) -> bool {
        self.inventory_panel().selected_item().is_some_and(|item| {
            self.inspect(oteryn_client_ui::inspection::Request::Item(item.handle))
        })
    }
}

impl PlayLink {
    pub fn inspection_control(&self, action: usize, fields: &[String], row: usize) -> bool {
        use oteryn_client_ui::inspection::Request;
        let panel = self.discovery_panel().inspection;
        match action {
            1 => self.inspect_selected_item(),
            2 => self.inspect(Request::Permission {
                action: 6,
                target: None,
            }),
            3 | 4 => panel
                .reply
                .and_then(|reply| reply.requests.get(row).copied())
                .or(match panel.last_request {
                    Some(Request::Permission { target, .. }) => target,
                    Some(Request::Player(target)) => Some(target),
                    _ => None,
                })
                .is_some_and(|target| {
                    self.inspect(Request::Permission {
                        action: if action == 3 { 2 } else { 3 },
                        target: Some(target),
                    })
                }),
            5 | 6 => self.inspect(Request::Permission {
                action: if action == 5 { 4 } else { 5 },
                target: None,
            }),
            7 => fields
                .first()
                .and_then(|value| value.parse().ok())
                .is_some_and(|id| self.inspect(Request::Catalogue(id))),
            8 => panel
                .last_request
                .is_some_and(|request| self.inspect(request)),
            9 => fields
                .get(1)
                .and_then(|value| value.parse::<u8>().ok())
                .zip(fields.get(2).and_then(|value| match value.as_str() {
                    "0" => Some(false),
                    "1" => Some(true),
                    _ => None,
                }))
                .is_some_and(|(index, counter)| self.inspect(Request::Trade { counter, index })),
            _ => false,
        }
    }
}

#[cfg(test)]
#[path = "tests/catalogue.rs"]
mod catalogue_tests;
