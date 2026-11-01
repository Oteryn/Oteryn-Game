pub use oteryn_client_runtime::player_trade::*;
use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};

/// Ctrl+T opens the direct-player-trade panel.
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false } if code.get() == 20 && modifiers.contains(Modifier::Control) && !modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Super)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Offer { partner: EntityRef, item: u64 },
    Accept,
    Cancel,
    LookOwn,
    LookCounter,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerTradePanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub state: Option<player_trade::State>,
    pub feedback: Option<&'static str>,
    /// Exact live handles supplied by world interaction for the Offer action.
    pub offer: Option<(EntityRef, u64)>,
}

pub fn request(panel: &mut PlayerTradePanel, intent: Intent) -> Option<PlayerTradeCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    let wire = match intent {
        Intent::Offer { partner, item } => player_trade::Intent::Offer { partner, item },
        Intent::Cancel => player_trade::Intent::Cancel,
        Intent::Accept | Intent::LookOwn | Intent::LookCounter => {
            let trade_id = panel.state.as_ref().and_then(|state| state.trade_id);
            let Some(trade_id) = trade_id else {
                panel.feedback = Some("No active trade");
                return None;
            };
            match intent {
                Intent::Accept => player_trade::Intent::Accept { trade_id },
                Intent::LookOwn => player_trade::Intent::Look {
                    trade_id,
                    counter_offer: false,
                },
                Intent::LookCounter => player_trade::Intent::Look {
                    trade_id,
                    counter_offer: true,
                },
                Intent::Offer { .. } | Intent::Cancel => return None,
            }
        }
    };
    let command = PlayerTradeCommand::new(wire)?;
    panel.pending = true;
    panel.feedback = None;
    Some(command)
}

pub fn apply_result(panel: &mut PlayerTradePanel, response: &PlayerTradeResponse) {
    panel.pending = false;
    panel.state = Some(response.state.clone());
    panel.feedback = match response.status {
        CommandStatus::Accepted | CommandStatus::DuplicateReplay => {
            let message = response.result.message();
            (!message.is_empty()).then_some(message)
        }
        _ => Some(response.result.message())
            .filter(|message| !message.is_empty())
            .or(Some("Sorry, not possible.")),
    };
}

#[must_use]
pub fn panel_text(panel: &PlayerTradePanel) -> String {
    if !panel.selected {
        return "Player trade unavailable in this session".into();
    }
    let Some(state) = &panel.state else {
        return panel.feedback.unwrap_or("No active trade").into();
    };
    let offer = |name: &str, value: Option<player_trade::Offer>| {
        value.map_or_else(
            || format!("{name}: none"),
            |value| {
                format!(
                    "{name}: item {} x{} (subtype {})",
                    value.definition, value.count, value.sub_type
                )
            },
        )
    };
    let phase = match state.phase {
        1 => "offered",
        2 => "ready",
        3 => "transferring",
        _ => "closed",
    };
    let mut lines = vec![
        format!("Trade: {phase}"),
        offer("Your offer", state.own),
        offer("Their offer", state.counter),
        format!(
            "Accepted: you={} partner={}",
            state.accepted[0], state.accepted[1]
        ),
    ];
    if !state.message.is_empty() {
        lines.push(state.message.clone());
    }
    if !state.look.is_empty() {
        lines.push(state.look.clone());
    }
    if let Some(feedback) = panel.feedback {
        lines.push(feedback.into());
    }
    lines.join("\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requests_all_wire_actions_and_applies_refusal() -> Result<(), &'static str> {
        let mut panel = PlayerTradePanel {
            selected: true,
            state: Some(player_trade::State {
                trade_id: Some([3; 16]),
                partner: Some(EntityRef {
                    identity: [4; 16],
                    generation: 1,
                }),
                own: Some(player_trade::Offer {
                    definition: 10,
                    count: 1,
                    sub_type: 0,
                }),
                phase: 1,
                reason: 0,
                ..Default::default()
            }),
            ..Default::default()
        };
        for intent in [
            Intent::Accept,
            Intent::Cancel,
            Intent::LookOwn,
            Intent::LookCounter,
        ] {
            assert!(request(&mut panel, intent).is_some());
            panel.pending = false;
        }
        assert!(
            request(
                &mut panel,
                Intent::Offer {
                    partner: EntityRef {
                        identity: [5; 16],
                        generation: 2
                    },
                    item: 7
                }
            )
            .is_some()
        );
        apply_result(
            &mut panel,
            &PlayerTradeResponse {
                status: CommandStatus::Accepted,
                result: player_trade::ResultCode::TooFar,
                state: player_trade::State::default(),
            },
        );
        if !panel_text(&panel).contains("Move closer") {
            return Err("refusal missing");
        }
        Ok(())
    }
}

mod input;
mod protocol;
mod state;
mod view;
