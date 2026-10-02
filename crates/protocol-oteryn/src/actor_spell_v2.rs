//! Candidate parameter-bearing spell intent. Numeric command/capability IDs are
//! deliberately owned by the protocol registry, not allocated by this codec.
use crate::actor_spell::{
    ActorSpellError, SpellCastIntent, decode_spell_cast_intent, encode_spell_cast_intent,
};

pub const CAPABILITY_NAME: &str = "ACTOR_SPELL_PARAMETERS_V2";
pub const COMMAND_NAME: &str = "WORLD_ACTOR_SPELL_CAST_PARAMETERS_V2";
pub const MAX_PARAMETER_BYTES: usize = 128;
pub const MAX_PARAMETER_INTENT_BYTES: usize = 192;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterSpellCastIntent {
    pub intent: SpellCastIntent,
    /// Absence and explicitly empty text are distinct. Spell-specific rules
    /// interpret this value after resolving the current content book.
    pub parameter: Option<String>,
}
fn varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 128 {
        out.push((value as u8 & 127) | 128);
        value >>= 7;
    }
    out.push(value as u8);
}
fn read(input: &[u8], cursor: &mut usize) -> Result<u64, ActorSpellError> {
    let mut result = 0;
    for shift in (0..70).step_by(7) {
        let byte = *input.get(*cursor).ok_or(ActorSpellError::Malformed)?;
        *cursor += 1;
        if shift == 63 && byte > 1 {
            return Err(ActorSpellError::Malformed);
        }
        result |= u64::from(byte & 127) << shift;
        if byte & 128 == 0 {
            return Ok(result);
        }
    }
    Err(ActorSpellError::Malformed)
}
fn bytes<'a>(input: &'a [u8], cursor: &mut usize) -> Result<&'a [u8], ActorSpellError> {
    let length = usize::try_from(read(input, cursor)?).map_err(|_| ActorSpellError::Malformed)?;
    let end = cursor
        .checked_add(length)
        .ok_or(ActorSpellError::Malformed)?;
    let result = input.get(*cursor..end).ok_or(ActorSpellError::Malformed)?;
    *cursor = end;
    Ok(result)
}
fn validate(text: &str) -> Result<(), ActorSpellError> {
    if text.len() > MAX_PARAMETER_BYTES {
        return Err(ActorSpellError::LimitExceeded);
    }
    if text.chars().any(char::is_control) {
        return Err(ActorSpellError::Malformed);
    }
    Ok(())
}
pub fn encode_parameter_spell_cast_intent(
    value: &ParameterSpellCastIntent,
) -> Result<Vec<u8>, ActorSpellError> {
    if let Some(text) = &value.parameter {
        validate(text)?;
    }
    let intent = encode_spell_cast_intent(&value.intent);
    let mut output = vec![8, 2, 18];
    varint(&mut output, intent.len() as u64);
    output.extend_from_slice(&intent);
    if let Some(text) = &value.parameter {
        output.push(26);
        varint(&mut output, text.len() as u64);
        output.extend_from_slice(text.as_bytes());
    }
    Ok(output)
}
pub fn decode_parameter_spell_cast_intent(
    input: &[u8],
) -> Result<ParameterSpellCastIntent, ActorSpellError> {
    if input.len() > MAX_PARAMETER_INTENT_BYTES {
        return Err(ActorSpellError::LimitExceeded);
    }
    let (mut cursor, mut revision, mut intent, mut parameter) = (0, None, None, None);
    let mut text_seen = false;
    while cursor < input.len() {
        match read(input, &mut cursor)? {
            8 if revision.is_none() => revision = Some(read(input, &mut cursor)?),
            18 if intent.is_none() => {
                intent = Some(decode_spell_cast_intent(bytes(input, &mut cursor)?)?)
            }
            26 if !text_seen => {
                text_seen = true;
                let text = std::str::from_utf8(bytes(input, &mut cursor)?)
                    .map_err(|_| ActorSpellError::Malformed)?;
                validate(text)?;
                parameter = Some(text.to_owned());
            }
            _ => return Err(ActorSpellError::Malformed),
        }
    }
    if revision != Some(2) {
        return Err(ActorSpellError::Malformed);
    }
    Ok(ParameterSpellCastIntent {
        intent: intent.ok_or(ActorSpellError::Malformed)?,
        parameter,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_fixture_and_absent_vs_empty() {
        let fixture = [8, 2, 18, 4, 8, 1, 16, 1, 26, 3, b'B', b'o', b'b'];
        let decoded = decode_parameter_spell_cast_intent(&fixture).unwrap();
        assert_eq!(decoded.parameter.as_deref(), Some("Bob"));
        assert_eq!(
            encode_parameter_spell_cast_intent(&decoded).unwrap(),
            fixture
        );
        assert!(decode_spell_cast_intent(&fixture).is_err());
        assert_eq!(
            decode_parameter_spell_cast_intent(&fixture[..8])
                .unwrap()
                .parameter,
            None
        );
        assert_eq!(
            decode_parameter_spell_cast_intent(&[8, 2, 18, 4, 8, 1, 16, 1, 26, 0])
                .unwrap()
                .parameter,
            Some(String::new())
        );
    }
    #[test]
    fn malformed_boundaries_fail_closed() {
        for malformed in [
            &[8, 1, 18, 4, 8, 1, 16, 1][..],               // wrong revision
            &[8, 2, 8, 2, 18, 4, 8, 1, 16, 1][..],         // repeated revision
            &[8, 2, 18, 4, 8, 1, 16, 1, 32, 1][..],        // unknown field
            &[8, 2, 18, 6, 8, 1, 16, 1, 40, 1][..],        // nested v1 unknown
            &[8, 2, 18, 4, 8, 1, 16, 1, 26, 1, 255][..],   // invalid UTF8
            &[8, 2, 18, 4, 8, 1, 16, 1, 26, 1, 10][..],    // controls
            &[8, 2, 18, 4, 8, 1, 16, 1, 26, 0, 26, 0][..], // repeated text
            &[8, 2, 18, 4, 8, 1, 16][..],                  // truncated
        ] {
            assert!(
                decode_parameter_spell_cast_intent(malformed).is_err(),
                "{malformed:?}"
            );
        }
        assert_eq!(
            decode_parameter_spell_cast_intent(&[0; 193]),
            Err(ActorSpellError::LimitExceeded)
        );
        let mut oversized = vec![8, 2, 18, 4, 8, 1, 16, 1, 26, 129, 1];
        oversized.extend_from_slice(&[b'a'; 129]);
        assert_eq!(
            decode_parameter_spell_cast_intent(&oversized),
            Err(ActorSpellError::LimitExceeded)
        );
    }
}

pub const MAX_PARAMETER_RESULT_BYTES: usize = 8192;
pub const MAX_PRIVATE_FEEDBACK_BYTES: usize = 1024;
pub const MAX_HOUSE_EDITOR_TEXT_BYTES: usize = 4096;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivateSpellEffect {
    None = 1,
    Poff = 2,
    MagicBlue = 3,
    Teleport = 4,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateSpellFeedback {
    pub text: String,
    pub effect: PrivateSpellEffect,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseEditorList {
    Guest,
    Subowner,
    Door(std::num::NonZeroU32),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellHouseEditor {
    pub editor_id: [u8; 16],
    pub house_key: String,
    pub list: HouseEditorList,
    pub ownership_revision: std::num::NonZeroU64,
    pub acl_revision: u64,
    /// The server resolves current native Character names for presentation;
    /// this text is not an authorization token and grants no membership.
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterSpellCastResult {
    pub disposition: crate::actor_spell::SpellCastDisposition,
    pub feedback: Option<PrivateSpellFeedback>,
    pub editor: Option<SpellHouseEditor>,
}
fn field(out: &mut Vec<u8>, tag: u8, value: &[u8]) {
    out.push(tag);
    varint(out, value.len() as u64);
    out.extend_from_slice(value);
}
fn scalar(out: &mut Vec<u8>, tag: u8, value: u64) {
    out.push(tag);
    varint(out, value);
}
fn valid_editor(editor: &SpellHouseEditor) -> Result<(), ActorSpellError> {
    let prefix = "oteryn:content.house.";
    if editor.editor_id[6] >> 4 != 7
        || editor.editor_id[8] & 0xc0 != 0x80
        || !editor.house_key.starts_with(prefix)
        || editor.house_key.len() == prefix.len()
        || !editor.house_key[prefix.len()..]
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        || editor.text.chars().any(|c| c.is_control() && c != '\n')
    {
        return Err(ActorSpellError::Malformed);
    }
    if editor.house_key.len() > 512 || editor.text.len() > MAX_HOUSE_EDITOR_TEXT_BYTES {
        return Err(ActorSpellError::LimitExceeded);
    }
    Ok(())
}
fn encode_feedback(value: &PrivateSpellFeedback) -> Result<Vec<u8>, ActorSpellError> {
    if value.text.len() > MAX_PRIVATE_FEEDBACK_BYTES {
        return Err(ActorSpellError::LimitExceeded);
    }
    if value.text.is_empty() && value.effect == PrivateSpellEffect::None
        || value.text.chars().any(char::is_control)
    {
        return Err(ActorSpellError::Malformed);
    }
    let mut output = Vec::new();
    field(&mut output, 10, value.text.as_bytes());
    scalar(&mut output, 16, value.effect as u64);
    Ok(output)
}
fn decode_feedback(input: &[u8]) -> Result<PrivateSpellFeedback, ActorSpellError> {
    let (mut cursor, mut text, mut effect) = (0, None, None);
    while cursor < input.len() {
        match read(input, &mut cursor)? {
            10 if text.is_none() => {
                text = Some(
                    std::str::from_utf8(bytes(input, &mut cursor)?)
                        .map_err(|_| ActorSpellError::Malformed)?
                        .to_owned(),
                )
            }
            16 if effect.is_none() => {
                effect = Some(match read(input, &mut cursor)? {
                    1 => PrivateSpellEffect::None,
                    2 => PrivateSpellEffect::Poff,
                    3 => PrivateSpellEffect::MagicBlue,
                    4 => PrivateSpellEffect::Teleport,
                    _ => return Err(ActorSpellError::Malformed),
                })
            }
            _ => return Err(ActorSpellError::Malformed),
        }
    }
    let value = PrivateSpellFeedback {
        text: text.ok_or(ActorSpellError::Malformed)?,
        effect: effect.ok_or(ActorSpellError::Malformed)?,
    };
    encode_feedback(&value)?;
    Ok(value)
}
fn encode_editor(value: &SpellHouseEditor) -> Result<Vec<u8>, ActorSpellError> {
    valid_editor(value)?;
    let mut output = Vec::new();
    field(&mut output, 10, &value.editor_id);
    field(&mut output, 18, value.house_key.as_bytes());
    let (list, door) = match value.list {
        HouseEditorList::Guest => (1, None),
        HouseEditorList::Subowner => (2, None),
        HouseEditorList::Door(id) => (3, Some(id)),
    };
    scalar(&mut output, 24, list);
    if let Some(id) = door {
        scalar(&mut output, 32, u64::from(id.get()));
    }
    scalar(&mut output, 40, value.ownership_revision.get());
    if value.acl_revision != 0 {
        scalar(&mut output, 48, value.acl_revision);
    }
    field(&mut output, 58, value.text.as_bytes());
    Ok(output)
}
fn decode_editor(input: &[u8]) -> Result<SpellHouseEditor, ActorSpellError> {
    let (
        mut cursor,
        mut identity,
        mut key,
        mut list,
        mut door,
        mut ownership,
        mut revision,
        mut text,
    ) = (0, None, None, None, None, None, None, None);
    while cursor < input.len() {
        match read(input, &mut cursor)? {
            10 if identity.is_none() => {
                identity = Some(
                    bytes(input, &mut cursor)?
                        .try_into()
                        .map_err(|_| ActorSpellError::Malformed)?,
                )
            }
            18 if key.is_none() => {
                key = Some(
                    std::str::from_utf8(bytes(input, &mut cursor)?)
                        .map_err(|_| ActorSpellError::Malformed)?
                        .to_owned(),
                )
            }
            24 if list.is_none() => list = Some(read(input, &mut cursor)?),
            32 if door.is_none() => {
                door = Some(
                    std::num::NonZeroU32::new(
                        u32::try_from(read(input, &mut cursor)?)
                            .map_err(|_| ActorSpellError::Malformed)?,
                    )
                    .ok_or(ActorSpellError::Malformed)?,
                )
            }
            40 if ownership.is_none() => {
                ownership = Some(
                    std::num::NonZeroU64::new(read(input, &mut cursor)?)
                        .ok_or(ActorSpellError::Malformed)?,
                )
            }
            48 if revision.is_none() => revision = Some(read(input, &mut cursor)?),
            58 if text.is_none() => {
                text = Some(
                    std::str::from_utf8(bytes(input, &mut cursor)?)
                        .map_err(|_| ActorSpellError::Malformed)?
                        .to_owned(),
                )
            }
            _ => return Err(ActorSpellError::Malformed),
        }
    }
    let list = match (list, door) {
        (Some(1), None) => HouseEditorList::Guest,
        (Some(2), None) => HouseEditorList::Subowner,
        (Some(3), Some(id)) => HouseEditorList::Door(id),
        _ => return Err(ActorSpellError::Malformed),
    };
    let value = SpellHouseEditor {
        editor_id: identity.ok_or(ActorSpellError::Malformed)?,
        house_key: key.ok_or(ActorSpellError::Malformed)?,
        list,
        ownership_revision: ownership.ok_or(ActorSpellError::Malformed)?,
        acl_revision: revision.unwrap_or(0),
        text: text.ok_or(ActorSpellError::Malformed)?,
    };
    valid_editor(&value)?;
    Ok(value)
}
pub fn encode_parameter_spell_cast_result(
    value: &ParameterSpellCastResult,
) -> Result<Vec<u8>, ActorSpellError> {
    if value.editor.is_some() && value.disposition != crate::actor_spell::SpellCastDisposition::Cast
    {
        return Err(ActorSpellError::Malformed);
    }
    let mut output = vec![8, 2];
    field(
        &mut output,
        18,
        &crate::actor_spell::encode_spell_cast_result(value.disposition),
    );
    if let Some(feedback) = &value.feedback {
        field(&mut output, 26, &encode_feedback(feedback)?);
    }
    if let Some(editor) = &value.editor {
        field(&mut output, 34, &encode_editor(editor)?);
    }
    if output.len() > MAX_PARAMETER_RESULT_BYTES {
        return Err(ActorSpellError::LimitExceeded);
    }
    Ok(output)
}
pub fn decode_parameter_spell_cast_result(
    input: &[u8],
) -> Result<ParameterSpellCastResult, ActorSpellError> {
    if input.len() > MAX_PARAMETER_RESULT_BYTES {
        return Err(ActorSpellError::LimitExceeded);
    }
    let (mut cursor, mut version, mut disposition, mut feedback, mut editor) =
        (0, None, None, None, None);
    while cursor < input.len() {
        match read(input, &mut cursor)? {
            8 if version.is_none() => version = Some(read(input, &mut cursor)?),
            18 if disposition.is_none() => {
                disposition = Some(crate::actor_spell::decode_spell_cast_result(bytes(
                    input,
                    &mut cursor,
                )?)?)
            }
            26 if feedback.is_none() => {
                feedback = Some(decode_feedback(bytes(input, &mut cursor)?)?)
            }
            34 if editor.is_none() => editor = Some(decode_editor(bytes(input, &mut cursor)?)?),
            _ => return Err(ActorSpellError::Malformed),
        }
    }
    if version != Some(2) {
        return Err(ActorSpellError::Malformed);
    }
    let value = ParameterSpellCastResult {
        disposition: disposition.ok_or(ActorSpellError::Malformed)?,
        feedback,
        editor,
    };
    if value.editor.is_some() && value.disposition != crate::actor_spell::SpellCastDisposition::Cast
    {
        return Err(ActorSpellError::Malformed);
    }
    Ok(value)
}
#[cfg(test)]
mod result_tests {
    use super::*;
    #[test]
    fn private_feedback_independent_fixture_and_v1_refusal() {
        let fixture = [8, 2, 18, 2, 8, 1, 26, 6, 10, 2, b'h', b'i', 16, 3];
        let result = decode_parameter_spell_cast_result(&fixture).unwrap();
        assert_eq!(result.feedback.as_ref().unwrap().text, "hi");
        assert_eq!(
            encode_parameter_spell_cast_result(&result).unwrap(),
            fixture
        );
        assert!(crate::actor_spell::decode_spell_cast_result(&fixture).is_err());
        for bad in [
            &[8, 2, 18, 2, 8, 1, 26, 0][..],
            &[8, 2, 18, 2, 8, 1, 40, 0][..],
            &[8, 2, 18, 2, 8, 1, 18, 2, 8, 1][..],
        ] {
            assert!(decode_parameter_spell_cast_result(bad).is_err());
        }
    }
    #[test]
    fn editor_binds_native_identity_and_typed_list() {
        let mut value = ParameterSpellCastResult {
            disposition: crate::actor_spell::SpellCastDisposition::Cast,
            feedback: None,
            editor: Some(SpellHouseEditor {
                editor_id: [0, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1],
                house_key: "oteryn:content.house.test".into(),
                list: HouseEditorList::Door(std::num::NonZeroU32::new(7).unwrap()),
                ownership_revision: std::num::NonZeroU64::new(1).unwrap(),
                acl_revision: 0,
                text: "Bob\n".into(),
            }),
        };
        assert_eq!(
            decode_parameter_spell_cast_result(
                &encode_parameter_spell_cast_result(&value).unwrap()
            )
            .unwrap(),
            value
        );
        value.disposition = crate::actor_spell::SpellCastDisposition::Rejected;
        assert!(encode_parameter_spell_cast_result(&value).is_err());
        value.disposition = crate::actor_spell::SpellCastDisposition::Cast;
        value.editor.as_mut().unwrap().editor_id[6] = 0;
        assert!(encode_parameter_spell_cast_result(&value).is_err());
    }
}

#[cfg(test)]
mod effect_only_result_tests {
    use super::*;
    #[test]
    fn source_effect_does_not_require_fabricated_success_message() {
        let mut value = ParameterSpellCastResult {
            disposition: crate::actor_spell::SpellCastDisposition::Cast,
            feedback: Some(PrivateSpellFeedback {
                text: String::new(),
                effect: PrivateSpellEffect::Teleport,
            }),
            editor: None,
        };
        let encoded = encode_parameter_spell_cast_result(&value).unwrap();
        assert_eq!(decode_parameter_spell_cast_result(&encoded).unwrap(), value);
        value.feedback.as_mut().unwrap().effect = PrivateSpellEffect::None;
        assert!(encode_parameter_spell_cast_result(&value).is_err());
    }
}
