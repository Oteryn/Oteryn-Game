use super::*;

pub(in crate::bevy_shell::game_ui::chat) fn transcript_rows(
    panel: &oteryn_client_ui::chat::ChatPanel,
) -> Vec<RenderedRow> {
    let mut single = panel.clone();
    single.lines.clear();
    single.recipients.clear();
    let font_px = panel.console.display.font_px;
    let plain = |text: String| RenderedRow {
        text,
        line: None,
        recipient: None,
        font_px,
    };
    single.feedback = None;
    single.console.npc_lines.clear();
    single.console.show_server_messages = false;
    single.console.display.time = false;
    let prefix = oteryn_client_ui::chat::panel_text(&single);
    let header = oteryn_client_ui::tr!("Local Chat — Say / Whisper / Yell / Private\n");
    let heading = prefix.strip_prefix(header).unwrap_or(&prefix);
    let mut rows = Vec::new();
    if !heading.is_empty() {
        rows.push(plain(heading.to_owned()));
    }
    for (index, line) in panel.lines.iter().enumerate().skip(
        panel
            .lines
            .len()
            .saturating_sub(usize::from(panel.console.presentation.limit)),
    ) {
        single.lines.clear();
        single.lines.push(line.clone());
        single.recipients.clear();
        single
            .recipients
            .push(panel.recipients.get(index).cloned().flatten());
        let rendered = oteryn_client_ui::chat::panel_text(&single);
        if let Some(text) = rendered
            .strip_prefix(&prefix)
            .filter(|text| !text.is_empty())
        {
            rows.push(RenderedRow {
                text: format!(
                    "{}{text}",
                    panel.console.metadata.prefix(panel.console.display, index)
                ),
                recipient: single.recipients[0].clone(),
                line: Some(line.clone()),
                font_px,
            });
        }
    }
    let console = &panel.console;
    use oteryn_client_ui::chat::{ChannelTab, NoticeScope};
    match &console.current {
        Some(ChannelTab::Npc) => {
            rows.extend(
                console
                    .npc_lines
                    .iter()
                    .map(|line| plain(format!("{line}\n"))),
            );
        }
        Some(ChannelTab::Private(id))
            if console.show_server_messages && console.shows_messages(ConsoleTab::Private) =>
        {
            for notice in console
                .channel_notices
                .iter()
                .filter(|notice| notice.channel_id == *id)
            {
                let text = oteryn_client_ui::chat_channel_participants::notice_text(notice);
                rows.push(plain(format!("{text}\n")));
            }
        }
        tab if console.show_server_messages => {
            let combined = NoticeScope::Combined {
                loot_tab_open: console.channels.contains(&ChannelTab::Loot),
            };
            let scope = match tab {
                Some(ChannelTab::Loot) => Some(NoticeScope::LootOnly),
                Some(ChannelTab::ServerLog) => Some(combined),
                None if console.tab == ConsoleTab::All && console.show_messages => Some(combined),
                _ => None,
            };
            if let Some(scope) = scope {
                rows.extend(notice_rows(console, scope).into_iter().map(plain));
            }
        }
        _ => {}
    }
    if let Some(feedback) = &panel.feedback
        && console.presentation.feedback
        && console.filters.information
    {
        rows.push(plain(format!("{feedback}\n")));
    }
    if rows.is_empty() {
        rows.push(plain(String::new()));
    }
    rows
}
/// The shared notice formatter adds a timestamp only to the first line of each entry.
/// Comparing its two projections preserves embedded newlines, including text that looks
/// like the start of another notice.
fn notice_rows(
    console: &oteryn_client_ui::chat::Console,
    scope: oteryn_client_ui::chat::NoticeScope,
) -> Vec<String> {
    let project = |time| {
        console.notices.scoped_text(
            console.filters,
            oteryn_client_ui::chat::ConsoleDisplay {
                time,
                ..console.display
            },
            scope,
        )
    };
    let plain = project(false);
    let stamped = project(true);
    let mut rows: Vec<String> = Vec::new();
    for (text, timed) in plain
        .split_inclusive('\n')
        .zip(stamped.split_inclusive('\n'))
    {
        let displayed = if console.display.time { timed } else { text };
        if text == timed
            && let Some(previous) = rows.last_mut()
        {
            previous.push_str(displayed);
        } else {
            rows.push(displayed.to_owned());
        }
    }
    rows
}
