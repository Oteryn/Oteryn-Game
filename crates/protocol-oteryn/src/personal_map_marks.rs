//! Personal map display facts. No client command can create or alter a marker.
pub const CAPABILITY_PERSONAL_MAP_MARKS_V1: u32 = 451;
pub const COMMAND_TYPE_PERSONAL_MAP_MARKS_QUERY: u32 = 741;
pub const MAX_MARKS: usize = 256;
pub const MAX_PAYLOAD_BYTES: usize = 2 + MAX_MARKS * (2 + 128 + 2 + 256 + 6);
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersonalMapMark {
    pub key: String,
    pub x: u16,
    pub y: u16,
    pub floor: i8,
    pub text: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PersonalMapMarks(pub Vec<PersonalMapMark>);
impl PersonalMapMarks {
    pub fn encode(&self) -> Option<Vec<u8>> {
        if self.0.len() > MAX_MARKS || self.0.windows(2).any(|pair| pair[0].key >= pair[1].key) {
            return None;
        }
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&u16::try_from(self.0.len()).ok()?.to_le_bytes());
        for mark in &self.0 {
            if !(-15..=0).contains(&mark.floor) || mark.text.chars().count() > 128 {
                return None;
            }
            for (value, max) in [(&mark.key, 128), (&mark.text, 256)] {
                if value.trim().is_empty()
                    || value.len() > max
                    || value.chars().any(char::is_control)
                {
                    return None;
                }
                bytes.extend_from_slice(&u16::try_from(value.len()).ok()?.to_le_bytes());
                bytes.extend_from_slice(value.as_bytes());
            }
            bytes.extend_from_slice(&mark.x.to_le_bytes());
            bytes.extend_from_slice(&mark.y.to_le_bytes());
            bytes.push(mark.floor as u8);
            bytes.push(1); // Cross; unknown kinds are refused, never rendered as Cross.
        }
        (bytes.len() <= MAX_PAYLOAD_BYTES).then_some(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_PAYLOAD_BYTES {
            return None;
        }
        let mut rest = bytes;
        fn take<'a>(rest: &mut &'a [u8], size: usize) -> Option<&'a [u8]> {
            let value = rest.get(..size)?;
            *rest = &rest[size..];
            Some(value)
        }
        fn integer(rest: &mut &[u8]) -> Option<u16> {
            Some(u16::from_le_bytes(take(rest, 2)?.try_into().ok()?))
        }
        fn text(rest: &mut &[u8], max: usize) -> Option<String> {
            let size = usize::from(integer(rest)?);
            if size > max {
                return None;
            }
            Some(std::str::from_utf8(take(rest, size)?).ok()?.to_owned())
        }
        let count = usize::from(integer(&mut rest)?);
        if count > MAX_MARKS {
            return None;
        }
        let mut marks = Vec::with_capacity(count);
        for _ in 0..count {
            let key = text(&mut rest, 128)?;
            let text = text(&mut rest, 256)?;
            let x = integer(&mut rest)?;
            let y = integer(&mut rest)?;
            let floor = take(&mut rest, 1)?[0] as i8;
            if take(&mut rest, 1)? != [1] {
                return None;
            }
            marks.push(PersonalMapMark {
                key,
                x,
                y,
                floor,
                text,
            });
        }
        let result = Self(marks);
        if !rest.is_empty() || result.encode()?.as_slice() != bytes {
            return None;
        }
        Some(result)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn personal_map_marks_strict_round_trip_and_bounds() -> Result<(), &'static str> {
        let value = PersonalMapMarks(vec![PersonalMapMark {
            key: "sterling".into(),
            x: 31948,
            y: 31924,
            floor: -7,
            text: "Treasure".into(),
        }]);
        let bytes = value.encode().ok_or("encode")?;
        assert_eq!(PersonalMapMarks::decode(&bytes), Some(value.clone()));
        for length in 0..bytes.len() {
            assert!(PersonalMapMarks::decode(&bytes[..length]).is_none());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(PersonalMapMarks::decode(&trailing).is_none());
        let mut unknown = bytes;
        *unknown.last_mut().ok_or("kind")? = 2;
        assert!(PersonalMapMarks::decode(&unknown).is_none());
        let mut invalid = value.clone();
        invalid.0[0].floor = 1;
        assert!(invalid.encode().is_none());
        invalid = value.clone();
        invalid.0[0].text = "x".repeat(257);
        assert!(invalid.encode().is_none());
        invalid = value.clone();
        invalid.0.push(value.0[0].clone());
        assert!(invalid.encode().is_none());
        Ok(())
    }
    #[test]
    fn personal_map_marks_admit_exact_maximum_and_refuse_overflow() -> Result<(), &'static str> {
        let value = PersonalMapMarks(
            (0..MAX_MARKS)
                .map(|index| PersonalMapMark {
                    key: format!("{index:0>128}"),
                    x: u16::MAX,
                    y: u16::MAX,
                    floor: -15,
                    text: "é".repeat(128),
                })
                .collect(),
        );
        let bytes = value.encode().ok_or("maximum")?;
        assert_eq!(bytes.len(), MAX_PAYLOAD_BYTES);
        assert_eq!(PersonalMapMarks::decode(&bytes), Some(value.clone()));
        let mut too_many = value;
        too_many.0.push(PersonalMapMark {
            key: "z".repeat(128),
            x: 0,
            y: 0,
            floor: 0,
            text: "Treasure".into(),
        });
        assert!(too_many.encode().is_none());
        assert_eq!(
            PersonalMapMarks::decode(&[0, 0]),
            Some(PersonalMapMarks::default())
        );
        Ok(())
    }
}
