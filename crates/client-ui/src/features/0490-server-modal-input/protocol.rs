//! Capability 490 `SERVER_MODAL_INPUT_V1`: a dialog may ask for an amount or text and its answer
//! returns the value with a "don't show again" choice. Without it the V1 dialog and answer stay.
use oteryn_client_runtime::server_modal::{
    Answer, Reply,
    input::{DialogInput, InputAnswer, InputKind, InputValue, decode_reply_with_input},
};

/// What the player entered next to the chosen button.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Entry {
    pub value: Option<InputValue>,
    pub dont_show_again: bool,
}

/// A 736 reply: with 490 selected a dialog carries its input block, else the V1 bytes.
pub fn decode_reply(
    selected: bool,
    payload: &[u8],
) -> Result<(Reply, Option<DialogInput>), &'static str> {
    if selected {
        return decode_reply_with_input(payload);
    }
    Reply::decode(payload).map(|reply| (reply, None))
}

/// Read what the player typed as the value the input asks for. `None` when it is not a number
/// (amount) or violates the limits.
#[must_use]
pub fn parse_value(input: &DialogInput, text: &str) -> Option<InputValue> {
    let value = match input.kind {
        InputKind::None => return None,
        InputKind::Amount => InputValue::Amount(text.trim().parse().ok()?),
        InputKind::Text | InputKind::Edit => InputValue::Text(text.to_owned()),
    };
    input.accepts(&value).then_some(value)
}

/// The prompt line above the input field.
#[must_use]
pub fn prompt(input: &DialogInput) -> String {
    match input.kind {
        InputKind::None => String::new(),
        InputKind::Amount => format!("Amount ({} to {})", input.minimum, input.maximum),
        InputKind::Text | InputKind::Edit => {
            format!("Text ({} to {} bytes)", input.minimum, input.maximum)
        }
    }
}

/// The text the field starts with.
#[must_use]
pub fn initial_text(input: &DialogInput) -> String {
    match &input.initial {
        Some(InputValue::Amount(amount)) => amount.to_string(),
        Some(InputValue::Text(text)) => text.clone(),
        None => String::new(),
    }
}

/// The command 737 payload. Without 490 it is the V1 answer; with it the value tag and the
/// "don't show again" byte follow. `None` when the dialog asks for an input and the answer, other
/// than its cancel button, carries no acceptable value.
#[must_use]
pub fn answer_payload(
    selected: bool,
    input: Option<&DialogInput>,
    cancel: bool,
    answer: Answer,
    entry: &Entry,
) -> Option<Vec<u8>> {
    if !selected {
        return answer.encode().ok();
    }
    let value = match (input, cancel) {
        (Some(input), false) => Some(entry.value.clone().filter(|value| input.accepts(value))?),
        _ => None,
    };
    InputAnswer {
        answer,
        value,
        dont_show_again: entry.dont_show_again && !cancel,
    }
    .encode()
    .ok()
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
