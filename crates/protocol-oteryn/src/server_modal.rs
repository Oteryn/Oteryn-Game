//! Generic server-owned choices, buttons and bounded inline image data.
pub const CAPABILITY_SERVER_MODAL_V1: u32 = 450;
pub const COMMAND_TYPE_SERVER_MODAL_QUERY: u32 = 736;
pub const COMMAND_TYPE_SERVER_MODAL_ANSWER: u32 = 737;
pub const COMMAND_TYPE_HIRELING_STORE_SELECTION: u32 = 6196;
/// Opt in to the typed terminal acknowledgement on this physical connection.
/// The empty legacy query retains the original reply vocabulary.
pub const QUERY_WITH_TERMINAL_REPLY: &[u8] = &[1];
pub const MAX_REPLY_BYTES: usize = crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES;
pub const MAX_CHOICES: usize = 32;
pub const MAX_BUTTONS: usize = 8;
pub const MAX_IMAGE_EDGE: u16 = 96;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u16,
    pub height: u16,
    pub rgba: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dialog {
    pub id: u32,
    pub title: String,
    pub message: String,
    pub choices: Vec<(u8, String)>,
    pub buttons: Vec<(u8, String)>,
    pub cancel_button: Option<u8>,
    pub image: Option<Image>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Answer {
    pub dialog_id: u32,
    pub button_id: u8,
    pub choice_id: Option<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    None,
    Dialog(Dialog),
    Answered(Answer),
    AnsweredAndLoggedOut(Answer),
}
type WireResult<T> = std::result::Result<T, &'static str>;
pub fn decode_query(payload: &[u8]) -> WireResult<bool> {
    match payload {
        [] => Ok(false),
        [1] => Ok(true),
        _ => Err("invalid modal query"),
    }
}
fn valid_text(text: &str, maximum: usize) -> bool {
    !text.is_empty() && text.len() <= maximum && !text.contains('\0')
}
impl Dialog {
    pub fn valid(&self) -> bool {
        fn items(items: &[(u8, String)], maximum: usize) -> bool {
            items.len() <= maximum
                && items.iter().enumerate().all(|(index, (id, label))| {
                    valid_text(label, 256)
                        && !items[..index].iter().any(|(previous, _)| previous == id)
                })
        }
        self.id != 0
            && valid_text(&self.title, 256)
            && valid_text(&self.message, 8192)
            && items(&self.choices, MAX_CHOICES)
            && !self.buttons.is_empty()
            && items(&self.buttons, MAX_BUTTONS)
            && self
                .cancel_button
                .is_none_or(|button| self.buttons.iter().any(|(id, _)| *id == button))
            && self.image.as_ref().is_none_or(|image| {
                (1..=MAX_IMAGE_EDGE).contains(&image.width)
                    && (1..=MAX_IMAGE_EDGE).contains(&image.height)
                    && image.rgba.len() == usize::from(image.width) * usize::from(image.height) * 4
            })
    }
    pub fn accepts(&self, answer: Answer) -> bool {
        if !self.valid()
            || answer.dialog_id != self.id
            || !self.buttons.iter().any(|(id, _)| *id == answer.button_id)
        {
            return false;
        }
        match answer.choice_id {
            Some(choice) => self.choices.iter().any(|(id, _)| *id == choice),
            None => self.choices.is_empty() || self.cancel_button == Some(answer.button_id),
        }
    }
}
impl Answer {
    pub fn encode(self) -> WireResult<Vec<u8>> {
        if self.dialog_id == 0 {
            return Err("invalid modal identity");
        }
        let mut out = self.dialog_id.to_le_bytes().to_vec();
        out.extend([self.button_id, u8::from(self.choice_id.is_some())]);
        if let Some(choice) = self.choice_id {
            out.push(choice);
        }
        Ok(out)
    }
    pub fn decode(bytes: &[u8]) -> WireResult<Self> {
        if bytes.len() < 6 {
            return Err("truncated modal answer");
        }
        let choice_id = match (bytes[5], bytes.len()) {
            (0, 6) => None,
            (1, 7) => Some(bytes[6]),
            _ => return Err("invalid modal choice"),
        };
        let answer = Self {
            dialog_id: u32::from_le_bytes(bytes[..4].try_into().map_err(|_| "modal id")?),
            button_id: bytes[4],
            choice_id,
        };
        answer.encode()?;
        Ok(answer)
    }
}
impl Reply {
    pub fn encode(&self) -> WireResult<Vec<u8>> {
        codec::encode(self)
    }
    pub fn decode(bytes: &[u8]) -> WireResult<Self> {
        codec::decode(bytes)
    }
}
#[path = "server_modal/codec.rs"]
mod codec;
#[path = "server_modal/input.rs"]
pub mod input;

#[cfg(test)]
mod tests {
    use super::*;
    fn dialog() -> Dialog {
        Dialog {
            id: 42,
            title: "Choose".into(),
            message: "Select one entry".into(),
            choices: vec![(7, "First".into()), (9, "Second".into())],
            buttons: vec![(1, "Accept".into()), (2, "Cancel".into())],
            cancel_button: Some(2),
            image: Some(Image {
                width: 2,
                height: 1,
                rgba: vec![255, 0, 0, 255, 0, 0, 255, 255],
            }),
        }
    }
    #[test]
    fn server_modal_terminal_reply_and_query_opt_in_are_exact() -> WireResult<()> {
        assert_eq!(decode_query(&[]), Ok(false));
        assert_eq!(decode_query(QUERY_WITH_TERMINAL_REPLY), Ok(true));
        for malformed in [&[0][..], &[2][..], &[1, 0][..]] {
            assert!(decode_query(malformed).is_err());
        }
        let answer = Answer {
            dialog_id: 42,
            button_id: 1,
            choice_id: Some(6),
        };
        for (reply, tag) in [
            (Reply::Answered(answer), 2),
            (Reply::AnsweredAndLoggedOut(answer), 3),
        ] {
            let mut bytes = vec![tag];
            bytes.extend(answer.encode()?);
            assert_eq!(reply.encode()?, bytes);
            assert_eq!(Reply::decode(&bytes)?, reply);
            bytes.push(0);
            assert!(Reply::decode(&bytes).is_err());
        }
        assert!(Reply::decode(&[3]).is_err());
        assert!(Reply::decode(&[4]).is_err());
        Ok(())
    }
    #[test]
    fn largest_server_modal_fits_the_existing_command_result_budget()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut value = dialog();
        value.title = "t".repeat(256);
        value.message = "m".repeat(8192);
        value.choices = (0..MAX_CHOICES)
            .map(|id| (id as u8, "c".repeat(256)))
            .collect();
        value.buttons = (0..MAX_BUTTONS)
            .map(|id| (id as u8, "b".repeat(256)))
            .collect();
        value.image = Some(Image {
            width: MAX_IMAGE_EDGE,
            height: MAX_IMAGE_EDGE,
            rgba: vec![255; usize::from(MAX_IMAGE_EDGE) * usize::from(MAX_IMAGE_EDGE) * 4],
        });
        let encoded = Reply::Dialog(value).encode()?;
        assert!(encoded.len() <= crate::MAX_COMMAND_RESULT_PAYLOAD_BYTES);
        assert_eq!(Reply::decode(&encoded)?.encode()?, encoded);
        Ok(())
    }
    #[test]
    fn server_modal_wire_round_trip_validates_choices_buttons_and_image_bounds()
    -> Result<(), Box<dyn std::error::Error>> {
        let value = dialog();
        let reply = Reply::Dialog(value.clone());
        let encoded = reply.encode()?;
        assert_eq!(Reply::decode(&encoded)?, reply);
        let answer = Answer {
            dialog_id: 42,
            button_id: 1,
            choice_id: Some(7),
        };
        assert!(value.accepts(answer));
        assert_eq!(Answer::decode(&answer.encode()?)?, answer);
        assert!(!value.accepts(Answer {
            dialog_id: 41,
            ..answer
        }));
        assert!(!value.accepts(Answer {
            button_id: 99,
            ..answer
        }));
        assert!(!value.accepts(Answer {
            choice_id: Some(99),
            ..answer
        }));
        assert!(!value.accepts(Answer {
            choice_id: None,
            ..answer
        }));
        assert!(value.accepts(Answer {
            button_id: 2,
            choice_id: None,
            ..answer
        }));
        let mut trailing = encoded;
        trailing.push(0);
        assert!(Reply::decode(&trailing).is_err());
        let mut duplicate = value.clone();
        duplicate.choices.push((7, "Duplicate".into()));
        assert!(Reply::Dialog(duplicate).encode().is_err());
        let mut image = value;
        if let Some(image) = &mut image.image {
            image.rgba.pop();
        }
        assert!(Reply::Dialog(image).encode().is_err());
        assert!(Answer::decode(&[0; 64]).is_err());
        Ok(())
    }
}
