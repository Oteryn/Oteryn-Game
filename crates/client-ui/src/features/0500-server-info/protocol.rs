use super::Panel;
use super::{CAPABILITY_SERVER_INFO_V1, COMMAND_TYPE_SERVER_INFO_QUERY, Query, Reply};
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};
pub fn request(panel: &mut Panel, query: Query) -> Option<FeatureCommand> {
    if !panel.selected || panel.pending.contains(&query) {
        return None;
    }
    let payload = query.encode()?;
    panel.pending.push(query);
    Some(FeatureCommand {
        capability: CAPABILITY_SERVER_INFO_V1,
        kind: COMMAND_TYPE_SERVER_INFO_QUERY,
        payload,
    })
}
pub fn apply_result(
    panel: &mut Panel,
    query: &Query,
    result: &GatedCommandResult,
) -> Option<Reply> {
    let index = panel.pending.iter().position(|q| q == query)?;
    panel.pending.remove(index);
    if !panel.selected
        || !matches!(
            result.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        )
    {
        return None;
    }
    let reply = Reply::decode(&result.payload)?;
    reply.matches(query).then_some(reply)
}

pub fn observe(
    reply: Reply,
    quest: &mut crate::quest_log::QuestPanel,
    spells: &mut crate::spell_catalog::Panel,
) {
    match reply {
        Reply::Quests(rows) => quest.titles = rows.into_iter().collect(),
        Reply::Spells(rows) => {
            for row in rows {
                spells.info.insert(row.cast_index, row);
            }
        }
    }
}
