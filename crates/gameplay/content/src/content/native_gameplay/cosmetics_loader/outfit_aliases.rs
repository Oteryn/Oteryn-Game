//! Explicit promoted identity aliases preserve existing grants and one wardrobe entry.
use super::{CosmeticsDiagnostic, Outfit, Result};
use std::collections::BTreeMap;

pub(super) fn deserialize<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> std::result::Result<BTreeMap<String, String>, D::Error> {
    struct UniqueAliases;
    impl<'de> serde::de::Visitor<'de> for UniqueAliases {
        type Value = BTreeMap<String, String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("at most 512 unique outfit aliases")
        }

        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> std::result::Result<Self::Value, M::Error> {
            use serde::de::Error;
            let mut aliases = BTreeMap::new();
            while let Some((alias, canonical)) = map.next_entry::<String, String>()? {
                if aliases.len() == 512 || aliases.insert(alias, canonical).is_some() {
                    return Err(M::Error::custom("duplicate or excessive outfit aliases"));
                }
            }
            Ok(aliases)
        }
    }
    deserializer.deserialize_map(UniqueAliases)
}

pub(super) fn bind(
    rows: &[Outfit],
    aliases: &BTreeMap<String, String>,
    index: &mut BTreeMap<String, usize>,
) -> Result<()> {
    let invalid = || CosmeticsDiagnostic::Definition {
        family: "OutfitAlias",
        index: 0,
    };
    if aliases.len() > rows.len() {
        return Err(invalid());
    }
    let mut bindings = Vec::new();
    for (alias, target) in aliases {
        if aliases.contains_key(target) {
            return Err(invalid());
        }
        let old = &rows[*index.get(alias).ok_or_else(invalid)?];
        let target_index = *index.get(target).ok_or_else(invalid)?;
        let canonical = &rows[target_index];
        if old.female.id != canonical.female.id
            || old.female.source_record_sha256 != canonical.female.source_record_sha256
            || old.male.id != canonical.male.id
            || old.male.source_record_sha256 != canonical.male.source_record_sha256
            || old.addon_count != canonical.addon_count
            || old.starter_female != canonical.starter_female
            || old.starter_male != canonical.starter_male
            || old.starter_addons_female != canonical.starter_addons_female
            || old.starter_addons_male != canonical.starter_addons_male
            || (old.premium && !canonical.premium)
        {
            return Err(invalid());
        }
        bindings.push((alias.clone(), target_index));
    }
    for (alias, target) in bindings {
        index.insert(alias, target);
    }
    Ok(())
}
