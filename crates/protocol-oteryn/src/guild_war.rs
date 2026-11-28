//! GUILD-WAR-1 bounded declaration/acceptance wire contract.
use crate::charm_wire::{
    CyclopediaWireError, WireResult, push_message_field, push_varint_field, read_bytes,
    read_varint, set_once,
};
pub const CAPABILITY_GUILD_WAR_V1: u32 = 103;
pub const CAPABILITY_GUILD_WAR_STAKE_V1: u32 = 628;
pub const MAX_GUILD_WAR_STAKE: u32 = 2_000_000_000;
pub const COMMAND_TYPE_GUILD_WAR_INTENT: u32 = 119;
pub const MAX_GUILD_WAR_REQUEST_BYTES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildWarIntent {
    Declare {
        opponent: [u8; 16],
        frag_limit: u16,
        duration_days: u16,
    },
    Accept {
        war: [u8; 16],
    },
    DeclareWithStake {
        opponent: [u8; 16],
        frag_limit: u16,
        duration_days: u16,
        stake: u32,
    },
    AcceptWithStake {
        war: [u8; 16],
        stake: u32,
    },
}

impl GuildWarIntent {
    pub fn stake(self) -> Option<u32> {
        match self {
            Self::DeclareWithStake { stake, .. } | Self::AcceptWithStake { stake, .. } => {
                Some(stake)
            }
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildWarResult {
    pub code: u8,
    pub war: [u8; 16],
}

fn uuid(bytes: &[u8]) -> WireResult<[u8; 16]> {
    let value: [u8; 16] = bytes
        .try_into()
        .map_err(|_| CyclopediaWireError::Malformed)?;
    if value[6] >> 4 != 7 || value[8] & 0xc0 != 0x80 {
        return Err(CyclopediaWireError::Malformed);
    }
    Ok(value)
}
pub fn encode_guild_war_intent(value: &GuildWarIntent) -> WireResult<Vec<u8>> {
    let mut out = Vec::new();
    match value {
        GuildWarIntent::Declare {
            opponent,
            frag_limit,
            duration_days,
        }
        | GuildWarIntent::DeclareWithStake {
            opponent,
            frag_limit,
            duration_days,
            ..
        } => {
            uuid(opponent)?;
            push_varint_field(&mut out, 1, 1);
            push_message_field(&mut out, 2, opponent);
            push_varint_field(&mut out, 3, u64::from(*frag_limit));
            push_varint_field(&mut out, 4, u64::from(*duration_days));
        }
        GuildWarIntent::Accept { war } | GuildWarIntent::AcceptWithStake { war, .. } => {
            uuid(war)?;
            push_varint_field(&mut out, 1, 2);
            push_message_field(&mut out, 2, war);
        }
    }
    Ok(out)
}
pub fn decode_guild_war_intent(bytes: &[u8]) -> WireResult<GuildWarIntent> {
    if bytes.len() > MAX_GUILD_WAR_REQUEST_BYTES {
        return Err(CyclopediaWireError::LimitExceeded);
    }
    let (mut c, mut kind, mut id, mut frags, mut days, mut stake) =
        (0, None, None, None, None, None);
    while c < bytes.len() {
        match read_varint(bytes, &mut c)? {
            8 => set_once(&mut kind, read_varint(bytes, &mut c)?)?,
            18 => set_once(&mut id, uuid(read_bytes(bytes, &mut c)?)?)?,
            24 => set_once(
                &mut frags,
                u16::try_from(read_varint(bytes, &mut c)?)
                    .map_err(|_| CyclopediaWireError::Malformed)?,
            )?,
            32 => set_once(
                &mut days,
                u16::try_from(read_varint(bytes, &mut c)?)
                    .map_err(|_| CyclopediaWireError::Malformed)?,
            )?,
            40 => {
                let amount = read_varint(bytes, &mut c)?;
                if amount > u64::from(MAX_GUILD_WAR_STAKE) {
                    return Err(CyclopediaWireError::Malformed);
                }
                set_once(&mut stake, amount as u32)?;
            }
            _ => return Err(CyclopediaWireError::Malformed),
        }
    }
    match (kind, id, frags, days, stake) {
        (Some(1), Some(opponent), Some(frag_limit), Some(duration_days), None) => {
            Ok(GuildWarIntent::Declare {
                opponent,
                frag_limit,
                duration_days,
            })
        }
        (Some(2), Some(war), None, None, None) => Ok(GuildWarIntent::Accept { war }),
        (Some(1), Some(opponent), Some(frag_limit), Some(duration_days), Some(stake)) => {
            Ok(GuildWarIntent::DeclareWithStake {
                opponent,
                frag_limit,
                duration_days,
                stake,
            })
        }
        (Some(2), Some(war), None, None, Some(stake)) => {
            Ok(GuildWarIntent::AcceptWithStake { war, stake })
        }
        _ => Err(CyclopediaWireError::Malformed),
    }
}
pub fn encode_guild_war_result(value: &GuildWarResult) -> WireResult<Vec<u8>> {
    uuid(&value.war)?;
    let mut out = Vec::new();
    push_varint_field(&mut out, 1, u64::from(value.code));
    push_message_field(&mut out, 2, &value.war);
    Ok(out)
}
pub fn decode_guild_war_result(bytes: &[u8]) -> WireResult<GuildWarResult> {
    let (mut c, mut code, mut war) = (0, None, None);
    while c < bytes.len() {
        match read_varint(bytes, &mut c)? {
            8 => set_once(
                &mut code,
                u8::try_from(read_varint(bytes, &mut c)?)
                    .map_err(|_| CyclopediaWireError::Malformed)?,
            )?,
            18 => set_once(&mut war, uuid(read_bytes(bytes, &mut c)?)?)?,
            _ => return Err(CyclopediaWireError::Malformed),
        }
    }
    Ok(GuildWarResult {
        code: code.ok_or(CyclopediaWireError::Malformed)?,
        war: war.ok_or(CyclopediaWireError::Malformed)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_declaration_and_acceptance_bytes_are_unchanged() -> WireResult<()> {
        let id = [1, 1, 1, 1, 1, 1, 0x71, 1, 0x81, 1, 1, 1, 1, 1, 1, 1];
        let declare = [
            8, 1, 18, 16, 1, 1, 1, 1, 1, 1, 0x71, 1, 0x81, 1, 1, 1, 1, 1, 1, 1, 24, 10, 32, 7,
        ];
        let accept = [
            8, 2, 18, 16, 1, 1, 1, 1, 1, 1, 0x71, 1, 0x81, 1, 1, 1, 1, 1, 1, 1,
        ];
        let declaration = GuildWarIntent::Declare {
            opponent: id,
            frag_limit: 10,
            duration_days: 7,
        };
        let acceptance = GuildWarIntent::Accept { war: id };
        assert_eq!(decode_guild_war_intent(&declare)?, declaration);
        assert_eq!(decode_guild_war_intent(&accept)?, acceptance);
        assert_eq!(encode_guild_war_intent(&declaration)?, declare);
        assert_eq!(encode_guild_war_intent(&acceptance)?, accept);
        Ok(())
    }

    #[test]
    fn round_trip() -> WireResult<()> {
        let mut id = [1; 16];
        id[6] = 0x71;
        id[8] = 0x81;
        for value in [
            GuildWarIntent::Declare {
                opponent: id,
                frag_limit: 100,
                duration_days: 14,
            },
            GuildWarIntent::Accept { war: id },
            GuildWarIntent::DeclareWithStake {
                opponent: id,
                frag_limit: 100,
                duration_days: 14,
                stake: MAX_GUILD_WAR_STAKE,
            },
            GuildWarIntent::AcceptWithStake { war: id, stake: 0 },
        ] {
            assert_eq!(
                decode_guild_war_intent(&encode_guild_war_intent(&value)?),
                Ok(value)
            );
        }
        assert!(decode_guild_war_intent(&[0; 65]).is_err());
        Ok(())
    }
}
