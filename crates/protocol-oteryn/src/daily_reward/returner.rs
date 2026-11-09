//! Game-only welcome-back entitlement, DAILY_REWARD_V2 owner decision B.
pub const QUERY: u32 = 6119;
pub const CLAIM: u32 = 6120;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reply {
    pub state: u8,
    pub id: Option<[u8; 16]>,
}
fn valid(id: &[u8; 16]) -> bool {
    id[6] >> 4 == 7 && id[8] >> 6 == 2
}
pub fn decode_claim(bytes: &[u8]) -> Result<[u8; 16], &'static str> {
    let id: [u8; 16] = bytes.try_into().map_err(|_| "entitlement identity")?;
    if !valid(&id) {
        return Err("entitlement identity");
    }
    Ok(id)
}
impl Reply {
    pub fn encode(self) -> Result<Vec<u8>, &'static str> {
        if self.state > 2
            || (self.state == 0) != self.id.is_none()
            || self.id.is_some_and(|id| !valid(&id))
        {
            return Err("returner reply");
        }
        let mut bytes = b"RET1".to_vec();
        bytes.push(self.state);
        bytes.extend(self.id.unwrap_or([0; 16]));
        Ok(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() != 21 || &bytes[..4] != b"RET1" {
            return Err("returner reply");
        }
        let id: [u8; 16] = bytes[5..].try_into().map_err(|_| "identity")?;
        if bytes[4] == 0 && id != [0; 16] {
            return Err("returner reply");
        }
        let reply = Self {
            state: bytes[4],
            id: if bytes[4] == 0 { None } else { Some(id) },
        };
        reply.encode()?;
        Ok(reply)
    }
}
