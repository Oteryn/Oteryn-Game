// Immutable qualification content is prepared before admission, never between frames.
use crate::content::native_gameplay::NativeGameplayInput;
use std::sync::OnceLock;

pub(super) fn native_input() -> TestResult<&'static NativeGameplayInput> {
    static INPUT: OnceLock<Result<NativeGameplayInput, String>> = OnceLock::new();
    INPUT
        .get_or_init(|| {
            let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            NativeGameplayInput::from_manifest(&root.join("content/spells.manifest.json"))
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| error.clone().into())
}

pub(super) fn native_spell_book() -> TestResult<&'static crate::spell::SpellBook> {
    static BOOK: OnceLock<Result<crate::spell::SpellBook, String>> = OnceLock::new();
    BOOK.get_or_init(|| {
        let compile = || -> TestResult<crate::spell::SpellBook> {
            let input = native_input()?;
            let catalog = crate::spell::executable_catalog::compile(
                &input.catalog.bytes,
                &input.catalog.sha256,
            )?;
            let selections = crate::spell::executable_catalog::compile_source_selection(
                &input.source_selection.bytes,
                &input.source_selection.sha256,
                &catalog,
            )?;
            Ok(catalog.into_spell_book_with_selections(&selections)?)
        };
        compile().map_err(|error| error.to_string())
    })
    .as_ref()
    .map_err(|error| error.clone().into())
}

#[cfg(test)]
mod native_content_tests {
    use super::*;

    #[test]
    fn qualification_native_content_reuses_prewarmed_instances() -> TestResult {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| -> Result<(), String> {
                let input = native_input().map_err(|error| error.to_string())?;
                let book = native_spell_book().map_err(|error| error.to_string())?;
                assert!(std::ptr::eq(
                    input,
                    native_input().map_err(|error| error.to_string())?
                ));
                assert!(std::ptr::eq(
                    book,
                    native_spell_book().map_err(|error| error.to_string())?
                ));
                assert!(book.spoken("utani hur").is_some());
                Ok(())
            })?
            .join()
            .map_err(|_| "qualification content worker panicked")??;
        Ok(())
    }
}

/// Match served creature names against the same pinned native profiles as the server.
pub(super) fn native_creature_name(key: &str) -> TestResult<&'static str> {
    use crate::content::ProjectV2AuthoringProfileData;
    use crate::content::native_gameplay::CreatureProfilesDocument;
    static NAMES: OnceLock<Result<std::collections::BTreeMap<String, String>, String>> =
        OnceLock::new();
    let names = NAMES
        .get_or_init(|| {
            (|| -> TestResult<std::collections::BTreeMap<String, String>> {
                let profiles: CreatureProfilesDocument =
                    serde_json::from_slice(&native_input()?.creature_profiles.bytes)?;
                Ok(profiles
                    .records
                    .into_iter()
                    .filter_map(|record| {
                        if let ProjectV2AuthoringProfileData::Creature(profile) =
                            record.profile.data
                        {
                            profile
                                .details
                                .map(|details| (record.profile.target.key, details.display_name))
                        } else {
                            None
                        }
                    })
                    .collect())
            })()
            .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| error.clone())?;
    names
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("native creature display name unavailable: {key}").into())
}

pub(super) fn native_food_references()
-> TestResult<&'static std::collections::BTreeSet<std::num::NonZeroU32>> {
    use crate::content::{ReferenceItemConsumption, ReferenceItemField};
    static FOODS: OnceLock<Result<std::collections::BTreeSet<std::num::NonZeroU32>, String>> =
        OnceLock::new();
    FOODS
        .get_or_init(|| {
            (|| -> TestResult<std::collections::BTreeSet<std::num::NonZeroU32>> {
                let input = native_input()?;
                let bytes = &input
                    .item_profiles
                    .as_ref()
                    .ok_or("native item profiles absent")?
                    .bytes;
                let profiles: crate::content::native_gameplay::ItemProfilesDocument =
                    serde_json::from_slice(bytes)?;
                let index = provisioning::item_index()?;
                profiles
                    .records
                    .iter()
                    .filter(|record| {
                        matches!(
                            record.semantics.consumption,
                            ReferenceItemField::Known(ReferenceItemConsumption::Food(_))
                        )
                    })
                    .map(|record| {
                        index
                            .definition_ref(
                                &record.production_definition.family,
                                &record.production_definition.production_key,
                                &record.production_definition.revision_ref,
                            )
                            .ok_or_else(|| {
                                format!(
                                    "native food reference absent: {}",
                                    record.production_definition.production_key
                                )
                                .into()
                            })
                    })
                    .collect()
            })()
            .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|error| error.clone().into())
}
