//! Explicit first-vocation choice after the Newhaven portal.
pub const CAPABILITY_NEWHAVEN_VOCATION_V1: u32 = 238;
pub const COMMAND_TYPE_NEWHAVEN_VOCATION: u32 = 456;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewhavenVocationRequest {
    pub vocation: u8,
}
impl NewhavenVocationRequest {
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        match bytes {
            [8, vocation @ 1..=5] => Some(Self {
                vocation: *vocation,
            }),
            _ => None,
        }
    }
    pub fn encode(self) -> Option<Vec<u8>> {
        self.key().map(|_| vec![8, self.vocation])
    }
    pub fn key(self) -> Option<&'static str> {
        match self.vocation {
            1 => Some("sorcerer"),
            2 => Some("druid"),
            3 => Some("paladin"),
            4 => Some("knight"),
            5 => Some("monk"),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NewhavenChoiceResult {
    Chosen = 1,
    AlreadyChosen = 2,
    Rejected = 3,
    Unavailable = 4,
}
impl NewhavenChoiceResult {
    pub fn encode(self) -> Vec<u8> {
        vec![8, self as u8]
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        match bytes {
            [8, 1] => Some(Self::Chosen),
            [8, 2] => Some(Self::AlreadyChosen),
            [8, 3] => Some(Self::Rejected),
            [8, 4] => Some(Self::Unavailable),
            _ => None,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn newhaven_choice_accepts_only_five_canonical_base_vocations() {
        for vocation in 1..=5 {
            let value = NewhavenVocationRequest { vocation };
            assert_eq!(
                value
                    .encode()
                    .as_deref()
                    .and_then(NewhavenVocationRequest::decode),
                Some(value)
            );
        }
        for bytes in [&[][..], &[8, 0], &[8, 6], &[8, 1, 0], &[16, 1]] {
            assert!(NewhavenVocationRequest::decode(bytes).is_none());
        }
    }
}
