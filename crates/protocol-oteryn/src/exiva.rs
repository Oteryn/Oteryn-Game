//! Target-owned Exiva settings command 170.
pub const CAPABILITY_EXIVA_SETTINGS_V1: u32 = 147;
pub const COMMAND_TYPE_EXIVA_SETTINGS: u32 = 170;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    SetModes,
    AddPlayer,
    RemovePlayer,
    AddGuild,
    RemoveGuild,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request {
    pub operation: Operation,
    pub modes: u8,
    pub player: Option<[u8; 16]>,
    pub guild: Option<[u8; 16]>,
}
impl Request {
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        match bytes {
            [0, modes] if modes & !0x1f == 0 => Some(Self {
                operation: Operation::SetModes,
                modes: *modes,
                player: None,
                guild: None,
            }),
            [operation @ (1 | 2), rest @ ..] if rest.len() == 16 => Some(Self {
                operation: if *operation == 1 {
                    Operation::AddPlayer
                } else {
                    Operation::RemovePlayer
                },
                modes: 0,
                player: Some(rest.try_into().ok()?),
                guild: None,
            }),
            [operation @ (3 | 4), rest @ ..] if rest.len() == 16 => Some(Self {
                operation: if *operation == 3 {
                    Operation::AddGuild
                } else {
                    Operation::RemoveGuild
                },
                modes: 0,
                player: None,
                guild: Some(rest.try_into().ok()?),
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_crystal_modes_and_allow_list_edits_are_canonical() {
        for modes in [1, 2, 4, 8, 16, 31] {
            assert_eq!(Request::decode(&[0, modes]).map(|r| r.modes), Some(modes));
        }
        let player = [7; 16];
        for (tag, operation) in [(1, Operation::AddPlayer), (2, Operation::RemovePlayer)] {
            let mut bytes = vec![tag];
            bytes.extend(player);
            assert_eq!(
                Request::decode(&bytes),
                Some(Request {
                    operation,
                    modes: 0,
                    player: Some(player),
                    guild: None
                })
            );
        }
        for (tag, operation) in [(3, Operation::AddGuild), (4, Operation::RemoveGuild)] {
            let mut bytes = vec![tag];
            bytes.extend(player);
            assert_eq!(
                Request::decode(&bytes).map(|r| (r.operation, r.guild)),
                Some((operation, Some(player)))
            );
        }
        assert!(Request::decode(&[0, 32]).is_none());
        assert!(Request::decode(&[1; 16]).is_none());
    }
}
