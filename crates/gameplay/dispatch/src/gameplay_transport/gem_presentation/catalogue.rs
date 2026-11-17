use super::*;
use oteryn_gameplay_progression::wheel_effects::{basic_milli, supreme_effect};
use std::collections::BTreeSet;
fn unit(s: &str) -> Option<u8> {
    match s {
        "percent_points" => Some(0),
        "flat" => Some(1),
        "seconds" => Some(2),
        "points" => Some(3),
        _ => None,
    }
}
pub(super) fn project(data: &WheelGemData, vocation: &str) -> Option<Vec<wire::Modifier>> {
    let family = &data.gems()["vocations"][vocation];
    let catalogue = &data.gems()["catalogue"];
    let mut ids = BTreeSet::new();
    for (kind, key) in [
        (0, "basic_mods_position_1"),
        (0, "basic_mods_position_2"),
        (1, "supreme_mods"),
    ] {
        for id in family[key].as_array()? {
            ids.insert((kind, u16::try_from(id.as_u64()?).ok()?));
        }
    }
    if ids.is_empty() || ids.len() > wire::MAX_MODIFIERS {
        return None;
    }
    ids.into_iter()
        .map(|(kind, id)| {
            let row = catalogue[if kind == 0 {
                "basic_mods"
            } else {
                "supreme_mods"
            }]
            .as_array()?
            .iter()
            .find(|r| r["source_id"].as_u64() == Some(u64::from(id)))?;
            let mut effects = Vec::new();
            if kind == 0 {
                for e in row["effects"].as_array()? {
                    let mut values = [0; 4];
                    for grade in 0..4 {
                        values[usize::from(grade)] = basic_milli(id, grade, vocation, e)?;
                    }
                    effects.push(wire::Effect {
                        name: e["name"].as_str()?.into(),
                        target: String::new(),
                        unit: unit(e["unit"].as_str()?)?,
                        values,
                    });
                }
            } else {
                for (position, first) in row["grades"][0]["numeric_effects"]
                    .as_array()?
                    .iter()
                    .enumerate()
                {
                    let first = supreme_effect(id, 0, first)?;
                    let mut values = [0; 4];
                    let expected_unit =
                        unit(row["grades"][0]["numeric_effects"][position]["unit"].as_str()?)?;
                    for grade in 0..4 {
                        let raw = &row["grades"][usize::from(grade)]["numeric_effects"][position];
                        let value = supreme_effect(id, grade, raw)?;
                        if value.kind != first.kind
                            || value.target != first.target
                            || unit(raw["unit"].as_str()?)? != expected_unit
                        {
                            return None;
                        }
                        values[usize::from(grade)] = value.value_milli;
                    }
                    effects.push(wire::Effect {
                        name: first.kind.into(),
                        target: first.target.into(),
                        unit: expected_unit,
                        values,
                    });
                }
            }
            let name = if kind == 0 {
                effects
                    .iter()
                    .map(|e| e.name.as_str())
                    .collect::<Vec<_>>()
                    .join(" / ")
            } else {
                row["summary_name"].as_str()?.into()
            };
            Some(wire::Modifier {
                kind,
                id,
                name,
                effects,
            })
        })
        .collect()
}
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
