//! Bounded session observations for battle filters; never infer unknown player facts.
use oteryn_client_runtime::{
    combat::EntityRef,
    cui3::battle_metadata::{self, PlayerMetadata},
    server_mechanic::{CommandStatus, ServerMechanicCommand, ServerMechanicResponse},
};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlayerMetadataState {
    pub selected: bool,
    pub values: BTreeMap<EntityRef, PlayerMetadata>,
    pending: Vec<EntityRef>,
    last_query: Option<Instant>,
    cursor: usize,
    pub error: Option<&'static str>,
}
pub fn request(
    state: &mut PlayerMetadataState,
    visible: &[EntityRef],
    now: Instant,
) -> Option<ServerMechanicCommand> {
    if !state.selected
        || visible.is_empty()
        || !state.pending.is_empty()
        || state
            .last_query
            .is_some_and(|last| now.saturating_duration_since(last) < Duration::from_secs(1))
    {
        return None;
    }
    state.values.retain(|entity, _| visible.contains(entity));
    let start = state.cursor % visible.len();
    let refs = visible
        .iter()
        .cycle()
        .skip(start)
        .take(visible.len().min(battle_metadata::MAX_ENTRIES))
        .copied()
        .collect::<Vec<_>>();
    let payload = battle_metadata::encode_query(&refs)?;
    state.cursor = (start + refs.len()) % visible.len();
    state.pending = refs;
    state.last_query = Some(now);
    state.error = None;
    Some(ServerMechanicCommand {
        capability: 17,
        command_type: battle_metadata::COMMAND_TYPE_BATTLE_METADATA_QUERY,
        payload,
    })
}
impl PlayerMetadataState {
    pub fn new(selected: bool) -> Self {
        Self {
            selected,
            ..Self::default()
        }
    }
    pub fn send_failed(&mut self) {
        self.pending.clear();
        self.selected = false;
        self.error = Some("Player details unavailable: session ended");
    }
    pub fn status(&self) -> &str {
        if !self.pending.is_empty() {
            "Loading player details…"
        } else {
            self.error.unwrap_or("")
        }
    }
    pub fn finish(&mut self, response: &ServerMechanicResponse) {
        let expected = std::mem::take(&mut self.pending);
        if expected.is_empty() {
            return;
        }
        for entity in &expected {
            self.values.remove(entity);
        }
        self.error = Some("Player details are unavailable; Any filters include unknown players");
        if !matches!(
            response.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ) {
            return;
        }
        let Some(rows) = battle_metadata::decode_reply(&response.payload) else {
            return;
        };
        if rows.len() != expected.len() || rows.iter().any(|row| !expected.contains(&row.entity)) {
            return;
        }
        self.error = rows
            .iter()
            .any(|row| row.vocation.is_none() || row.same_guild.is_none())
            .then_some("Some player details are unknown; Any filters include these players");
        self.values
            .extend(rows.into_iter().map(|row| (row.entity, row)));
    }
}
