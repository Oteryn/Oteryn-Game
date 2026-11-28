use crate::combat_input::PlayCommand;
use crate::play::feature_0115_mail::protocol::{Panels, accepted};
use oteryn_client_runtime::{
    cyclopedia_map::{CAPABILITY_CYCLOPEDIA_MAP_V1, COMMAND_TYPE_CYCLOPEDIA_QUERY, raid_warnings},
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::cyclopedia_map::Section;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;

const REFRESH: Duration = Duration::from_secs(60);

#[derive(Debug, Default)]
pub(in crate::play) struct Poll {
    next: Option<Instant>,
    pending: bool,
}

impl Poll {
    pub(in crate::play) fn closed(&mut self) {
        self.next = None;
    }
}

pub(super) fn poll(panels: &mut Panels, commands: &UnboundedSender<PlayCommand>, now: Instant) {
    let panel = &panels.cyclopedia_map;
    let visible = panel.geometry.open || (panel.open && panel.section == Section::Map);
    let poll = &mut panels.raid_warning_poll;
    if !panel.selected
        || !panel.raid_warnings_selected
        || !visible
        || poll.pending
        || poll.next.is_some_and(|next| now < next)
    {
        return;
    }

    poll.next = Some(now + REFRESH);
    poll.pending = commands
        .send(PlayCommand::Feature(FeatureCommand {
            capability: CAPABILITY_CYCLOPEDIA_MAP_V1,
            kind: COMMAND_TYPE_CYCLOPEDIA_QUERY,
            payload: raid_warnings::QUERY.to_vec(),
        }))
        .is_ok();
}

pub(in crate::play) fn is_query(command: &FeatureCommand) -> bool {
    command.kind == COMMAND_TYPE_CYCLOPEDIA_QUERY && command.payload == raid_warnings::QUERY
}

pub(in crate::play) fn apply(panels: &mut Panels, result: &GatedCommandResult) {
    if !std::mem::take(&mut panels.raid_warning_poll.pending) || !accepted(result) {
        return;
    }
    // Invalid snapshots leave the last confirmed overlay intact.
    let _ = oteryn_client_ui::cyclopedia_map::raid_warnings::apply(
        &mut panels.cyclopedia_map.raid_warnings,
        &result.payload,
    );
}
