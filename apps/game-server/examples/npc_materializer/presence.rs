//! NPC-CONTENT-2: source movement facts and admitted voices on NPC definitions (NPC-BEHAVIOUR0
//! §5, §6). Adds candidate fields only; no actor, think or voice is activated.
use super::*;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub(super) const FIELDS: [&str; 5] = [
    "oteryn:source.npc.base_speed",
    "oteryn:source.npc.floor_change",
    "oteryn:source.npc.voice_chance_percent",
    "oteryn:source.npc.voice_interval_ms",
    "oteryn:source.npc.voice_lines",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet {
    schema: String,
    from_project_revision: String,
    project_revision: String,
    #[allow(dead_code, reason = "provenance, read by reviewers")]
    sources: Value,
    #[allow(dead_code, reason = "provenance, read by reviewers")]
    defaults: Value,
    #[allow(dead_code, reason = "provenance, read by reviewers")]
    counts: Value,
    npcs: Vec<Row>,
    #[allow(dead_code, reason = "provenance, read by reviewers")]
    held: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    key: String,
    fields: Vec<ProjectV2CandidateField>,
}

/// Adds the staged fields to their NPC definitions and mints the packet's project revision.
/// Returns the number of NPC definitions changed.
pub(super) fn apply(
    draft: &mut ProjectV2Draft,
    bytes: &[u8],
    packet_sha256: &str,
) -> Result<usize> {
    if hex_sha256(bytes) != packet_sha256 {
        return Err("NPC presence packet digest drifted".into());
    }
    let packet: Packet = serde_json::from_slice(bytes)?;
    if packet.schema != "OTERYN_NPC_PRESENCE_STAGE/v1"
        || packet.from_project_revision != draft.core.project_revision
        || packet.project_revision == packet.from_project_revision
    {
        return Err("NPC presence envelope drifted".into());
    }
    let mut rows = BTreeMap::new();
    for row in packet.npcs {
        if row.fields.is_empty()
            || row
                .fields
                .iter()
                .any(|field| !FIELDS.contains(&field.field_path.as_str()))
        {
            return Err("NPC presence row carries an unknown field".into());
        }
        if rows.insert(row.key, row.fields).is_some() {
            return Err("duplicate NPC presence row".into());
        }
    }
    let staged = rows.len();
    for declaration in &mut draft.state.declarations {
        let ProjectV2Declaration::Npc {
            identity, fields, ..
        } = declaration
        else {
            continue;
        };
        let Some(added) = rows.remove(&identity.key) else {
            continue;
        };
        if fields
            .iter()
            .any(|field| FIELDS.contains(&field.field_path.as_str()))
        {
            return Err("NPC definition already carries presence fields".into());
        }
        fields.extend(added);
        fields.sort_by(|left, right| left.field_path.cmp(&right.field_path));
    }
    if !rows.is_empty() {
        return Err("NPC presence row has no NPC definition".into());
    }
    draft.core.project_revision = packet.project_revision;
    Ok(staged)
}
