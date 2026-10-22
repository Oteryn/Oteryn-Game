use super::selection::{colour_bytes, colours};
use super::wire::*;
use super::*;
use std::collections::BTreeSet;

impl Look {
    fn encode(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        match self {
            Self::Outfit {
                look_type,
                colours,
                addons,
            } => {
                nonzero(u64::from(*look_type))?;
                uint(&mut out, 1, 1);
                uint(&mut out, 2, u64::from(*look_type));
                bytes(&mut out, 3, &colour_bytes(*colours));
                uint(&mut out, 4, u64::from(*addons));
            }
            Self::Item { definition_ref } => {
                nonzero(u64::from(*definition_ref))?;
                uint(&mut out, 1, 2);
                uint(&mut out, 5, u64::from(*definition_ref));
            }
        }
        Ok(out)
    }
    fn decode(input: &[u8]) -> Result<Self> {
        let f = Fields::read(input, 64, &[0, 0, 2, 0, 0], &[])?;
        match f.u(1)? {
            1 if !f.has(5) => Ok(Self::Outfit {
                look_type: narrow(nonzero(f.u(2)?)?)?,
                colours: colours(f.b(3)?)?,
                addons: narrow(f.u(4)?)?,
            }),
            2 if !f.has(2) && !f.has(3) && !f.has(4) => Ok(Self::Item {
                definition_ref: narrow(nonzero(f.u(5)?)?)?,
            }),
            _ => malformed(),
        }
    }
}
fn encode_entity(entity: EntityRef) -> Result<Vec<u8>> {
    nonzero(entity.generation)?;
    let mut out = Vec::new();
    bytes(&mut out, 1, &entity.identity);
    uint(&mut out, 2, entity.generation);
    Ok(out)
}
fn decode_entity(input: &[u8]) -> Result<EntityRef> {
    let f = Fields::read(input, 32, &[2, 0], &[])?;
    Ok(EntityRef {
        identity: f.b(1)?.try_into().map_err(|_| AppearanceError::Malformed)?,
        generation: nonzero(f.u(2)?)?,
    })
}
impl Binding {
    pub fn encode(&self) -> Result<Vec<u8>> {
        nonzero(u64::from(self.appearance_ref))?;
        nonzero(self.root_revision)?;
        if self
            .selection_revision
            .is_some_and(|v| v == 0 || v > self.root_revision)
        {
            return malformed();
        }
        let mut out = Vec::new();
        context(&mut out, self.context);
        uint(&mut out, 3, u64::from(self.appearance_ref));
        bytes(&mut out, 4, &encode_entity(self.entity)?);
        uint(&mut out, 5, self.root_revision);
        uint(&mut out, 6, self.condition_revision);
        uint(&mut out, 7, self.owner_time_us);
        bytes(&mut out, 8, &self.look.encode()?);
        if let Some(revision) = self.selection_revision {
            uint(&mut out, 9, revision);
        }
        bound(out, 256)
    }
    pub fn decode(input: &[u8]) -> Result<Self> {
        let f = Fields::read(input, 256, &[2, 2, 0, 2, 0, 0, 0, 2, 0], &[])?;
        let result = Self {
            context: f.context()?,
            appearance_ref: narrow(nonzero(f.u(3)?)?)?,
            entity: decode_entity(f.b(4)?)?,
            root_revision: nonzero(f.u(5)?)?,
            condition_revision: f.u(6)?,
            owner_time_us: f.u(7)?,
            look: Look::decode(f.b(8)?)?,
            selection_revision: f.has(9).then(|| f.u(9)).transpose()?,
        };
        result.encode()?;
        Ok(result)
    }
}
fn validate_bindings(context: Context, bindings: &[Binding]) -> Result<()> {
    if bindings.len() > 256 {
        return Err(AppearanceError::LimitExceeded);
    }
    ordered(bindings.iter().map(|b| b.entity))?;
    let mut refs = BTreeSet::new();
    if bindings
        .iter()
        .any(|b| b.context != context || !refs.insert(b.appearance_ref))
    {
        return malformed();
    }
    Ok(())
}
impl Mappings {
    pub fn encode(&self) -> Result<Vec<u8>> {
        validate_bindings(self.context, &self.upserts)?;
        ordered(self.forgets.iter())?;
        if self.forgets.len() > 256
            || self
                .upserts
                .iter()
                .any(|b| self.forgets.contains(&b.entity))
        {
            return malformed();
        }
        let mut out = Vec::new();
        context(&mut out, self.context);
        for binding in &self.upserts {
            bytes(&mut out, 3, &binding.encode()?);
        }
        for entity in &self.forgets {
            bytes(&mut out, 4, &encode_entity(*entity)?);
        }
        bound(out, 131072)
    }
    pub fn decode(input: &[u8]) -> Result<Self> {
        let f = Fields::read(input, 131072, &[2, 2, 2, 2], &[3, 4])?;
        let result = Self {
            context: f.context()?,
            upserts: f
                .all(3)?
                .into_iter()
                .map(Binding::decode)
                .collect::<Result<_>>()?,
            forgets: f
                .all(4)?
                .into_iter()
                .map(decode_entity)
                .collect::<Result<_>>()?,
        };
        result.encode()?;
        Ok(result)
    }
}
impl Snapshot {
    pub fn encode(&self) -> Result<Vec<u8>> {
        validate_bindings(self.catalogue.context, &self.bindings)?;
        if self
            .bindings
            .iter()
            .any(|b| b.appearance_ref > self.high_water)
        {
            return malformed();
        }
        let mut out = Vec::new();
        bytes(&mut out, 1, &self.catalogue.encode()?);
        for binding in &self.bindings {
            bytes(&mut out, 2, &binding.encode()?);
        }
        uint(&mut out, 3, u64::from(self.high_water));
        bound(out, 262144)
    }
    pub fn decode(input: &[u8]) -> Result<Self> {
        let f = Fields::read(input, 262144, &[2, 2, 0], &[2])?;
        let result = Self {
            catalogue: Catalogue::decode(f.b(1)?)?,
            bindings: f
                .all(2)?
                .into_iter()
                .map(Binding::decode)
                .collect::<Result<_>>()?,
            high_water: narrow(f.u(3)?)?,
        };
        result.encode()?;
        Ok(result)
    }
}
