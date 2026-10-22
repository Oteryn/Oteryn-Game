//! Required-field profile over the existing strict charm wire primitives.
use super::{AppearanceError, Context};
pub(super) use crate::charm_wire::{push_message_field as bytes, push_varint_field as uint};
use crate::charm_wire::{read_bytes, read_varint};
pub(super) type Result<T> = std::result::Result<T, AppearanceError>;
pub(super) fn malformed<T>() -> Result<T> {
    Err(AppearanceError::Malformed)
}

#[derive(Clone, Copy)]
enum Field<'a> {
    Uint(u64),
    Bytes(&'a [u8]),
}
pub(super) struct Fields<'a> {
    fields: Vec<(usize, Field<'a>)>,
}
impl<'a> Fields<'a> {
    pub(super) fn read(
        input: &'a [u8],
        maximum: usize,
        types: &[u8],
        repeated: &[usize],
    ) -> Result<Self> {
        if input.len() > maximum {
            return Err(AppearanceError::LimitExceeded);
        }
        let mut fields = Vec::new();
        let mut cursor = 0;
        while cursor < input.len() {
            let tag = read_varint(input, &mut cursor)?;
            let field = usize::try_from(tag >> 3).map_err(|_| AppearanceError::Malformed)?;
            if field == 0
                || types.get(field - 1).copied() != Some((tag & 7) as u8)
                || (!repeated.contains(&field) && fields.iter().any(|(f, _)| *f == field))
            {
                return malformed();
            }
            let value = match tag & 7 {
                0 => Field::Uint(read_varint(input, &mut cursor)?),
                2 => Field::Bytes(read_bytes(input, &mut cursor)?),
                _ => return malformed(),
            };
            fields.push((field, value));
            if fields.len() > 1024 {
                return Err(AppearanceError::LimitExceeded);
            }
        }
        Ok(Self { fields })
    }
    pub(super) fn u(&self, field: usize) -> Result<u64> {
        match self.fields.iter().find(|(f, _)| *f == field) {
            Some((_, Field::Uint(v))) => Ok(*v),
            _ => malformed(),
        }
    }
    pub(super) fn b(&self, field: usize) -> Result<&'a [u8]> {
        match self.fields.iter().find(|(f, _)| *f == field) {
            Some((_, Field::Bytes(v))) => Ok(v),
            _ => malformed(),
        }
    }
    pub(super) fn has(&self, field: usize) -> bool {
        self.fields.iter().any(|(f, _)| *f == field)
    }
    pub(super) fn all(&self, field: usize) -> Result<Vec<&'a [u8]>> {
        self.fields
            .iter()
            .filter(|(f, _)| *f == field)
            .map(|(_, v)| match v {
                Field::Bytes(v) => Ok(*v),
                _ => malformed(),
            })
            .collect()
    }
    pub(super) fn context(&self) -> Result<Context> {
        Ok(Context {
            content_generation: self
                .b(1)?
                .try_into()
                .map_err(|_| AppearanceError::Malformed)?,
            cosmetics_digest: self
                .b(2)?
                .try_into()
                .map_err(|_| AppearanceError::Malformed)?,
        })
    }
}
pub(super) fn context(output: &mut Vec<u8>, value: Context) {
    bytes(output, 1, &value.content_generation);
    bytes(output, 2, &value.cosmetics_digest);
}
pub(super) fn text(value: &[u8], maximum: usize) -> Result<String> {
    let value = std::str::from_utf8(value).map_err(|_| AppearanceError::Malformed)?;
    if value.is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
        return malformed();
    }
    Ok(value.to_owned())
}
pub(super) fn nonzero(value: u64) -> Result<u64> {
    if value == 0 { malformed() } else { Ok(value) }
}
pub(super) fn narrow<T: TryFrom<u64>>(value: u64) -> Result<T> {
    T::try_from(value).map_err(|_| AppearanceError::Malformed)
}
pub(super) fn ordered<T: Ord>(values: impl Iterator<Item = T>) -> Result<()> {
    let mut previous = None;
    for value in values {
        if previous.as_ref().is_some_and(|p| p >= &value) {
            return malformed();
        }
        previous = Some(value);
    }
    Ok(())
}
pub(super) fn bound(bytes: Vec<u8>, maximum: usize) -> Result<Vec<u8>> {
    if bytes.len() > maximum {
        Err(AppearanceError::LimitExceeded)
    } else {
        Ok(bytes)
    }
}
