//! Own PvP observation on command 155's explicit view=1 query. Empty queries retain icons.
use super::*;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PvpSituation {
    pub incoming: bool,
    pub remaining_ms: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PvpStatus {
    pub points_milli: [u64; 3],
    pub red_limits_milli: [u64; 3],
    pub black_limits_milli: [u64; 3],
    pub skull: u8,
    pub remaining_ms: u64,
    pub situations: Vec<PvpSituation>,
}
impl PvpStatus {
    pub fn encode(&self) -> Result<Vec<u8>, PartyWireError> {
        if !matches!(self.skull, 0 | 3..=5)
            || self.remaining_ms > 45 * 86_400_000
            || (self.skull == 0 && self.remaining_ms != 0)
            || self.situations.len() > 64
            || self
                .red_limits_milli
                .iter()
                .zip(self.black_limits_milli)
                .any(|(red, black)| *red == 0 || black < *red)
        {
            return Err(PartyWireError::Malformed);
        }
        let mut body = Vec::new();
        for (index, value) in self
            .points_milli
            .into_iter()
            .chain(self.red_limits_milli)
            .chain(self.black_limits_milli)
            .chain([u64::from(self.skull), self.remaining_ms])
            .enumerate()
        {
            push_varint_field(&mut body, index as u64 + 1, value);
        }
        for situation in &self.situations {
            if situation.remaining_ms == 0 || situation.remaining_ms > 900_000 {
                return Err(PartyWireError::Malformed);
            }
            let mut entry = Vec::new();
            push_varint_field(&mut entry, 1, u64::from(situation.incoming));
            push_varint_field(&mut entry, 2, situation.remaining_ms);
            push_message_field(&mut body, 12, &entry);
        }
        let mut out = Vec::new();
        push_message_field(&mut out, 2, &body);
        if out.len() > MAX_PARTY_ICON_BYTES {
            return Err(PartyWireError::LimitExceeded);
        }
        Ok(out)
    }
    pub fn decode(input: &[u8]) -> Result<Self, PartyWireError> {
        if input.len() > MAX_PARTY_ICON_BYTES {
            return Err(PartyWireError::LimitExceeded);
        }
        let mut cursor = 0;
        if read_varint(input, &mut cursor)? != 18 {
            return Err(PartyWireError::Malformed);
        }
        let body = read_bytes(input, &mut cursor)?;
        if cursor != input.len() {
            return Err(PartyWireError::Malformed);
        }
        let mut values = [None; 11];
        let mut situations = Vec::new();
        let mut at = 0;
        while at < body.len() {
            let tag = read_varint(body, &mut at)?;
            if tag == 98 {
                if situations.len() >= 64 {
                    return Err(PartyWireError::LimitExceeded);
                }
                let entry = read_bytes(body, &mut at)?;
                let mut pos = 0;
                if read_varint(entry, &mut pos)? != 8 {
                    return Err(PartyWireError::Malformed);
                }
                let incoming = read_varint(entry, &mut pos)?;
                if incoming > 1 || read_varint(entry, &mut pos)? != 16 {
                    return Err(PartyWireError::Malformed);
                }
                let remaining_ms = read_varint(entry, &mut pos)?;
                if pos != entry.len() {
                    return Err(PartyWireError::Malformed);
                }
                situations.push(PvpSituation {
                    incoming: incoming == 1,
                    remaining_ms,
                });
            } else if tag & 7 == 0 && (1..=11).contains(&(tag >> 3)) {
                let slot = &mut values[((tag >> 3) - 1) as usize];
                set_once(slot, read_varint(body, &mut at)?)?;
            } else {
                return Err(PartyWireError::Malformed);
            }
        }
        let values = values
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(PartyWireError::Malformed)?;
        let result = Self {
            points_milli: [values[0], values[1], values[2]],
            red_limits_milli: [values[3], values[4], values[5]],
            black_limits_milli: [values[6], values[7], values[8]],
            skull: u8::try_from(values[9]).map_err(|_| PartyWireError::Malformed)?,
            remaining_ms: values[10],
            situations,
        };
        result.encode()?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn own_pvp_codec_rejects_truncated_foreign_and_oversized_observations()
    -> Result<(), PartyWireError> {
        let mut status = PvpStatus {
            points_milli: [500, 500, 500],
            red_limits_milli: [3000, 5000, 10000],
            black_limits_milli: [6000, 10000, 20000],
            skull: 4,
            remaining_ms: 1000,
            situations: vec![PvpSituation {
                incoming: false,
                remaining_ms: 2000,
            }],
        };
        let bytes = status.encode()?;
        assert_eq!(PvpStatus::decode(&bytes)?, status);
        for end in 0..bytes.len() {
            assert!(PvpStatus::decode(&bytes[..end]).is_err());
        }
        assert!(PvpStatus::decode(&super::super::encode_party_icons(&[])?).is_err());
        let mut extra = bytes;
        extra.push(0);
        assert!(PvpStatus::decode(&extra).is_err());
        status.situations = vec![
            PvpSituation {
                incoming: true,
                remaining_ms: 1
            };
            65
        ];
        assert!(status.encode().is_err());
        status.situations.clear();
        status.skull = 2;
        assert!(status.encode().is_err());
        status.skull = 0;
        assert!(status.encode().is_err());
        status.remaining_ms = 0;
        assert!(status.encode().is_ok());
        Ok(())
    }
}
