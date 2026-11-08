//! Source-authored quest recipes use the same atomic carried consume/mint owner as cooking.
use super::*;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipe {
    key: String,
    ingredients: [u32; 2],
    outputs: Vec<(u32, u32)>,
    source: String,
}

pub(super) fn carried(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    target: &TypedDefinitionRef,
) -> Option<CookingConversion> {
    let used = source_ref(native, used)?;
    let target = source_ref(native, target)?;
    let id = |key: &str| key.strip_prefix("oteryn:item.tibia.i")?.parse::<u32>().ok();
    if used.family != ProjectV2Family::Item
        || target.family != ProjectV2Family::Item
        || used.revision != "definition-r1"
        || target.revision != "definition-r1"
    {
        return None;
    }
    let pair = [id(&used.key)?, id(&target.key)?];
    static ROWS: OnceLock<Option<Vec<Recipe>>> = OnceLock::new();
    let rows = ROWS
        .get_or_init(|| {
            serde_json::from_str(include_str!(
                "../../../../../content/quests/carried-recipes.json"
            ))
            .ok()
        })
        .as_ref()?;
    let mut matching = rows
        .iter()
        .filter(|r| r.ingredients == pair || r.ingredients == [pair[1], pair[0]]);
    let row = matching.next()?;
    if matching.next().is_some()
        || row.outputs.is_empty()
        || row.outputs.len() > 4
        || !row
            .source
            .starts_with("crystal:201189f871d23597b82caf88fd611454a2ddefb8:")
    {
        return None;
    }
    let outputs = row
        .outputs
        .iter()
        .map(|(id, count)| {
            if !(1..=100).contains(count) {
                return None;
            }
            Some((
                output_definition(
                    native,
                    &ProjectV2DefinitionRef {
                        family: ProjectV2Family::Item,
                        key: format!("oteryn:item.tibia.i{id}"),
                        revision: "definition-r1".into(),
                    },
                )?,
                *count,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(CookingConversion {
        recipe: row.key.clone(),
        outputs,
    })
}
