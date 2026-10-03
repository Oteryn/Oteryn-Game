//! Source-qualified repairs under the owner-requested D10 Fandom extension.
//! Source-bound whole D9 static programs; quest callbacks are excluded.
use super::*;
use oteryn_game_server::content::ProjectV2Family;
const PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261001-npc-source-audit-r8/native-repairs.json"
);
const DIGEST: &str = "604606715f06362547beedd41ea5fa54856e8dbfadf14f0de67ff144c27ff2ed";
const FROM: &str = "g4-npc-qualified-source-r11";
const TO: &str = "g4-npc-d10-fandom-variants-r12";
const KEYS: [(&str, &str); 2] = [
    (
        "oteryn:npc.rapanaio_boat",
        "oteryn:dialogue.npc.rapanaio_boat",
    ),
    (
        "oteryn:npc.rapanaio_isle_of_evil",
        "oteryn:dialogue.npc.rapanaio_isle_of_evil",
    ),
];
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Packet {
    schema: String,
    from_project_revision: String,
    project_revision: String,
    npc_updates: Vec<Update>,
    dialogues: Vec<ProjectV2Declaration>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Update {
    identity: ProjectV2Identity,
    before: ProjectV2Declaration,
    after: ProjectV2Declaration,
}
pub(super) fn apply(draft: &mut ProjectV2Draft) -> Result<usize, Box<dyn std::error::Error>> {
    if hex_sha256(PACKET) != DIGEST {
        return Err("qualified D10 admission packet digest drifted".into());
    }
    let packet: Packet = serde_json::from_slice(PACKET)?;
    if packet.schema != "OTERYN_NPC_D10_VARIANT_ADMISSION/v1"
        || packet.from_project_revision != FROM
        || packet.project_revision != TO
        || draft.core.project_revision != FROM
        || packet.npc_updates.len() != 2
        || packet.dialogues.len() != 2
    {
        return Err("qualified D10 admission schema/revision/count drifted".into());
    }
    let mut replacements = Vec::new();
    for (slot, (npc_key, dialogue_key)) in KEYS.iter().enumerate() {
        let update = &packet.npc_updates[slot];
        let ProjectV2Declaration::Dialogue {
            identity: dialogue_id,
            ..
        } = &packet.dialogues[slot]
        else {
            return Err("qualified D10 addition is not Dialogue".into());
        };
        if dialogue_id.key != *dialogue_key || dialogue_id.revision != "definition-r1"
            || draft.state.declarations.iter().any(|value| {
                matches!(value, ProjectV2Declaration::Dialogue { identity, .. } if identity == dialogue_id)
            }) {
            return Err("qualified D10 identity is wrong or already admitted".into());
        }
        let mut permitted = update.before.clone();
        let (
            ProjectV2Declaration::Npc {
                identity, dialogue, ..
            },
            ProjectV2Declaration::Npc {
                identity: after_id,
                dialogue: after_dialogue,
                ..
            },
        ) = (&mut permitted, &update.after)
        else {
            return Err("qualified D10 update must preserve existing NPC kind".into());
        };
        let Some(reference) = after_dialogue else {
            return Err("qualified D10 closed Dialogue reference missing".into());
        };
        if identity != &update.identity
            || identity != after_id
            || identity.key != *npc_key
            || identity.revision != "definition-r1"
            || dialogue.is_some()
            || reference.family != ProjectV2Family::Dialogue
            || reference.key != dialogue_id.key
            || reference.revision != dialogue_id.revision
        {
            return Err("qualified D10 NPC/reference identity drifted".into());
        }
        dialogue.clone_from(after_dialogue);
        if permitted != update.after {
            return Err("qualified D10 changed NPC fields outside Dialogue reference".into());
        }
        let index = draft
            .state
            .declarations
            .iter()
            .position(|value| value == &update.before)
            .ok_or("qualified D10 exact NPC before fence drifted")?;
        replacements.push((index, update.after.clone()));
    }
    // Validate the entire packet before mutating the draft.
    for (index, declaration) in replacements {
        draft.state.declarations[index] = declaration;
    }
    draft.state.declarations.extend(packet.dialogues);
    draft.core.project_revision = TO.to_owned();
    Ok(2)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    fn fixture() -> ProjectV2Draft {
        let packet: Packet = serde_json::from_slice(PACKET).expect("typed proposed packet");
        ProjectV2Draft {
            core: ProjectDraft {
                project_revision: FROM.to_owned(),
                package_key: "oteryn:content.world-project".to_owned(),
                semantic_schema_version: "reference-schema-v1".to_owned(),
                licensing_metadata: "PENDING".to_owned(),
                world_id: "0123456789ab70cd8ef0123456789abc".to_owned(),
                coordinate_frame: "global-target-2026-09-27".to_owned(),
                records: vec![],
                imports: vec![],
                metadata: vec![],
            },
            state: ProjectV2State {
                declarations: packet.npc_updates.into_iter().map(|u| u.before).collect(),
                ..ProjectV2State::default()
            },
        }
    }
    #[test]
    fn admits_exactly_two_closed_dialogues_and_preserves_other_npc_fields() {
        let mut draft = fixture();
        let before = draft.clone();
        assert_eq!(apply(&mut draft).expect("typed static admission"), 2);
        assert_eq!(draft.state.declarations.len(), 4);
        let packet: Packet = serde_json::from_slice(PACKET).expect("typed proposed packet");
        for i in 0..2 {
            assert_eq!(draft.state.declarations[i], packet.npc_updates[i].after);
            assert_eq!(draft.state.declarations[i + 2], packet.dialogues[i]);
        }
        assert_eq!(draft.core.records, before.core.records);
        assert_eq!(draft.state.sources, before.state.sources);
        assert_eq!(draft.state.placements, before.state.placements);
        assert_eq!(draft.core.project_revision, TO);
    }
    #[test]
    fn rejects_before_fence_or_revision_drift_without_partial_mutation() {
        let mut draft = fixture();
        if let ProjectV2Declaration::Npc {
            dialogue,
            presentation,
            ..
        } = &mut draft.state.declarations[1]
        {
            *dialogue = presentation.clone();
        }
        let before = draft.clone();
        assert!(apply(&mut draft).is_err());
        assert_eq!(draft, before);
        let mut draft = fixture();
        draft.core.project_revision = "wrong-revision".to_owned();
        let before = draft.clone();
        assert!(apply(&mut draft).is_err());
        assert_eq!(draft, before);
    }
    #[test]
    fn rejects_existing_dialogue_identity_without_partial_mutation() {
        let mut draft = fixture();
        let packet: Packet = serde_json::from_slice(PACKET).expect("typed proposed packet");
        draft.state.declarations.push(packet.dialogues[1].clone());
        let before = draft.clone();
        assert!(apply(&mut draft).is_err());
        assert_eq!(draft, before);
    }
}
