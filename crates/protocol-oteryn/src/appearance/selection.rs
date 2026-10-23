use super::wire::*;
use super::*;

pub(super) fn colour_bytes(value: [u16; 4]) -> Vec<u8> {
    value.into_iter().flat_map(u16::to_be_bytes).collect()
}
pub(super) fn colours(value: &[u8]) -> Result<[u16; 4]> {
    let raw: [u8; 8] = value.try_into().map_err(|_| AppearanceError::Malformed)?;
    Ok(std::array::from_fn(|i| {
        u16::from_be_bytes([raw[i * 2], raw[i * 2 + 1]])
    }))
}
impl Selection {
    pub fn encode(&self) -> Result<Vec<u8>> {
        text(self.outfit_key.as_bytes(), 512)?;
        let mut out = Vec::new();
        bytes(&mut out, 1, self.outfit_key.as_bytes());
        bytes(&mut out, 2, &colour_bytes(self.colours));
        uint(&mut out, 3, u64::from(self.addons));
        if let Some(key) = &self.mount_key {
            text(key.as_bytes(), 512)?;
            bytes(&mut out, 4, key.as_bytes());
        }
        bound(out, 1100)
    }
    pub fn decode(input: &[u8]) -> Result<Self> {
        let f = Fields::read(input, 1100, &[2, 2, 0, 2], &[])?;
        Ok(Self {
            outfit_key: text(f.b(1)?, 512)?,
            colours: colours(f.b(2)?)?,
            addons: narrow(f.u(3)?)?,
            mount_key: f.has(4).then(|| text(f.b(4)?, 512)).transpose()?,
        })
    }
}
impl Selected {
    pub fn encode(&self) -> Result<Vec<u8>> {
        nonzero(self.revision)?;
        text(self.provenance.as_bytes(), 128)?;
        let mut out = Vec::new();
        bytes(&mut out, 1, &self.selection.encode()?);
        uint(&mut out, 2, self.revision);
        bytes(&mut out, 3, self.provenance.as_bytes());
        bound(out, 1250)
    }
    pub fn decode(input: &[u8]) -> Result<Self> {
        let f = Fields::read(input, 1250, &[2, 0, 2], &[])?;
        Ok(Self {
            selection: Selection::decode(f.b(1)?)?,
            revision: nonzero(f.u(2)?)?,
            provenance: text(f.b(3)?, 128)?,
        })
    }
}
impl Change {
    pub fn encode(&self) -> Result<Vec<u8>> {
        nonzero(self.expected_root_revision)?;
        text(self.content_revision.as_bytes(), 128)?;
        if self
            .prior
            .as_ref()
            .is_some_and(|p| p.revision > self.expected_root_revision)
        {
            return malformed();
        }
        let mut out = Vec::new();
        context(&mut out, self.context);
        uint(&mut out, 3, self.expected_root_revision);
        if let Some(prior) = &self.prior {
            bytes(&mut out, 4, &prior.encode()?);
        }
        bytes(&mut out, 5, &self.selection.encode()?);
        bytes(&mut out, 6, self.content_revision.as_bytes());
        bound(out, 4096)
    }
    pub fn decode(input: &[u8]) -> Result<Self> {
        let f = Fields::read(input, 4096, &[2, 2, 0, 2, 2, 2], &[])?;
        let result = Self {
            context: f.context()?,
            expected_root_revision: nonzero(f.u(3)?)?,
            prior: f.has(4).then(|| Selected::decode(f.b(4)?)).transpose()?,
            selection: Selection::decode(f.b(5)?)?,
            content_revision: text(f.b(6)?, 128)?,
        };
        result.encode()?;
        Ok(result)
    }
}
impl ChangeResult {
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        match self {
            Self::Committed(revision) => {
                nonzero(*revision)?;
                uint(&mut out, 1, 1);
                uint(&mut out, 2, *revision);
            }
            Self::Rejected => uint(&mut out, 1, 2),
        }
        bound(out, 16)
    }
    pub fn decode(input: &[u8]) -> Result<Self> {
        let f = Fields::read(input, 16, &[0, 0], &[])?;
        match f.u(1)? {
            1 => Ok(Self::Committed(nonzero(f.u(2)?)?)),
            2 if !f.has(2) => Ok(Self::Rejected),
            _ => malformed(),
        }
    }
}
