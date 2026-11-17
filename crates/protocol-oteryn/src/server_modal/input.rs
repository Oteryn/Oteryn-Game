//! Capability 490: an optional amount or text input on a server dialog, and the answer that
//! returns its value with a "don't show again" choice. Sessions without 490 keep the V1 bytes.
use super::{Answer, Dialog, MAX_REPLY_BYTES, Reply, WireResult, codec};

pub const CAPABILITY_SERVER_MODAL_INPUT_V1: u32 = 490;
/// The byte bound of a text input value and of a text limit.
pub const MAX_INPUT_TEXT_BYTES: u32 = 1024;
/// The largest command 737 payload with 490 selected: a V1 answer with a choice (7 bytes),
/// a text value (`tag:u8, len:u16le` and the text) and the "don't show again" byte.
pub const MAX_INPUT_ANSWER_BYTES: usize = 7 + 1 + 2 + MAX_INPUT_TEXT_BYTES as usize + 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InputKind {
    None = 0,
    /// A whole number between the limits.
    Amount = 1,
    /// One line of text whose byte length lies between the limits.
    Text = 2,
    /// Multi-line text whose byte length lies between the limits.
    Edit = 3,
}
impl InputKind {
    pub fn from_id(id: u8) -> Option<Self> {
        [Self::None, Self::Amount, Self::Text, Self::Edit]
            .into_iter()
            .find(|kind| *kind as u8 == id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputValue {
    Amount(u64),
    Text(String),
}

/// The input a dialog asks for. `minimum` and `maximum` bound the amount, or the text's byte
/// length; `initial` prefills the field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogInput {
    pub kind: InputKind,
    pub minimum: u32,
    pub maximum: u32,
    pub initial: Option<InputValue>,
}

fn text_fits(kind: InputKind, text: &str, maximum: u32) -> bool {
    u32::try_from(text.len()).is_ok_and(|length| length <= maximum)
        && !text.contains('\0')
        && (kind == InputKind::Edit || !text.contains(['\n', '\r']))
}

impl DialogInput {
    pub fn valid(&self) -> bool {
        if self.minimum > self.maximum {
            return false;
        }
        match (self.kind, &self.initial) {
            (InputKind::None, _) => false,
            (InputKind::Amount, None) => true,
            (InputKind::Amount, Some(InputValue::Amount(value))) => self.amount_fits(*value),
            (InputKind::Text | InputKind::Edit, None) => self.maximum <= MAX_INPUT_TEXT_BYTES,
            (InputKind::Text | InputKind::Edit, Some(InputValue::Text(text))) => {
                self.maximum <= MAX_INPUT_TEXT_BYTES && text_fits(self.kind, text, self.maximum)
            }
            _ => false,
        }
    }
    fn amount_fits(&self, value: u64) -> bool {
        (u64::from(self.minimum)..=u64::from(self.maximum)).contains(&value)
    }
    /// Whether `value` is an acceptable entry for this input.
    pub fn accepts(&self, value: &InputValue) -> bool {
        self.valid()
            && match (self.kind, value) {
                (InputKind::Amount, InputValue::Amount(amount)) => self.amount_fits(*amount),
                (InputKind::Text | InputKind::Edit, InputValue::Text(text)) => {
                    u32::try_from(text.len()).is_ok_and(|length| length >= self.minimum)
                        && text_fits(self.kind, text, self.maximum)
                }
                _ => false,
            }
    }
}

fn push_value(out: &mut Vec<u8>, value: Option<&InputValue>) -> WireResult<()> {
    match value {
        None => out.push(0),
        Some(InputValue::Amount(amount)) => {
            out.push(1);
            out.extend_from_slice(&amount.to_le_bytes());
        }
        Some(InputValue::Text(text)) => {
            if !u32::try_from(text.len()).is_ok_and(|length| length <= MAX_INPUT_TEXT_BYTES) {
                return Err("modal input text exceeds its bound");
            }
            out.push(2);
            let length = u16::try_from(text.len()).map_err(|_| "modal input text")?;
            out.extend_from_slice(&length.to_le_bytes());
            out.extend_from_slice(text.as_bytes());
        }
    }
    Ok(())
}

/// The value and the bytes after it.
fn read_value(bytes: &[u8]) -> WireResult<(Option<InputValue>, &[u8])> {
    let (&tag, rest) = bytes.split_first().ok_or("truncated modal input")?;
    match tag {
        0 => Ok((None, rest)),
        1 => {
            let (amount, rest) = rest.split_at_checked(8).ok_or("truncated modal amount")?;
            let amount = u64::from_le_bytes(amount.try_into().map_err(|_| "modal amount")?);
            Ok((Some(InputValue::Amount(amount)), rest))
        }
        2 => {
            let (length, rest) = rest.split_at_checked(2).ok_or("truncated modal text")?;
            let length = u16::from_le_bytes(length.try_into().map_err(|_| "modal text")?);
            if u32::from(length) > MAX_INPUT_TEXT_BYTES {
                return Err("modal input text exceeds its bound");
            }
            let (text, rest) = rest
                .split_at_checked(usize::from(length))
                .ok_or("truncated modal text")?;
            let text = std::str::from_utf8(text).map_err(|_| "invalid modal UTF-8")?;
            Ok((Some(InputValue::Text(text.to_owned())), rest))
        }
        _ => Err("invalid modal input value"),
    }
}

/// `kind:u8`, then for an input `minimum:u32le, maximum:u32le` and the initial value
/// (`0` none, `1` and `amount:u64le`, `2` and `len:u16le` with the text).
fn push_input(out: &mut Vec<u8>, input: Option<&DialogInput>) -> WireResult<()> {
    let Some(input) = input else {
        out.push(InputKind::None as u8);
        return Ok(());
    };
    if !input.valid() {
        return Err("invalid modal input");
    }
    out.push(input.kind as u8);
    out.extend_from_slice(&input.minimum.to_le_bytes());
    out.extend_from_slice(&input.maximum.to_le_bytes());
    push_value(out, input.initial.as_ref())
}

/// A dialog reply for a session that selected 490: the V1 dialog followed by its input block.
pub fn encode_dialog_with_input(
    dialog: &Dialog,
    input: Option<&DialogInput>,
) -> WireResult<Vec<u8>> {
    let mut out = Reply::Dialog(dialog.clone()).encode()?;
    push_input(&mut out, input)?;
    if out.len() > MAX_REPLY_BYTES {
        return Err("modal reply exceeds its bound");
    }
    Ok(out)
}

/// A modal reply as a session that selected 490 receives it: a dialog carries its input block,
/// every other reply is unchanged.
pub fn decode_reply_with_input(bytes: &[u8]) -> WireResult<(Reply, Option<DialogInput>)> {
    let (reply, rest) = codec::decode_prefix(bytes)?;
    if !matches!(reply, Reply::Dialog(_)) {
        return if rest.is_empty() {
            Ok((reply, None))
        } else {
            Err("trailing modal bytes")
        };
    }
    let (&kind, rest) = rest.split_first().ok_or("truncated modal input")?;
    let kind = InputKind::from_id(kind).ok_or("invalid modal input kind")?;
    if kind == InputKind::None {
        return if rest.is_empty() {
            Ok((reply, None))
        } else {
            Err("trailing modal bytes")
        };
    }
    let (limits, rest) = rest.split_at_checked(8).ok_or("truncated modal input")?;
    let minimum = u32::from_le_bytes(limits[..4].try_into().map_err(|_| "modal minimum")?);
    let maximum = u32::from_le_bytes(limits[4..].try_into().map_err(|_| "modal maximum")?);
    let (initial, rest) = read_value(rest)?;
    if !rest.is_empty() {
        return Err("trailing modal bytes");
    }
    let input = DialogInput {
        kind,
        minimum,
        maximum,
        initial,
    };
    if !input.valid() {
        return Err("invalid modal input");
    }
    Ok((reply, Some(input)))
}

/// Command 737 from a session that selected 490: the V1 answer, the entered value and the
/// "don't show again" choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputAnswer {
    pub answer: Answer,
    pub value: Option<InputValue>,
    pub dont_show_again: bool,
}
impl InputAnswer {
    /// The V1 answer bytes, `value` (as in the dialog input block) and `dont_show_again:u8`.
    pub fn encode(&self) -> WireResult<Vec<u8>> {
        let mut out = self.answer.encode()?;
        push_value(&mut out, self.value.as_ref())?;
        out.push(u8::from(self.dont_show_again));
        Ok(out)
    }
    pub fn decode(bytes: &[u8]) -> WireResult<Self> {
        if bytes.len() > MAX_INPUT_ANSWER_BYTES {
            return Err("modal answer exceeds its bound");
        }
        let answer_length = match bytes.get(5) {
            Some(0) => 6,
            Some(1) => 7,
            _ => return Err("invalid modal choice"),
        };
        let (answer, rest) = bytes
            .split_at_checked(answer_length)
            .ok_or("truncated modal answer")?;
        let answer = Answer::decode(answer)?;
        let (value, rest) = read_value(rest)?;
        let dont_show_again = match rest {
            [0] => false,
            [1] => true,
            _ => return Err("invalid modal answer flags"),
        };
        Ok(Self {
            answer,
            value,
            dont_show_again,
        })
    }
}

/// Command 737 as the session's selection shapes it: without 490 the V1 answer, with no value.
pub fn decode_answer_selected(input: bool, bytes: &[u8]) -> WireResult<InputAnswer> {
    if input {
        return InputAnswer::decode(bytes);
    }
    Answer::decode(bytes).map(|answer| InputAnswer {
        answer,
        value: None,
        dont_show_again: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn dialog() -> Dialog {
        Dialog {
            id: 5,
            title: "Amount".into(),
            message: "How many?".into(),
            choices: Vec::new(),
            buttons: vec![(1, "Ok".into()), (2, "Cancel".into())],
            cancel_button: Some(2),
            image: None,
        }
    }

    #[test]
    fn dialog_input_block_follows_the_v1_dialog_only_when_selected() -> WireResult<()> {
        let value = dialog();
        let plain = Reply::Dialog(value.clone()).encode()?;
        let none = encode_dialog_with_input(&value, None)?;
        assert_eq!(none[..plain.len()], plain[..]);
        assert_eq!(none[plain.len()..], [0]);
        assert_eq!(
            decode_reply_with_input(&none)?,
            (Reply::Dialog(value.clone()), None)
        );
        let amount = DialogInput {
            kind: InputKind::Amount,
            minimum: 1,
            maximum: 100,
            initial: Some(InputValue::Amount(10)),
        };
        let encoded = encode_dialog_with_input(&value, Some(&amount))?;
        let mut tail = vec![1, 1, 0, 0, 0, 100, 0, 0, 0, 1];
        tail.extend(10_u64.to_le_bytes());
        assert_eq!(encoded[plain.len()..], tail[..]);
        assert_eq!(
            decode_reply_with_input(&encoded)?,
            (Reply::Dialog(value.clone()), Some(amount.clone()))
        );
        let text = DialogInput {
            kind: InputKind::Edit,
            minimum: 0,
            maximum: MAX_INPUT_TEXT_BYTES,
            initial: Some(InputValue::Text("line one\nline two".into())),
        };
        let encoded = encode_dialog_with_input(&value, Some(&text))?;
        assert_eq!(
            decode_reply_with_input(&encoded)?,
            (Reply::Dialog(value.clone()), Some(text))
        );
        assert!(
            Reply::decode(&encoded).is_err(),
            "a V1 decoder never sees the block"
        );
        assert!(decode_reply_with_input(&plain).is_err());
        let mut trailing = encoded;
        trailing.push(0);
        assert!(decode_reply_with_input(&trailing).is_err());
        let answered = Reply::Answered(Answer {
            dialog_id: 5,
            button_id: 1,
            choice_id: None,
        });
        assert_eq!(
            decode_reply_with_input(&answered.encode()?)?,
            (answered, None),
            "answer replies are unchanged"
        );
        for invalid in [
            DialogInput {
                kind: InputKind::None,
                minimum: 0,
                maximum: 0,
                initial: None,
            },
            DialogInput {
                minimum: 101,
                ..amount.clone()
            },
            DialogInput {
                initial: Some(InputValue::Amount(101)),
                ..amount.clone()
            },
            DialogInput {
                initial: Some(InputValue::Text("10".into())),
                ..amount.clone()
            },
            DialogInput {
                kind: InputKind::Text,
                minimum: 0,
                maximum: MAX_INPUT_TEXT_BYTES + 1,
                initial: None,
            },
            DialogInput {
                kind: InputKind::Text,
                minimum: 0,
                maximum: 10,
                initial: Some(InputValue::Text("two\nlines".into())),
            },
        ] {
            assert!(encode_dialog_with_input(&value, Some(&invalid)).is_err());
        }
        Ok(())
    }

    #[test]
    fn input_answer_carries_the_value_and_dont_show_again_exactly() -> WireResult<()> {
        let base = Answer {
            dialog_id: 5,
            button_id: 1,
            choice_id: None,
        };
        let amount = InputAnswer {
            answer: base,
            value: Some(InputValue::Amount(42)),
            dont_show_again: true,
        };
        let encoded = amount.encode()?;
        let mut expected = base.encode()?;
        expected.push(1);
        expected.extend(42_u64.to_le_bytes());
        expected.push(1);
        assert_eq!(encoded, expected);
        assert_eq!(decode_answer_selected(true, &encoded)?, amount);
        assert!(decode_answer_selected(false, &encoded).is_err());
        let v1 = base.encode()?;
        assert_eq!(
            decode_answer_selected(false, &v1)?,
            InputAnswer {
                answer: base,
                value: None,
                dont_show_again: false,
            }
        );
        assert!(decode_answer_selected(true, &v1).is_err());
        let longest = InputAnswer {
            answer: Answer {
                choice_id: Some(3),
                ..base
            },
            value: Some(InputValue::Text("t".repeat(MAX_INPUT_TEXT_BYTES as usize))),
            dont_show_again: false,
        };
        let encoded = longest.encode()?;
        assert_eq!(encoded.len(), MAX_INPUT_ANSWER_BYTES);
        assert_eq!(InputAnswer::decode(&encoded)?, longest);
        let mut flag = encoded.clone();
        if let Some(last) = flag.last_mut() {
            *last = 2;
        }
        assert!(InputAnswer::decode(&flag).is_err());
        let too_long = InputAnswer {
            value: Some(InputValue::Text(
                "t".repeat(MAX_INPUT_TEXT_BYTES as usize + 1),
            )),
            ..longest
        };
        assert!(too_long.encode().is_err());
        let input = DialogInput {
            kind: InputKind::Text,
            minimum: 2,
            maximum: 4,
            initial: None,
        };
        assert!(input.accepts(&InputValue::Text("abc".into())));
        assert!(!input.accepts(&InputValue::Text("a".into())));
        assert!(!input.accepts(&InputValue::Text("abcde".into())));
        assert!(!input.accepts(&InputValue::Text("a\nb".into())));
        assert!(!input.accepts(&InputValue::Amount(3)));
        Ok(())
    }

    #[test]
    fn largest_dialog_with_input_fits_the_existing_command_result_budget() -> WireResult<()> {
        let mut value = dialog();
        value.title = "t".repeat(256);
        value.message = "m".repeat(8192);
        value.choices = (0..super::super::MAX_CHOICES)
            .map(|id| (u8::try_from(id).unwrap_or(0), "c".repeat(256)))
            .collect();
        value.buttons = (0..super::super::MAX_BUTTONS)
            .map(|id| (u8::try_from(id).unwrap_or(0), "b".repeat(256)))
            .collect();
        value.cancel_button = Some(1);
        let edge = super::super::MAX_IMAGE_EDGE;
        value.image = Some(super::super::Image {
            width: edge,
            height: edge,
            rgba: vec![255; usize::from(edge) * usize::from(edge) * 4],
        });
        let input = DialogInput {
            kind: InputKind::Edit,
            minimum: 0,
            maximum: MAX_INPUT_TEXT_BYTES,
            initial: Some(InputValue::Text("e".repeat(MAX_INPUT_TEXT_BYTES as usize))),
        };
        let encoded = encode_dialog_with_input(&value, Some(&input))?;
        assert!(encoded.len() <= MAX_REPLY_BYTES);
        assert_eq!(
            decode_reply_with_input(&encoded)?,
            (Reply::Dialog(value), Some(input))
        );
        Ok(())
    }
}
