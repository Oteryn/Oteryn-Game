//! Names preserved from npcfix's checked retail snapshot; keys and all other facts stay intact.
//! Evidence: docs/reference/content-prep/npcfix-m38/preserved-display-names.md.
use oteryn_game_server::content::{
    ProjectV2CandidateField, ProjectV2CandidateValue, ProjectV2Declaration,
};

const FIELD: &str = "oteryn:source.npc.display_name";
const NAMES: &[(&str, &str)] = &[
    ("oteryn:npc.ferryman_kamil_meluna", "Ferryman Kamil"),
    ("oteryn:npc.harlow_vengoth", "Harlow"),
    ("oteryn:npc.junkar_mines", "Junkar"),
    ("oteryn:npc.maris_mistrock", "Maris"),
];

pub(super) fn apply(
    records: &mut [ProjectV2Declaration],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pending = Vec::new();
    for &(key, name) in NAMES {
        let matches: Vec<_> = records
            .iter()
            .enumerate()
            .filter_map(|(index, record)| match record {
                ProjectV2Declaration::Npc {
                    identity, fields, ..
                } if identity.key == key => Some((index, identity, fields)),
                _ => None,
            })
            .collect();
        let [(index, identity, fields)] = matches.as_slice() else {
            return Err(format!("preserved display name requires exactly one NPC: {key}").into());
        };
        let names: Vec<_> = fields
            .iter()
            .filter(|field| field.field_path == FIELD)
            .collect();
        if identity.revision != "definition-r1"
            || names.len() > 1
            || names
                .first()
                .is_some_and(|field| field.value != ProjectV2CandidateValue::Text(name.into()))
        {
            return Err(format!("preserved display name predecessor drift: {key}").into());
        }
        if names.is_empty() {
            pending.push((*index, name));
        }
    }
    for (index, name) in pending {
        let ProjectV2Declaration::Npc { fields, .. } = &mut records[index] else {
            unreachable!()
        };
        fields.push(ProjectV2CandidateField {
            field_path: FIELD.into(),
            value: ProjectV2CandidateValue::Text(name.into()),
        });
        fields.sort_by(|a, b| a.field_path.cmp(&b.field_path));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn records() -> Result<Vec<ProjectV2Declaration>, Box<dyn std::error::Error>> {
        NAMES.iter().chain([&( "oteryn:npc.unrelated", "Unrelated")]).map(|(key, _)| {
            Ok(serde_json::from_value(serde_json::json!({
                "kind":"NPC", "identity":{"key":key,"revision":"definition-r1"},
                "presentation":null,"behavior":null,"dialogue":null,"services":[],
                "fields":[{"field_path":"oteryn:source.keep","value":{"type":"Text","value":"unchanged"}}]
            }))?)
        }).collect()
    }

    #[test]
    fn npcfix_preserved_names_are_additive_and_idempotent() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut records = records()?;
        let before = records.clone();
        apply(&mut records)?;
        for ((_, name), (before, after)) in NAMES.iter().zip(before.iter().zip(&records)) {
            let mut expected = before.clone();
            let ProjectV2Declaration::Npc { fields, .. } = &mut expected else {
                return Err("NPC fixture".into());
            };
            fields.push(ProjectV2CandidateField {
                field_path: FIELD.into(),
                value: ProjectV2CandidateValue::Text((*name).into()),
            });
            fields.sort_by(|a, b| a.field_path.cmp(&b.field_path));
            assert_eq!(after, &expected);
        }
        assert_eq!(records.last(), before.last());
        let once = records.clone();
        apply(&mut records)?;
        assert_eq!(records, once);
        Ok(())
    }

    #[test]
    fn npcfix_preserved_names_refuse_drift_atomically() -> Result<(), Box<dyn std::error::Error>> {
        for duplicate in [false, true] {
            let mut records = records()?;
            if duplicate {
                records.push(records[3].clone());
            } else {
                records.pop();
                records.pop();
            }
            let before = records.clone();
            assert!(apply(&mut records).is_err());
            assert_eq!(records, before);
        }
        let mut records = records()?;
        let ProjectV2Declaration::Npc { fields, .. } = &mut records[3] else {
            return Err("NPC fixture".into());
        };
        fields.push(ProjectV2CandidateField {
            field_path: FIELD.into(),
            value: ProjectV2CandidateValue::Text("Other name".into()),
        });
        let before = records.clone();
        assert!(apply(&mut records).is_err());
        assert_eq!(records, before);
        Ok(())
    }
}
