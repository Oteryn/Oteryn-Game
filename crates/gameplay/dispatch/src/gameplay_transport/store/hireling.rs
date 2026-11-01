//! Hireling offers use the ordinary Store purchase/receipt path, with frozen Game customization.
use super::catalogue::HirelingOffer;
use crate::durability::store::{HirelingStoreAction,canonical_hireling_name};
use oteryn_protocol_oteryn::store::StoreHirelingCustomization;

pub(super) fn action(offer:&HirelingOffer,customization:Option<&StoreHirelingCustomization>)->Option<HirelingStoreAction> {
    let value=match (offer.effect.as_str(),customization) {
        ("hire",Some(c)) if c.hireling_id.is_none()=>HirelingStoreAction::Hire {name:canonical_hireling_name(c.name.as_deref()?)?,sex:c.sex?},
        ("unlock",None)=>HirelingStoreAction::Unlock {entitlement_id:offer.source_id},
        ("rename",Some(c)) if c.sex.is_none()=>HirelingStoreAction::Rename {hireling_id:c.hireling_id?,name:canonical_hireling_name(c.name.as_deref()?)?},
        ("sex_change",Some(c)) if c.name.is_none() && c.sex.is_none()=>HirelingStoreAction::SexChange {hireling_id:c.hireling_id?},
        _=>return None,
    };
    value.validate().ok()?;
    Some(value)
}

pub(super) fn request_binding(base:[u8;32],action:Option<&HirelingStoreAction>)->Option<Vec<u8>> {
    let mut bytes=base.to_vec();
    if let Some(action)=action {
        let customization=serde_json::to_vec(action).ok()?;
        bytes.extend(b"\0hireling-v1\0");
        bytes.extend(u32::try_from(customization.len()).ok()?.to_be_bytes());
        bytes.extend(customization);
    }
    Some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hireling_store_rejects_customization_for_another_offer_kind() {
        let hire=HirelingOffer{effect:"hire".into(),source_id:25440};
        let c=StoreHirelingCustomization{hireling_id:None,name:Some("aLeX".into()),sex:Some(1)};
        assert_eq!(action(&hire,Some(&c)),Some(HirelingStoreAction::Hire{name:"Alex".into(),sex:1}));
        assert!(action(&hire,None).is_none());
        let unlock=HirelingOffer{effect:"unlock".into(),source_id:1001};
        assert!(action(&unlock,Some(&c)).is_none());
        assert_eq!(action(&unlock,None),Some(HirelingStoreAction::Unlock{entitlement_id:1001}));
    }
    #[test]
    fn hireling_store_retry_binding_includes_normalized_customization() {
        let a=HirelingStoreAction::Hire{name:"Alex".into(),sex:1};
        let b=HirelingStoreAction::Hire{name:"Alex".into(),sex:2};
        assert_ne!(request_binding([0;32],Some(&a)),request_binding([0;32],Some(&b)));
        assert_eq!(request_binding([0;32],None),Some(vec![0;32]));
    }
}
