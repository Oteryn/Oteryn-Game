use super::wire::*;
use super::*;

impl OutfitOption {
    fn encode(&self) -> Result<Vec<u8>> {
        text(self.key.as_bytes(), 512)?;
        nonzero(u64::from(self.look_type))?;
        let mut out = Vec::new();
        bytes(&mut out, 1, self.key.as_bytes());
        uint(&mut out, 2, u64::from(self.look_type));
        uint(&mut out, 3, u64::from(self.permitted_addons));
        Ok(out)
    }
    fn decode(input: &[u8]) -> Result<Self> {
        let f = Fields::read(input, 540, &[2, 0, 0], &[])?;
        Ok(Self {
            key: text(f.b(1)?, 512)?,
            look_type: narrow(nonzero(f.u(2)?)?)?,
            permitted_addons: narrow(f.u(3)?)?,
        })
    }
}
impl Catalogue {
    pub fn encode(&self) -> Result<Vec<u8>> {
        nonzero(self.root_revision)?;
        if self.palette.is_empty()
            || self.palette.len() > 256
            || self.outfits.len() > 256
            || self.mounts.len() > 256
        {
            return Err(AppearanceError::LimitExceeded);
        }
        if self
            .selected
            .as_ref()
            .is_some_and(|s| s.revision > self.root_revision)
        {
            return malformed();
        }
        ordered(self.outfits.iter().map(|o| &o.key))?;
        ordered(self.mounts.iter())?;
        let mut out = Vec::new();
        context(&mut out, self.context);
        uint(&mut out, 3, self.root_revision);
        if let Some(selected) = &self.selected {
            bytes(&mut out, 4, &selected.encode()?);
        }
        bytes(
            &mut out,
            5,
            &self.palette.iter().flatten().copied().collect::<Vec<_>>(),
        );
        for outfit in &self.outfits {
            bytes(&mut out, 6, &outfit.encode()?);
        }
        for mount in &self.mounts {
            text(mount.as_bytes(), 512)?;
            bytes(&mut out, 7, mount.as_bytes());
        }
        bound(out, 196608)
    }
    pub fn decode(input: &[u8]) -> Result<Self> {
        let f = Fields::read(input, 196608, &[2, 2, 0, 2, 2, 2, 2], &[6, 7])?;
        let raw_palette = f.b(5)?;
        if raw_palette.is_empty() || raw_palette.len() % 3 != 0 || raw_palette.len() > 768 {
            return malformed();
        }
        let result = Self {
            context: f.context()?,
            root_revision: nonzero(f.u(3)?)?,
            selected: f.has(4).then(|| Selected::decode(f.b(4)?)).transpose()?,
            palette: raw_palette.as_chunks::<3>().0.to_vec(),
            outfits: f
                .all(6)?
                .into_iter()
                .map(OutfitOption::decode)
                .collect::<Result<_>>()?,
            mounts: f
                .all(7)?
                .into_iter()
                .map(|v| text(v, 512))
                .collect::<Result<_>>()?,
        };
        result.encode()?;
        Ok(result)
    }
}
