//! Immutable OTS media evidence intake; no Item binding or media execution.
use oteryn_game_server::content::{
    CandidateValue, ImportBatch, ReimportDecision, decide_reimport, world_project_sha256,
};

pub const BATCH_ID: &str = "g4-item-fx-audio295-raw-evidence-r1";
pub const STATE_COUNT: usize = 296;
pub const PAYLOAD: &[u8] =
    include_bytes!("../../../../imports/ots-source-evidence/item-fx-audio295/import-batch.json");
pub const PAYLOAD_SHA256: &str = "1eae9484506106c16b53c731e2693cf0ab9aac97ccb07025c977d27574ea156f";

pub fn append(imports: &mut Vec<ImportBatch>) -> Result<usize, String> {
    if world_project_sha256(PAYLOAD) != PAYLOAD_SHA256 {
        return Err("raw FX/audio payload digest drift".into());
    }
    let batch: ImportBatch = serde_json::from_slice(PAYLOAD).map_err(|e| e.to_string())?;
    if batch.batch_id != BATCH_ID
        || !batch.candidates.is_empty()
        || batch.reimport_states.len() != STATE_COUNT
        || batch.reimport_states.iter().any(|state| {
            state.baseline.is_some()
                || state.local.is_some()
                || !matches!(state.upstream, Some(CandidateValue::Text(_)))
                || state.decision != ReimportDecision::AdoptUpstream
                || state.decision != decide_reimport(&state.baseline, &state.upstream, &state.local)
        })
    {
        return Err("raw FX/audio closed evidence shape drift".into());
    }
    let existing: Vec<_> = imports.iter().filter(|b| b.batch_id == BATCH_ID).collect();
    if !existing.is_empty() {
        return if existing.len() == 1 && existing[0] == &batch {
            Ok(0)
        } else {
            Err("raw FX/audio existing batch conflict".into())
        };
    }
    imports.push(batch);
    Ok(STATE_COUNT)
}
