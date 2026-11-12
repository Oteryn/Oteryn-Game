use super::{ChatPanel, Intent};
use oteryn_client_runtime::chat_private_channel::{
    ChatChannelList, ChatDisposition, ChatLine, ChatOutcome,
};

pub fn replace_state(panel: &mut ChatPanel, list: Option<ChatChannelList>, lines: Vec<ChatLine>) {
    // Snapshots contain the whole bounded log. Keep cutoffs relative to its surviving suffix.
    let overlap = (0..=panel.lines.len().min(lines.len()))
        .rev()
        .find(|&count| panel.lines[panel.lines.len() - count..] == lines[..count])
        .unwrap_or(0);
    let evicted = panel.lines.len() - overlap;
    for start in panel.history_start.values_mut() {
        *start = start.saturating_sub(evicted);
    }
    if let Some(previous) = &panel.list {
        for id in previous
            .own_channel
            .into_iter()
            .chain(previous.invited.iter().map(|invite| invite.channel_id))
        {
            if !list.as_ref().is_some_and(|list| {
                list.own_channel == Some(id)
                    || list.invited.iter().any(|invite| invite.channel_id == id)
            }) {
                panel.history_start.insert(id, lines.len());
            }
        }
    }
    panel.history_start.retain(|_, start| *start > 0);
    panel.list = list;
    panel.lines = lines;
    if let Some(list) = &panel.list {
        if !panel.channel.is_some_and(|id| {
            list.own_channel == Some(id)
                || list.invited.iter().any(|invite| invite.channel_id == id)
        }) {
            panel.channel = list
                .own_channel
                .or_else(|| list.invited.first().map(|invite| invite.channel_id));
        }
    } else {
        panel.channel = None;
    }
}

pub fn apply_result(panel: &mut ChatPanel, outcome: &ChatOutcome) {
    let intent = panel.pending_intent.take();
    panel.pending = false;
    panel.feedback = match outcome.disposition {
        ChatDisposition::Ok => None,
        ChatDisposition::NotAllowed => Some(
            match intent {
                Some(Intent::Create) => tr!("Premium is required to create a private channel"),
                Some(Intent::Invite | Intent::Exclude) => tr!("You have no own private channel"),
                _ => tr!("You are not allowed in this channel"),
            }
            .into(),
        ),
        ChatDisposition::RoomNotOpen => Some(
            if intent == Some(Intent::Send) {
                tr!("Join the channel first")
            } else {
                tr!("This channel is not open")
            }
            .into(),
        ),
        ChatDisposition::NotInvited => Some(tr!("You are not invited to this channel").into()),
        ChatDisposition::NotOnline => Some(tr!("The named player is not online").into()),
        ChatDisposition::Muted | ChatDisposition::Exhausted => Some(lfmt!(
            "Wait {} seconds before speaking",
            outcome.wait_seconds
        )),
        ChatDisposition::LevelTooLow => Some(tr!("Your level is too low").into()),
        ChatDisposition::NoVocation => Some(tr!("This action requires a vocation").into()),
        ChatDisposition::ChatUnavailable => Some(tr!("Private channels are unavailable").into()),
        ChatDisposition::Rejected => {
            Some(tr!("The server rejected the private-channel command").into())
        }
    };
}
