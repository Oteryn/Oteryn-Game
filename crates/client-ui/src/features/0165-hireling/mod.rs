pub use oteryn_client_runtime::hireling::{
    CommandStatus, HirelingCommand, HirelingResponse, hireling,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Refresh,
    Summon,
    Dismiss,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HirelingPanel {
    pub selected: bool,
    pub open: bool,
    pub active: Option<bool>,
    pub target: Option<(u16, u16, i8)>,
    pub pending: Option<HirelingCommand>,
    pub feedback: Option<&'static str>,
}

pub fn request(panel: &mut HirelingPanel, intent: Intent) -> Option<HirelingCommand> {
    let result = if !panel.selected {
        Err("Hirelings are unavailable in this session")
    } else if panel.pending.is_some() {
        Err("Waiting for the server")
    } else {
        match intent {
            Intent::Refresh => Ok(HirelingCommand(hireling::Request::Query)),
            Intent::Dismiss => Ok(HirelingCommand(hireling::Request::Dismiss)),
            Intent::Summon => panel.target.map_or(
                Err("Select an adjacent summon location"),
                |(x, y, floor)| Ok(HirelingCommand(hireling::Request::Summon { x, y, floor })),
            ),
        }
    };
    match result {
        Ok(command) => {
            panel.pending = Some(command);
            panel.feedback = None;
            Some(command)
        }
        Err(message) => {
            panel.feedback = Some(message);
            None
        }
    }
}

pub fn apply_result(panel: &mut HirelingPanel, response: &HirelingResponse) -> bool {
    if panel.pending.take().is_none() {
        return false;
    }
    match (response.status, response.active) {
        (CommandStatus::Accepted, Some(active))
        | (CommandStatus::DuplicateReplay, Some(active)) => {
            panel.active = Some(active);
            panel.feedback = Some("The server confirmed the hireling status");
            true
        }
        (CommandStatus::Rejected, _) => {
            panel.feedback = Some("The server refused the hireling request");
            false
        }
        _ => {
            panel.feedback = Some("The server returned an invalid hireling response");
            false
        }
    }
}

pub fn panel_text(panel: &HirelingPanel) -> String {
    let status = match panel.active {
        Some(true) => "Banker hireling: summoned",
        Some(false) => "Banker hireling: dismissed",
        None => "Banker hireling: status unknown",
    };
    let mut text = status.to_owned();
    if panel.pending.is_some() {
        text.push_str("\r\nWaiting for the server…");
    }
    if let Some(feedback) = panel.feedback {
        text.push_str("\r\n");
        text.push_str(feedback);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_use_the_codec_and_refusals_are_visible() {
        let mut panel = HirelingPanel {
            selected: true,
            target: Some((7, 9, -1)),
            ..Default::default()
        };
        let Some(command) = request(&mut panel, Intent::Summon) else {
            return;
        };
        assert_eq!(
            hireling::Request::decode(&command.0.encode()),
            Some(command.0)
        );
        assert!(!apply_result(
            &mut panel,
            &HirelingResponse {
                status: CommandStatus::Rejected,
                active: None
            }
        ));
        assert!(panel_text(&panel).contains("refused"));
    }

    #[test]
    fn accepted_status_updates_plain_text() {
        let mut panel = HirelingPanel {
            selected: true,
            ..Default::default()
        };
        assert!(request(&mut panel, Intent::Refresh).is_some());
        assert!(apply_result(
            &mut panel,
            &HirelingResponse {
                status: CommandStatus::Accepted,
                active: Some(true)
            }
        ));
        assert!(panel_text(&panel).contains("summoned"));
    }
}
