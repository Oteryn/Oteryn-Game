use oteryn_client_runtime::server_mechanic::{ServerMechanicCommand, ServerMechanicResponse};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServerMechanicPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub response: Vec<u8>,
    pub feedback: Option<String>,
}

pub fn request(
    panel: &mut ServerMechanicPanel,
    command: ServerMechanicCommand,
) -> Option<ServerMechanicCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    panel.pending = true;
    panel.feedback = None;
    Some(command)
}

pub fn apply_result(panel: &mut ServerMechanicPanel, result: &ServerMechanicResponse) {
    panel.pending = false;
    panel.response.clone_from(&result.payload);
    panel.feedback = (!matches!(
        result.status,
        oteryn_client_runtime::server_mechanic::CommandStatus::Accepted
            | oteryn_client_runtime::server_mechanic::CommandStatus::DuplicateReplay
    ))
    .then(|| format!("Server refused command: {:?}", result.status));
}

pub fn panel_text(panel: &ServerMechanicPanel, title: &str) -> String {
    if !panel.selected {
        return format!("{title} unavailable in this session");
    }
    let mut text = format!("{title} server response ({} bytes)", panel.response.len());
    if let Some(feedback) = &panel.feedback {
        text.push_str("\r\n");
        text.push_str(feedback);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_client_runtime::server_mechanic::CommandStatus;

    #[test]
    fn server_mechanic_gates_selection_and_one_pending_request() {
        let mut panel = ServerMechanicPanel::default();
        let command = ServerMechanicCommand {
            capability: 152,
            command_type: 181,
            payload: vec![1],
        };
        assert!(request(&mut panel, command.clone()).is_none());
        assert!(!panel.pending);
        panel.selected = true;
        assert_eq!(request(&mut panel, command.clone()), Some(command.clone()));
        assert!(request(&mut panel, command).is_none());
    }

    #[test]
    fn server_mechanic_replay_keeps_the_successful_result() {
        let mut panel = ServerMechanicPanel {
            selected: true,
            pending: true,
            ..Default::default()
        };
        let result = ServerMechanicResponse {
            command_id: 1,
            status: CommandStatus::DuplicateReplay,
            server_sequence: 2,
            payload: vec![3],
        };
        apply_result(&mut panel, &result);
        assert!(!panel.pending);
        assert_eq!(panel.response, [3]);
        assert!(panel.feedback.is_none());
        apply_result(
            &mut panel,
            &ServerMechanicResponse {
                status: CommandStatus::Rejected,
                ..result
            },
        );
        assert!(panel_text(&panel, "Forge").contains("refused"));
    }
}
