pub use oteryn_client_runtime::vip::{VipCommand, VipResponse, vip};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    Add(String),
    Remove(u64),
    Edit {
        number: u64,
        text: String,
        icon: u8,
        notify: bool,
        groups: Vec<u8>,
    },
    GroupAdd(String),
    GroupRename(u64, String),
    GroupRemove(u64),
    Accept(u64),
    Reject(u64),
    Revoke(u64),
    Withdraw(u64),
    Status(u64),
    Query(u64),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VipPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub revision: u64,
    pub view: Option<String>,
    pub feedback: Option<String>,
}

pub fn request(panel: &mut VipPanel, intent: Intent) -> Option<VipCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    let (operation, number, text, icon, notify, groups) = match intent {
        Intent::Add(text) => (vip::Operation::Add, 0, text, 0, false, vec![]),
        Intent::Remove(n) => (vip::Operation::Remove, n, String::new(), 0, false, vec![]),
        Intent::Edit {
            number,
            text,
            icon,
            notify,
            groups,
        } => (vip::Operation::Edit, number, text, icon, notify, groups),
        Intent::GroupAdd(text) => (vip::Operation::GroupAdd, 0, text, 0, false, vec![]),
        Intent::GroupRename(n, text) => (vip::Operation::GroupRename, n, text, 0, false, vec![]),
        Intent::GroupRemove(n) => (
            vip::Operation::GroupRemove,
            n,
            String::new(),
            0,
            false,
            vec![],
        ),
        Intent::Accept(n) => (vip::Operation::Accept, n, String::new(), 0, false, vec![]),
        Intent::Reject(n) => (vip::Operation::Reject, n, String::new(), 0, false, vec![]),
        Intent::Revoke(n) => (vip::Operation::Revoke, n, String::new(), 0, false, vec![]),
        Intent::Withdraw(n) => (vip::Operation::Withdraw, n, String::new(), 0, false, vec![]),
        Intent::Status(n) => (vip::Operation::Status, n, String::new(), 0, false, vec![]),
        Intent::Query(n) => (vip::Operation::Query, n, String::new(), 0, false, vec![]),
    };
    let command = VipCommand::new(&vip::Request {
        operation,
        number,
        text,
        icon,
        notify,
        groups,
    });
    if command.is_some() {
        panel.pending = true;
        panel.feedback = None;
    } else {
        panel.feedback = Some("Invalid VIP request".into());
    }
    command
}

pub fn apply_result(panel: &mut VipPanel, response: &VipResponse) {
    panel.pending = false;
    panel.revision = response.result.revision;
    if !response.result.view.is_empty() {
        panel.view = Some(response.result.view.clone());
    }
    panel.feedback = (response.result.code != 1).then(|| {
        match response.result.code {
            2 => "Unknown character name",
            3 => "Character is already listed",
            4 => "VIP list is full",
            5 => "VIP group limit reached",
            6 => "That group name is taken",
            7 => "VIP entry was not found",
            8 => "VIP request rate limited",
            9 => "VIP service unavailable",
            _ => "VIP request rejected",
        }
        .to_owned()
    });
}

#[must_use]
pub fn panel_text(panel: &VipPanel) -> String {
    if !panel.selected {
        return "VIP unavailable in this session".into();
    }
    let mut text = panel
        .view
        .clone()
        .unwrap_or_else(|| "Loading VIP list…".into());
    if let Some(feedback) = &panel.feedback {
        text.push_str("\r\n");
        text.push_str(feedback);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_client_runtime::vip::CommandStatus;
    #[test]
    fn requests_every_operation_and_displays_refusals() {
        let mut panel = VipPanel {
            selected: true,
            ..VipPanel::default()
        };
        let intents = [
            Intent::Add("A".into()),
            Intent::Remove(1),
            Intent::GroupAdd("G".into()),
            Intent::GroupRename(1, "H".into()),
            Intent::GroupRemove(1),
            Intent::Accept(1),
            Intent::Reject(1),
            Intent::Revoke(1),
            Intent::Withdraw(1),
            Intent::Status(1),
            Intent::Query(0),
        ];
        for intent in intents {
            assert!(request(&mut panel, intent).is_some());
            panel.pending = false;
        }
        assert!(
            request(
                &mut panel,
                Intent::Edit {
                    number: 1,
                    text: "friend".into(),
                    icon: 2,
                    notify: true,
                    groups: vec![1]
                }
            )
            .is_some()
        );
        apply_result(
            &mut panel,
            &VipResponse {
                status: CommandStatus::Accepted,
                result: vip::ResultView {
                    code: 4,
                    revision: 3,
                    number: 0,
                    wait: 0,
                    view: "{\"entries\":[]}".into(),
                },
            },
        );
        assert!(panel_text(&panel).contains("VIP list is full"));
        assert_eq!(panel.revision, 3);
    }
}
