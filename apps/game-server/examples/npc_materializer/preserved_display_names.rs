//! Display names from source snapshots and registered NPC configurations.
//! Evidence: NPCFIX-M38 and CNPC2 under docs/reference/content-prep.
use oteryn_game_server::content::{
    ProjectV2CandidateField, ProjectV2CandidateValue, ProjectV2Declaration,
};

const FIELD: &str = "oteryn:source.npc.display_name";
const NAMES: &[(&str, &str)] = &[
    ("oteryn:npc.anaztassja_moroia_init", "Anaztassja Moroia"),
    ("oteryn:npc.baa_leal", "Baa'Leal"),
    ("oteryn:npc.bo_ques", "Bo'Ques"),
    ("oteryn:npc.buddel_helheim", "Buddel"),
    ("oteryn:npc.buddel_okolnir", "Buddel"),
    ("oteryn:npc.buddel_raider_camp", "Buddel"),
    ("oteryn:npc.buddel_tyrsung", "Buddel"),
    ("oteryn:npc.captain_max_calassa", "Captain Max"),
    ("oteryn:npc.fa_hradin", "Fa'Hradin"),
    ("oteryn:npc.ferryman_kamil_meluna", "Ferryman Kamil"),
    ("oteryn:npc.gnomish_operative_catching", "Gnomish Operative"),
    ("oteryn:npc.gnomish_operative_gt_m_01", "Gnomish Operative"),
    ("oteryn:npc.gnomish_operative_raiders", "Gnomish Operative"),
    (
        "oteryn:npc.gnomish_operative_resonating",
        "Gnomish Operative",
    ),
    ("oteryn:npc.gnomish_operative_volcano", "Gnomish Operative"),
    ("oteryn:npc.harlow_vengoth", "Harlow"),
    ("oteryn:npc.jack_fate_goroma", "Jack Fate"),
    ("oteryn:npc.john_bounac", "John"),
    ("oteryn:npc.junkar_mines", "Junkar"),
    ("oteryn:npc.lesh_the_seer", "Lesh the Seer"),
    ("oteryn:npc.maris_mistrock", "Maris"),
    ("oteryn:npc.nah_bob", "Nah'Bob"),
    ("oteryn:npc.one_eyed_joe", "One-Eyed Joe"),
    ("oteryn:npc.rata_mari", "Rata'Mari"),
    ("oteryn:npc.sebastian_nargor", "Sebastian"),
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
