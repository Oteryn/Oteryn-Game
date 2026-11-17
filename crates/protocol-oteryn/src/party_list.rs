//! Bounded Party List display data. No durable identity is exposed.
use super::*;
use crate::appearance::Binding;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyListEntity {
    pub entity: EntityRef,
    pub name: String,
    /// 0 unvocational, 1 knight, 2 paladin, 3 druid, 4 sorcerer, 5 monk.
    pub vocation: u8,
    pub summon: bool,
    pub distance: u32,
    /// All three are absent outside 30 horizontal fields; summons follow their owner.
    pub health_percent: Option<u8>,
    pub mana_percent: Option<u8>,
    pub skull: Option<u8>,
    pub portrait: Option<Binding>,
    pub portrait_colours: [[u8; 3]; 4],
}
impl PartyListEntity {
    pub(super) fn encode(&self) -> Result<Vec<u8>> {
        if self.entity.generation == 0
            || self.name.is_empty()
            || self.name.len() > MAX_PARTY_NAME_BYTES
            || self.vocation > 5
            || self.health_percent.is_some_and(|v| v > 100)
            || self.mana_percent.is_some_and(|v| v > 100)
            || self.skull.is_some_and(|v| v > 6)
            || (self.distance > 30
                && (self.health_percent.is_some()
                    || self.mana_percent.is_some()
                    || self.skull.is_some()))
            || self
                .portrait
                .as_ref()
                .is_some_and(|p| p.entity != self.entity)
        {
            return bad();
        }
        let mut out = self.entity.identity.to_vec();
        out.extend_from_slice(&self.entity.generation.to_be_bytes());
        out.extend_from_slice(&[self.vocation, u8::from(self.summon)]);
        out.extend_from_slice(&self.distance.to_be_bytes());
        out.extend_from_slice(&[
            self.health_percent.unwrap_or(255),
            self.mana_percent.unwrap_or(255),
            self.skull.unwrap_or(255),
        ]);
        out.push(self.name.len() as u8);
        out.extend_from_slice(self.name.as_bytes());
        let portrait = self
            .portrait
            .as_ref()
            .map(Binding::encode)
            .transpose()
            .map_err(|_| PartyWireError::Malformed)?
            .unwrap_or_default();
        out.extend_from_slice(&(portrait.len() as u16).to_be_bytes());
        out.extend_from_slice(&portrait);
        for colour in self.portrait_colours {
            out.extend_from_slice(&colour);
        }
        bounded(&out, 512)?;
        Ok(out)
    }
    pub(super) fn decode(input: &[u8]) -> Result<Self> {
        bounded(input, 512)?;
        if input.len() < 48 {
            return bad();
        }
        let name_end = 34 + usize::from(input[33]);
        if input.len() < name_end + 2 || input[25] > 1 {
            return bad();
        }
        let size = u16::from_be_bytes(
            input[name_end..name_end + 2]
                .try_into()
                .map_err(|_| PartyWireError::Malformed)?,
        ) as usize;
        if input.len() != name_end + 2 + size + 12 {
            return bad();
        }
        let optional = |v| (v != 255).then_some(v);
        let result = Self {
            entity: EntityRef {
                identity: input[..16]
                    .try_into()
                    .map_err(|_| PartyWireError::Malformed)?,
                generation: u64::from_be_bytes(
                    input[16..24]
                        .try_into()
                        .map_err(|_| PartyWireError::Malformed)?,
                ),
            },
            vocation: input[24],
            summon: input[25] == 1,
            distance: u32::from_be_bytes(
                input[26..30]
                    .try_into()
                    .map_err(|_| PartyWireError::Malformed)?,
            ),
            health_percent: optional(input[30]),
            mana_percent: optional(input[31]),
            skull: optional(input[32]),
            name: std::str::from_utf8(&input[34..name_end])
                .map_err(|_| PartyWireError::Malformed)?
                .into(),
            portrait_colours: input[name_end + 2 + size..]
                .as_chunks::<3>()
                .0
                .try_into()
                .map_err(|_| PartyWireError::Malformed)?,
            portrait: if size == 0 {
                None
            } else {
                Some(
                    Binding::decode(&input[name_end + 2..name_end + 2 + size])
                        .map_err(|_| PartyWireError::Malformed)?,
                )
            },
        };
        result.encode()?;
        Ok(result)
    }
}
