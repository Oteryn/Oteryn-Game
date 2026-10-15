//! One explicitly qualified Bank-family admission. The original Oteryn greeting is content,
//! distinct from the exact pinned callback evidence; no upstream Lua or quest behavior executes.
use super::*;
const PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261015-npc-bank-services/native-bank-admission.json"
);
const DIGEST: &str = "fa6edea043748245172dfd2cc70efb859d60d15a6ffe03bc78524cec42fb91ee";

pub(super) fn apply(draft: &mut ProjectV2Draft) -> Result<(), Box<dyn std::error::Error>> {
    if hex_sha256(PACKET) != DIGEST || draft.core.project_revision != RANGED_PROJECT_REVISION {
        return Err("Bank admission source or predecessor drifted".into());
    }
    let packet: Value = serde_json::from_slice(PACKET)?;
    let before_npc: ProjectV2Declaration = serde_json::from_value(packet["before_npc"].clone())?;
    let before_dialogue: ProjectV2Declaration =
        serde_json::from_value(packet["before_dialogue"].clone())?;
    let service: ProjectV2Declaration = serde_json::from_value(packet["service"].clone())?;
    let keyword: oteryn_game_server::content::ProjectV2DialogueKeyword = serde_json::from_value(
        packet["original_oteryn_authored_dialogue"]["bank_keyword"].clone(),
    )?;
    let greet: Vec<String> =
        serde_json::from_value(packet["original_oteryn_authored_dialogue"]["greet"].clone())?;
    let ProjectV2Declaration::Service {
        identity,
        bank: true,
        ..
    } = &service
    else {
        return Err("Bank service shape drifted".into());
    };
    if identity.key != "oteryn:service.bank.paulie" || identity.revision != "definition-r1"
        || draft.state.declarations.iter().any(|d| matches!(d, ProjectV2Declaration::Service { identity: current, .. } if current == identity)) { return Err("Bank service identity already exists or drifted".into()); }
    let npc = draft.state.declarations.iter_mut().find(|d| matches!(d, ProjectV2Declaration::Npc { identity, .. } if identity.key == "oteryn:npc.paulie")).ok_or("Paulie is absent")?;
    if *npc != before_npc {
        return Err("Paulie before-state drifted".into());
    }
    let ProjectV2Declaration::Npc { services, .. } = npc else {
        return Err("Paulie is not NPC".into());
    };
    services.push(ProjectV2DefinitionRef {
        family: ProjectV2Family::Service,
        key: identity.key.clone(),
        revision: identity.revision.clone(),
    });
    let dialogue = draft.state.declarations.iter_mut().find(|d| matches!(d, ProjectV2Declaration::Dialogue { identity, .. } if identity.key == "oteryn:dialogue.npc.paulie")).ok_or("Paulie Dialogue is absent")?;
    if *dialogue != before_dialogue {
        return Err("Paulie Dialogue before-state drifted".into());
    }
    let ProjectV2Declaration::Dialogue {
        greet: current_greet,
        keywords,
        ..
    } = dialogue
    else {
        return Err("Paulie Dialogue shape".into());
    };
    if !current_greet.is_empty() || !keywords.is_empty() {
        return Err("Bank greeting cannot replace admitted text".into());
    }
    *current_greet = greet;
    keywords.push(keyword);
    draft.state.declarations.push(service);
    draft.core.project_revision = "npc-bank-1-20261015-r1".into();
    Ok(())
}
