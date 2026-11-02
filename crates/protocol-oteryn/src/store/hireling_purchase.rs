//! Optional field6 of command185; capability250 selects Hireling customization.
use super::*;
use crate::charm_wire::{push_message_field, push_varint_field};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StoreHirelingCustomization {
    pub hireling_id: Option<[u8; 16]>,
    pub name: Option<String>,
    /// Crystal HIRELING_SEX: 1 male, 2 female. Absent for a sex-change purchase (toggle).
    pub sex: Option<u8>,
}

impl StoreHirelingCustomization {
    fn validate(&self) -> Result<(), StoreWireError> {
        if self.hireling_id.is_none() && self.name.is_none() && self.sex.is_none()
            || self
                .hireling_id
                .as_ref()
                .is_some_and(|id| !valid_uuid_v7(id))
            || self.sex.is_some_and(|sex| !(1..=2).contains(&sex))
            || self.name.as_ref().is_some_and(|name| {
                !(3..=18).contains(&name.len())
                    || !name.bytes().all(|b| b.is_ascii_alphabetic() || b == b' ')
            })
        {
            return Err(StoreWireError::Malformed);
        }
        Ok(())
    }

    pub(super) fn encode(&self) -> Result<Vec<u8>, StoreWireError> {
        self.validate()?;
        let mut out = Vec::new();
        if let Some(id) = self.hireling_id {
            push_message_field(&mut out, 1, &id)
        }
        if let Some(name) = &self.name {
            push_message_field(&mut out, 2, name.as_bytes())
        }
        if let Some(sex) = self.sex {
            push_varint_field(&mut out, 3, u64::from(sex))
        }
        Ok(out)
    }

    pub(super) fn decode(input: &[u8]) -> Result<Self, StoreWireError> {
        if input.len() > 64 {
            return Err(StoreWireError::LimitExceeded);
        }
        let mut out = Self::default();
        let mut at = 0;
        while at < input.len() {
            match read_varint(input, &mut at)? {
                10 => set_once(&mut out.hireling_id, as_uuid(read_bytes(input, &mut at)?)?)?,
                18 => set_once(
                    &mut out.name,
                    as_text(read_bytes(input, &mut at)?, 18)?.into(),
                )?,
                24 => set_once(
                    &mut out.sex,
                    u8::try_from(read_varint(input, &mut at)?)
                        .map_err(|_| StoreWireError::Malformed)?,
                )?,
                _ => return Err(StoreWireError::Malformed),
            }
        }
        out.validate()?;
        Ok(out)
    }
}

pub fn encode_hireling_purchase_request(
    request: &StorePurchaseRequest,
    customization: &StoreHirelingCustomization,
) -> Result<Vec<u8>, StoreWireError> {
    let mut out = encode_purchase_request(request)?;
    push_message_field(&mut out, 6, &customization.encode()?);
    if out.len() > MAX_STORE_REQUEST_BYTES {
        return Err(StoreWireError::LimitExceeded);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn store_hireling_customization_rejects_duplicates_unknown_fields_and_bounds() {
        let v = StoreHirelingCustomization {
            hireling_id: None,
            name: Some("Alex Moon".into()),
            sex: Some(1),
        };
        let bytes = v.encode().unwrap_or_default();
        assert_eq!(StoreHirelingCustomization::decode(&bytes), Ok(v));
        let mut duplicate = bytes.clone();
        duplicate.extend([24, 2]);
        assert!(StoreHirelingCustomization::decode(&duplicate).is_err());
        assert!(StoreHirelingCustomization::decode(&[32, 1]).is_err());
        assert!(StoreHirelingCustomization::decode(&[24, 0]).is_err());
        assert!(StoreHirelingCustomization::decode(&[24, 3]).is_err());
        assert!(StoreHirelingCustomization::decode(&[]).is_err());
        assert!(StoreHirelingCustomization::decode(&[0; 65]).is_err());
    }
}
