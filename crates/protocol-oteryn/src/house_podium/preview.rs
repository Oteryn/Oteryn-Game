//! Public artwork lookup for a podium draft; it grants no display ownership.
pub const COMMAND_PREVIEW: u32 = 739;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub display_kind: u8,
    pub direction: u8,
    pub key: String,
}
impl Request {
    pub fn encode(&self) -> Option<Vec<u8>> {
        if !(1..=3).contains(&self.display_kind)
            || self.direction > 3
            || self.key.is_empty()
            || self.key.len() > 160
        {
            return None;
        }
        let mut out = vec![
            self.display_kind,
            self.direction,
            u8::try_from(self.key.len()).ok()?,
        ];
        out.extend(self.key.as_bytes());
        Some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 3 || bytes.len() != 3 + usize::from(bytes[2]) {
            return None;
        }
        let request = Self {
            display_kind: bytes[0],
            direction: bytes[1],
            key: std::str::from_utf8(&bytes[3..]).ok()?.into(),
        };
        request.encode()?;
        Some(request)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Artwork {
    pub object: bool,
    pub direction: u8,
    pub looks: [u32; 2],
}
impl Artwork {
    pub fn encode(value: Option<Self>) -> Option<Vec<u8>> {
        let Some(value) = value else {
            return Some(vec![0]);
        };
        if value.direction > 3 || value.looks.contains(&0) {
            return None;
        }
        let mut out = vec![1, u8::from(value.object), value.direction];
        for look in value.looks {
            out.extend(look.to_be_bytes());
        }
        Some(out)
    }
    pub fn decode(bytes: &[u8]) -> Option<Option<Self>> {
        if bytes == [0] {
            return Some(None);
        }
        if bytes.len() != 11 || bytes[0] != 1 || bytes[1] > 1 {
            return None;
        }
        let value = Self {
            object: bytes[1] == 1,
            direction: bytes[2],
            looks: [
                u32::from_be_bytes(bytes[3..7].try_into().ok()?),
                u32::from_be_bytes(bytes[7..11].try_into().ok()?),
            ],
        };
        Self::encode(Some(value))?;
        Some(Some(value))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn podium_preview_codec_bounds_content_and_preserves_direction() -> Result<(), &'static str> {
        let request = Request {
            display_kind: 3,
            direction: 1,
            key: "oteryn:creature.rat".into(),
        };
        assert_eq!(
            Request::decode(&request.encode().ok_or("encode")?),
            Some(request.clone())
        );
        assert!(
            Request {
                key: "x".repeat(161),
                ..request
            }
            .encode()
            .is_none()
        );
        let art = Artwork {
            object: false,
            direction: 3,
            looks: [21, 21],
        };
        assert_eq!(
            Artwork::decode(&Artwork::encode(Some(art)).ok_or("art")?),
            Some(Some(art))
        );
        assert!(Artwork::decode(&[1, 0, 9, 0, 0, 0, 21, 0, 0, 0, 21]).is_none());
        Ok(())
    }
}
