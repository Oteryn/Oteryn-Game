#[path = "input/catalogue.rs"]
pub(in crate::play) mod catalogue;
use crate::play::{PlayLink, mailbox::lock};
impl PlayLink {
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

#[cfg(all(test, feature = "bevy-shell"))]
#[path = "tests/bevy_catalogue.rs"]
mod bevy_catalogue_tests;
