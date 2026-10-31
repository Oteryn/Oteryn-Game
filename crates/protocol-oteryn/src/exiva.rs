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
    List,
    AddName,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub operation: Operation,
    pub modes: u8,
    pub player: Option<[u8; 16]>,
    pub guild: Option<[u8; 16]>,
    pub name: Option<String>,
}
impl Request {
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        match bytes {
            [0, modes] if modes & !0x1f == 0 => Some(Self {
                operation: Operation::SetModes,
                modes: *modes,
                player: None,
                guild: None,
                name: None,
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
                name: None,
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
                name: None,
            }),
            [5] => Some(Self {
                operation: Operation::List,
                modes: 0,
                player: None,
                guild: None,
                name: None,
            }),
            [6, rest @ ..] => {
                let name = std::str::from_utf8(rest).ok()?;
                valid_name(name).then(|| Self {
                    operation: Operation::AddName,
                    modes: 0,
                    player: None,
                    guild: None,
                    name: Some(name.to_owned()),
                })
            }
            _ => None,
        }
    }
}

/// Oteryn character names follow the existing 2..29 ASCII letter/space naming contract.
pub fn valid_name(name: &str) -> bool {
    (2..=29).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphabetic() || b == b' ')
        && name.trim() == name
        && !name.contains("  ")
}

/// Full authoritative player list; the UUID keeps removal stable across renames.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerList {
    pub modes: u8,
    pub players: Vec<([u8; 16], String)>,
}
impl PlayerList {
    pub fn encode(&self) -> Option<Vec<u8>> {
        if self.modes & !31 != 0 || self.players.len() > 100 {
            return None;
        }
        let mut bytes = vec![2, self.modes, u8::try_from(self.players.len()).ok()?];
        for (id, name) in &self.players {
            if !valid_name(name) {
                return None;
            }
            bytes.extend(id);
            bytes.push(u8::try_from(name.len()).ok()?);
            bytes.extend(name.as_bytes());
        }
        Some(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let [2, modes, count, rest @ ..] = bytes else {
            return None;
        };
        let mut rest = rest;
        if modes & !31 != 0 || *count > 100 {
            return None;
        }
        let mut players = Vec::new();
        for _ in 0..*count {
            let id = rest.get(..16)?.try_into().ok()?;
            let len = usize::from(*rest.get(16)?);
            let name = std::str::from_utf8(rest.get(17..17 + len)?).ok()?;
            if !valid_name(name) || players.iter().any(|(held, _)| *held == id) {
                return None;
            }
            players.push((id, name.to_owned()));
            rest = rest.get(17 + len..)?;
        }
        rest.is_empty().then_some(Self {
            modes: *modes,
            players,
        })
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
                    guild: None,
                    name: None
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

#[cfg(test)]
mod client_list_tests {
    use super::*;
    #[test]
    fn exiva_list_and_name_edits_are_decodable() {
        assert!(Request::decode(&[5]).is_some());
        let mut payload = vec![6];
        payload.extend(b"Fixture Caster");
        assert!(Request::decode(&payload).is_some());
        assert!(Request::decode(&[6]).is_none());
        assert!(Request::decode(&[5, 0]).is_none());
    }
}

#[cfg(test)]
mod list_boundary_tests {
    use super::*;
    #[test]
    fn exiva_player_list_rejects_noncanonical_and_overbound_responses() -> Result<(), &'static str>
    {
        let list = PlayerList {
            modes: 31,
            players: vec![([7; 16], "Fixture Caster".into())],
        };
        let encoded = list.encode().ok_or("encode")?;
        assert_eq!(PlayerList::decode(&encoded), Some(list.clone()));
        for end in 0..encoded.len() {
            assert!(PlayerList::decode(&encoded[..end]).is_none());
        }
        let mut extra = encoded.clone();
        extra.push(0);
        assert!(PlayerList::decode(&extra).is_none());
        let mut invalid_modes = encoded.clone();
        invalid_modes[1] = 32;
        assert!(PlayerList::decode(&invalid_modes).is_none());
        let mut duplicate = encoded;
        duplicate[2] = 2;
        duplicate.extend_from_within(3..);
        assert!(PlayerList::decode(&duplicate).is_none());
        assert!(
            PlayerList {
                players: vec![([1; 16], "A".into())],
                ..list.clone()
            }
            .encode()
            .is_none()
        );
        assert!(
            PlayerList {
                players: vec![([1; 16], "A".repeat(30))],
                ..list
            }
            .encode()
            .is_none()
        );
        assert!(PlayerList::decode(&[2, 0, 101]).is_none());
        Ok(())
    }
}
