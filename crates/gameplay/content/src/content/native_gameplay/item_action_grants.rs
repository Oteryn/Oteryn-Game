//! SRVGAP-C4: explicit source grant compatibility for already admitted cosmetic definitions.
//! Crystal 201189f usable_outfit_items.lua:71-89 grants both looks; usable_mount_items.lua:30
//! grants the mount from mounts.xml. Numeric membership never creates a new catalogue.
use super::NativeCosmeticsView;
use crate::domain::appearance::{AppearanceContent, EarnedCosmetic, Sex, UnlockFact};
use serde_json::Value;
use std::sync::OnceLock;
const PROVENANCE: &str = "crystal-item-cosmetics-201189f-v1";
fn grants() -> Option<&'static Vec<Value>> {
    static SOURCE: OnceLock<Option<Value>> = OnceLock::new();
    let source = SOURCE
        .get_or_init(|| {
            serde_json::from_str(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../tools/content-schema/native-gameplay/item-cosmetic-actions.json"
            )))
            .ok()
        })
        .as_ref()?;
    if source["crystal_commit"] != "201189f871d23597b82caf88fd611454a2ddefb8"
        || source["provenance"] != PROVENANCE
    {
        return None;
    }
    source["grants"].as_array()
}
pub(super) fn keys() -> impl Iterator<Item = String> {
    grants()
        .into_iter()
        .flatten()
        .filter_map(|g| g["key"].as_str().map(str::to_owned))
}
pub(super) fn resolve(
    view: &NativeCosmeticsView,
    key: &str,
    provenance: &str,
    target: EarnedCosmetic<'_>,
) -> UnlockFact {
    let Some(grant) = grants().into_iter().flatten().find(|g| g["key"] == key) else {
        return UnlockFact::Absent;
    };
    let matches = match target {
        EarnedCosmetic::Outfit(outfit) | EarnedCosmetic::Addon(outfit, _) => {
            let same_looks = [Sex::Female, Sex::Male].iter().all(|sex| {
                view.look_type(outfit, *sex).is_some_and(|look| {
                    grant["look_types"]
                        .as_array()
                        .is_some_and(|looks| looks.contains(&Value::from(look)))
                })
            });
            same_looks
                && view.look_type(outfit, Sex::Female) != view.look_type(outfit, Sex::Male)
                && match target {
                    EarnedCosmetic::Outfit(_) => grant["kind"] == "outfit",
                    EarnedCosmetic::Addon(_, addon) => {
                        grant["kind"] == "addon"
                            && grant["addon"] == addon
                            && view
                                .outfit_addon_count(&outfit.to_owned())
                                .is_some_and(|n| addon <= n)
                    }
                    _ => false,
                }
        }
        EarnedCosmetic::Mount(mount) => {
            grant["kind"] == "mount"
                && view
                    .mount_metadata(mount)
                    .is_some_and(|(look, _, _)| grant["look_type"] == look)
        }
    };
    if !matches {
        UnlockFact::Absent
    } else if provenance == PROVENANCE {
        UnlockFact::Compatible
    } else {
        UnlockFact::Incompatible
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[allow(
        clippy::expect_used,
        reason = "fixed source-bound cosmetics fixture must decode"
    )]
    fn item_receipts_authorize_only_the_matching_admitted_look_and_provenance() {
        use crate::content::native_gameplay::test_support::cosmetics_fixture::{
            cosmetics_test_packet, decode_cosmetics,
        };
        let view = decode_cosmetics(&serde_json::to_vec(&cosmetics_test_packet()).expect("packet"))
            .expect("view");
        let outfit = view.outfit_keys().next().expect("outfit");
        let key = "oteryn:item-grant.outfit.128-136.addon2";
        assert!(view.earned_grant_keys().iter().any(|g| g == key));
        assert_eq!(
            view.earned_unlock(key, PROVENANCE, EarnedCosmetic::Addon(outfit, 2)),
            UnlockFact::Compatible
        );
        assert_eq!(
            view.earned_unlock(key, "other-revision", EarnedCosmetic::Addon(outfit, 2)),
            UnlockFact::Incompatible
        );
        assert_eq!(
            view.earned_unlock(key, PROVENANCE, EarnedCosmetic::Addon(outfit, 1)),
            UnlockFact::Absent
        );
        assert_eq!(
            view.earned_unlock(key, PROVENANCE, EarnedCosmetic::Outfit(outfit)),
            UnlockFact::Absent
        );
        assert_eq!(
            view.earned_unlock(
                "oteryn:item-grant.outfit.514-516.addon2",
                PROVENANCE,
                EarnedCosmetic::Addon(outfit, 2)
            ),
            UnlockFact::Absent
        );
    }
}
