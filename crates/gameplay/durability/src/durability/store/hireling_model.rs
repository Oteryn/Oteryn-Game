//! Crystal 201189f GameStore customization; immutable across purchase retries.
use serde::{Deserialize, Serialize};
use super::codec::{StoreValidationError, uuid_v7};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HirelingStoreAction {
    Hire { name: String, sex: u8 },
    Unlock { entitlement_id: u16 },
    Rename { hireling_id: [u8; 16], name: String },
    SexChange { hireling_id: [u8; 16] },
}

impl HirelingStoreAction {
    pub fn validate(&self) -> Result<(), StoreValidationError> {
        match self {
            Self::Hire { name, sex } if (1..=2).contains(sex) && canonical_hireling_name(name).as_ref() == Some(name) => Ok(()),
            Self::Unlock { entitlement_id } if (1001..=1004).contains(entitlement_id) || (2001..=2009).contains(entitlement_id) => Ok(()),
            Self::Rename { hireling_id, name } if uuid_v7(hireling_id) && canonical_hireling_name(name).as_ref() == Some(name) => Ok(()),
            Self::SexChange { hireling_id } if uuid_v7(hireling_id) => Ok(()),
            _ => Err(StoreValidationError::InvalidText),
        }
    }

    pub(super) const fn kind(&self) -> i16 {
        match self { Self::Hire { .. } => 1, Self::Unlock { .. } => 2, Self::Rename { .. } => 3, Self::SexChange { .. } => 4 }
    }

    pub(super) fn name(&self) -> Option<&str> {
        match self { Self::Hire { name, .. } | Self::Rename { name, .. } => Some(name), _ => None }
    }

    pub(super) const fn target(&self) -> Option<[u8; 16]> {
        match self { Self::Rename { hireling_id, .. } | Self::SexChange { hireling_id } => Some(*hireling_id), _ => None }
    }

    pub(super) const fn entitlement(&self) -> Option<i16> {
        match self { Self::Unlock { entitlement_id } => Some(*entitlement_id as i16), _ => None }
    }

    pub(super) fn bytes(&self) -> Result<Vec<u8>, StoreValidationError> {
        self.validate()?;
        serde_json::to_vec(self).map_err(|_| StoreValidationError::InvalidBytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirelingStorePurchase {
    pub purchase_request_id: [u8; 16],
    /// Current session origin channel, revalidated independently; never part of immutable customization.
    pub origin_channel: [u8;16],
    pub action: HirelingStoreAction,
}

impl HirelingStorePurchase {
    pub fn validate(&self) -> Result<(), StoreValidationError> {
        if !uuid_v7(&self.purchase_request_id) || !uuid_v7(&self.origin_channel) { return Err(StoreValidationError::InvalidUuid); }
        self.action.validate()
    }
}

/// Crystal init.lua:1475-1528,1888-1890: ASCII letters and at most one space group.
/// Consecutive spaces and a trailing space are deliberately preserved, as in the donor.
#[must_use]
pub fn canonical_hireling_name(name: &str) -> Option<String> {
    if !(3..=18).contains(&name.len()) || name.starts_with(' ') || !name.bytes().all(|b| b.is_ascii_alphabetic() || b == b' ') { return None; }
    let groups = name.as_bytes().windows(2).filter(|w| w[0] != b' ' && w[1] == b' ').count();
    if groups > 1 { return None; }
    let compact: String = name.bytes().filter(|b| *b != b' ').map(|b| char::from(b.to_ascii_lowercase())).collect();
    const BLOCKED: &[&str] = &["owner","gamemaster","hoster","admin","staff","tibia","account","god","anal","ass","fuck","sex","hitler","pussy","dick","rape","adm","cm","gm","tutor","counsellor"];
    if BLOCKED.iter().any(|word| compact.contains(word)) { return None; }
    let mut start = true;
    Some(name.bytes().map(|b| { let next = if start { b.to_ascii_uppercase() } else { b.to_ascii_lowercase() }; start = b == b' '; char::from(next) }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hireling_names_preserve_donor_whitespace_and_title_case() {
        assert_eq!(canonical_hireling_name("aLEx  mOON"), Some("Alex  Moon".into()));
        assert_eq!(canonical_hireling_name("alex "), Some("Alex ".into()));
        for name in ["ab", " abc", "a b c", "a\tb", "Bártek", "Admin", "Ga Memaster", "Class", "abcdefghijklmnopqrs"] { assert_eq!(canonical_hireling_name(name), None, "{name}"); }
    }
    #[test]
    fn hireling_store_action_refuses_noncanonical_and_invalid_values() {
        assert!(HirelingStoreAction::Hire { name: "Alex".into(), sex: 1 }.validate().is_ok());
        assert!(HirelingStoreAction::Hire { name: "alex".into(), sex: 1 }.validate().is_err());
        assert!(HirelingStoreAction::Hire { name: "Alex".into(), sex: 0 }.validate().is_err());
        for entitlement_id in [1000,1005,2000,2010,u16::MAX] { assert!(HirelingStoreAction::Unlock { entitlement_id }.validate().is_err()); }
        assert!(HirelingStoreAction::SexChange { hireling_id: [0;16] }.validate().is_err());
    }
}

/// Non-item effects still use the existing Store receipt/outbox and purchase ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirelingServiceDeliveryPlan {
    pub identity: super::codec::DeliveryIdentity,
    pub command: crate::foundation::CommandRef,
    pub purchase: HirelingStorePurchase,
}
