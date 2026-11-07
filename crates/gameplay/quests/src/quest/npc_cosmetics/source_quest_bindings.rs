//! Finite declaration coverage for the composed source-to-Quest child owner.
//! This admits routing only; Root must still evaluate and persist the real child.
use super::source_program::{Expression, StorageBinding};
use serde::Deserialize;
use std::sync::OnceLock;
#[derive(Deserialize)]
struct Bindings {
    rows: Vec<Row>,
}
#[derive(Deserialize)]
struct Row {
    source_key: String,
    matches: Vec<Member>,
}
#[derive(Deserialize)]
struct Member {
    storage: String,
    binding: Identity,
    value: serde_json::Value,
}
#[derive(Deserialize)]
struct Identity {
    key: String,
    quest: String,
}
static BINDINGS: OnceLock<Result<Bindings, serde_json::Error>> = OnceLock::new();
pub(super) fn covers(
    source: &str,
    storage: &str,
    binding: &StorageBinding,
    value: &Expression,
) -> bool {
    let Ok(bindings)=BINDINGS.get_or_init(||serde_json::from_str(include_str!("../../../../../../tools/content-schema/native-gameplay/npc-quest-endpoint-bindings.json"))) else {return false;};
    let Ok(actual) = serde_json::to_value(value) else {
        return false;
    };
    bindings.rows.len() == 23
        && bindings
            .rows
            .iter()
            .find(|row| row.source_key == source)
            .is_some_and(|row| {
                row.matches.iter().any(|member| {
                    member.storage == storage
                        && member.binding.key == binding.key
                        && member.binding.quest == binding.quest
                        && member.value == actual
                })
            })
}
