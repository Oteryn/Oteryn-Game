//! Explicit, off-by-default test content. No protocol or Item admission is added.
use super::*;
use serde::Deserialize;
use std::sync::OnceLock;

pub const NPC: &str = "oteryn:npc.testserver_assistant";
pub const FLAG: &str = "OTERYN_TEST_MERCHANT";
const DATA: &str = include_str!("../../../../../../content/test-packs/test-merchant/merchant.json");

#[derive(Debug, Deserialize)]
pub struct Category {
    pub keyword: String,
    pub service: ProjectV2DefinitionRef,
}

#[derive(Debug, Deserialize)]
pub struct Placement {
    pub npc: String,
    pub x: u16,
    pub y: u16,
    pub floor: i8,
}

#[derive(Debug, Deserialize)]
pub struct Pack {
    schema: String,
    pub declarations: Vec<ProjectV2Declaration>,
    pub profiles: Vec<ProjectV2AuthoringProfile>,
    pub categories: Vec<Category>,
    pub placement: Placement,
    #[serde(skip)]
    records: BTreeMap<ProjectV2DefinitionRef, usize>,
}

pub fn enabled() -> bool {
    std::env::var(FLAG).is_ok_and(|value| value == "1")
}

pub fn digest() -> String {
    Sha256::digest(DATA.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn pack() -> Result<&'static Pack, ProjectError> {
    static PACK: OnceLock<Result<Pack, &'static str>> = OnceLock::new();
    PACK.get_or_init(|| {
        let mut pack: Pack = serde_json::from_str(DATA).map_err(|_| "test merchant pack JSON")?;
        if pack.schema != "OTERYN_TEST_MERCHANT/v1" || pack.placement.npc != NPC {
            return Err("test merchant pack schema");
        }
        for (index, declaration) in pack.declarations.iter().enumerate() {
            let (family, identity) = match declaration {
                ProjectV2Declaration::Npc { identity, .. } if identity.key == NPC => {
                    (ProjectV2Family::Npc, identity)
                }
                ProjectV2Declaration::Service {
                    identity, offers, ..
                } if identity
                    .key
                    .starts_with("oteryn:service.trade.testserver_assistant.")
                    && !offers.is_empty()
                    && offers.len() <= 1024
                    && offers.iter().all(|offer| {
                        offer.unit_price == 1
                            && offer.direction
                                == super::super::ProjectV2ServiceOfferDirection::SellToPlayer
                    }) =>
                {
                    (ProjectV2Family::Service, identity)
                }
                _ => return Err("test merchant declaration bounds"),
            };
            let reference = ProjectV2DefinitionRef {
                family,
                key: identity.key.clone(),
                revision: identity.revision.clone(),
            };
            if pack.records.insert(reference, index).is_some() {
                return Err("test merchant duplicate declaration");
            }
        }
        if pack.categories.is_empty()
            || pack
                .categories
                .iter()
                .any(|category| !pack.records.contains_key(&category.service))
        {
            return Err("test merchant categories");
        }
        Ok(pack)
    })
    .as_ref()
    .map_err(|message| ProjectError::InvalidProject(message))
}

pub fn category(text: &str) -> Option<&'static Category> {
    pack()
        .ok()?
        .categories
        .iter()
        .find(|category| category.keyword.eq_ignore_ascii_case(text.trim()))
}

pub fn contains(keyword: Option<&str>, offer: &super::super::ProjectV2ServiceOffer) -> bool {
    let Some(category) = keyword.and_then(category) else {
        return false;
    };
    let Ok(pack) = pack() else {
        return false;
    };
    let Some(index) = pack.records.get(&category.service) else {
        return false;
    };
    matches!(&pack.declarations[*index], ProjectV2Declaration::Service { offers, .. } if offers.contains(offer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merchant_pack_has_bounded_services_and_source_outfit() -> Result<(), ProjectError> {
        let pack = pack()?;
        assert_eq!(
            pack.records
                .keys()
                .filter(|reference| reference.family == ProjectV2Family::Npc)
                .count(),
            1
        );
        assert_eq!(pack.categories.len() + 1, pack.declarations.len());
        let reference = pack
            .records
            .keys()
            .find(|reference| reference.key == NPC)
            .ok_or(ProjectError::InvalidProject("test merchant NPC missing"))?;
        let ProjectV2Declaration::Npc {
            presentation: Some(presentation),
            ..
        } = &pack.declarations[pack.records[reference]]
        else {
            return Err(ProjectError::InvalidProject(
                "test merchant presentation missing",
            ));
        };
        let profile = pack
            .profiles
            .iter()
            .find(|profile| &profile.target == presentation)
            .ok_or(ProjectError::InvalidProject(
                "test merchant profile missing",
            ))?;
        let crate::content::ProjectV2AuthoringProfileData::Presentation(profile) = &profile.data
        else {
            return Err(ProjectError::InvalidProject("test merchant profile type"));
        };
        assert_eq!(
            profile.asset_binding.as_deref(),
            Some("canary.appearance:outfit/396")
        );
        assert_eq!(profile.palette_bindings.len(), 4);
        assert!(
            profile
                .palette_bindings
                .iter()
                .all(|binding| binding.asset_binding == "canary.appearance:palette/0")
        );
        let pin: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../content/world/pins/oteryn.json"
        ))
        .map_err(|_| ProjectError::InvalidProject("test merchant default start"))?;
        assert_eq!(
            Some(u64::from(pack.placement.x)),
            pin["entry_start"]["x"]
                .as_u64()
                .and_then(|x| x.checked_add(1))
        );
        assert_eq!(
            Some(u64::from(pack.placement.y)),
            pin["entry_start"]["y"].as_u64()
        );
        assert_eq!(
            Some(i64::from(pack.placement.floor)),
            pin["entry_start"]["floor"].as_i64()
        );
        Ok(())
    }
}
