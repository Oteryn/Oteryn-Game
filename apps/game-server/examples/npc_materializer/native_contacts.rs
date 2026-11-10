//! Add source-bound passive contacts after the immutable historical NPC stages.
use oteryn_game_server::content::{
    ProjectV2CandidateValue, ProjectV2Declaration, ProjectV2Family, ProjectV2Source,
    ProjectV2SourceIdentityBinding, ProjectV2SourceIdentityDisposition,
};
use serde::Deserialize;
use std::collections::BTreeSet;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const PACKET: &str = include_str!(
    "../../../../tools/content-schema/npc-authoring/samples/native-contact-definitions.json"
);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet {
    schema: String,
    source: ProjectV2Source,
    records: Vec<Contact>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Contact {
    declaration: ProjectV2Declaration,
    source_bindings: Vec<ProjectV2SourceIdentityBinding>,
}

pub(super) fn apply(
    records: &mut Vec<ProjectV2Declaration>,
    bindings: &mut Vec<ProjectV2SourceIdentityBinding>,
    sources: &[ProjectV2Source],
) -> Result<usize> {
    apply_packet(records, bindings, sources, serde_json::from_str(PACKET)?)
}

fn apply_packet(
    records: &mut Vec<ProjectV2Declaration>,
    bindings: &mut Vec<ProjectV2SourceIdentityBinding>,
    sources: &[ProjectV2Source],
    packet: Packet,
) -> Result<usize> {
    if packet.schema != "OTERYN_NATIVE_CONTACT_NPCS/v1"
        || !(1..=16).contains(&packet.records.len())
        || !sources.contains(&packet.source)
    {
        return Err("native contact packet/source mismatch".into());
    }
    let mut keys = BTreeSet::new();
    let mut declarations = Vec::new();
    let mut additions = Vec::new();
    for row in packet.records {
        let ProjectV2Declaration::Npc {
            identity,
            visibility,
            presentation,
            behavior,
            dialogue,
            services,
            fields,
        } = &row.declaration
        else {
            return Err("native contact must be an NPC".into());
        };
        if !visibility.is_public()
            || !keys.insert(identity.key.clone())
            || presentation
                .as_ref()
                .is_none_or(|r| r.family != ProjectV2Family::Presentation)
            || behavior
                .as_ref()
                .is_none_or(|r| r.family != ProjectV2Family::Behavior)
            || dialogue.is_some()
            || !services.is_empty()
            || fields.iter().any(|f| {
                f.field_path != "oteryn:source.npc.display_name"
                    || !matches!(&f.value, ProjectV2CandidateValue::Text(name) if !name.is_empty())
            })
            || fields.len() > 1
        {
            return Err("native contact shape or identity mismatch".into());
        }
        let [binding] = row.source_bindings.as_slice() else {
            return Err("native contact requires one exact BR page".into());
        };
        if binding.source_key != packet.source.key
            || binding.source_revision != packet.source.revision
            || binding.identity_namespace != "mediawiki/page_id"
            || binding
                .external_id
                .parse::<u32>()
                .ok()
                .is_none_or(|id| id == 0)
            || binding.disposition != ProjectV2SourceIdentityDisposition::Exact
            || binding.target.family != ProjectV2Family::Npc
            || binding.target.key != identity.key
            || binding.target.revision != identity.revision
        {
            return Err("native contact page target mismatch".into());
        }
        let existing: Vec<_> = records
            .iter()
            .filter(|r| {
                matches!(r,
            ProjectV2Declaration::Npc { identity: found, .. } if found.key == identity.key)
            })
            .collect();
        match existing.as_slice() {
            [] => declarations.push(row.declaration.clone()),
            [found] if **found == row.declaration => {}
            _ => return Err("native contact declaration drift".into()),
        }
        let existing: Vec<_> = bindings
            .iter()
            .filter(|b| {
                (b.target.family == ProjectV2Family::Npc && b.target.key == identity.key)
                    || (b.source_key == binding.source_key
                        && b.source_revision == binding.source_revision
                        && b.identity_namespace == binding.identity_namespace
                        && b.external_id == binding.external_id)
            })
            .collect();
        match existing.as_slice() {
            [] => additions.push(binding.clone()),
            [found] if **found == *binding => {}
            _ => return Err("native contact binding drift".into()),
        }
    }
    let added = declarations.len();
    records.extend(declarations);
    bindings.extend(additions);
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet() -> Result<Packet> {
        Ok(serde_json::from_str(PACKET)?)
    }

    #[test]
    fn m38_native_spy_rat_is_additive_source_bound_and_idempotent() -> Result<()> {
        let packet = packet()?;
        let source = packet.source.clone();
        let mut records = Vec::new();
        let mut bindings = Vec::new();
        assert_eq!(
            apply(&mut records, &mut bindings, std::slice::from_ref(&source))?,
            1
        );
        assert_eq!(records, [packet.records[0].declaration.clone()]);
        assert_eq!(bindings, packet.records[0].source_bindings);
        let before = (records.clone(), bindings.clone());
        assert_eq!(apply(&mut records, &mut bindings, &[source])?, 0);
        assert_eq!((records, bindings), before);
        Ok(())
    }

    #[test]
    fn m38_native_contact_refuses_drift_without_partial_mutation() -> Result<()> {
        for change in 0..5 {
            let mut packet = packet()?;
            let source = packet.source.clone();
            let mut records = Vec::new();
            let mut bindings = Vec::new();
            match change {
                0 => packet.source.sha256 = "0".repeat(64),
                1 => packet.records[0].source_bindings[0].target.key = "oteryn:npc.other".into(),
                2 => {
                    let mut other = packet.records[0].source_bindings[0].clone();
                    other.target.key = "oteryn:npc.other".into();
                    bindings.push(other);
                }
                4 => {
                    let ProjectV2Declaration::Npc { visibility, .. } =
                        &mut packet.records[0].declaration
                    else {
                        return Err("NPC".into());
                    };
                    *visibility = serde_json::from_value(serde_json::json!("hidden"))?;
                }
                _ => {
                    let mut altered = packet.records[0].declaration.clone();
                    let ProjectV2Declaration::Npc { fields, .. } = &mut altered else {
                        return Err("NPC".into());
                    };
                    fields.clear();
                    records.push(altered);
                }
            }
            let before = (records.clone(), bindings.clone());
            assert!(apply_packet(&mut records, &mut bindings, &[source], packet).is_err());
            assert_eq!((records, bindings), before);
        }
        Ok(())
    }
}
