use super::WorldEventPanel;
use oteryn_client_runtime::world_event::SeasonalEventKind;

/// Civil-from-days, inverse of the calendar's days-from-civil conversion. Wire dates
/// are bounded to year 9999; the native panel always labels the server's UTC window.
fn utc_date(seconds: u64) -> String {
    let days = seconds / 86_400 + 719_468;
    let era = days / 146_097;
    let doe = days - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year + u64::from(month <= 2);
    let hour = seconds % 86_400 / 3_600;
    let minute = seconds % 3_600 / 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02} UTC")
}

pub fn panel_text(panel: &WorldEventPanel) -> String {
    if !panel.selected {
        return "World events unavailable in this session".into();
    }
    if panel.pending {
        return "Waiting for the server…".into();
    }
    if let Some(feedback) = panel.feedback {
        return feedback.into();
    }
    panel.state.as_ref().map_or_else(
        || "Refresh to query world events".into(),
        |state| {
            let announcement = if state.active {
                state.announcement.as_str()
            } else if !state.seasonal.is_empty() || state.cake.is_some() {
                "Seasonal event calendar"
            } else {
                "No active world event"
            };
            let mut text = if state.seasonal.is_empty() && state.cake.is_none() {
                format!(
                    "World event\r\n{announcement}\r\nReset epoch: {}",
                    state.reset_epoch
                )
            } else {
                format!("World events\r\n{announcement}")
            };
            for event in &state.seasonal {
                let name = match event.event {
                    SeasonalEventKind::Halloween => "Halloween",
                    SeasonalEventKind::Christmas => "Christmas — Santa and presents",
                    SeasonalEventKind::BirthdayCake => "Birthday Cake — A Piece of Cake",
                };
                let activity = if event.active { "active" } else { "scheduled" };
                text.push_str(&format!(
                    "\r\n\r\n{name}: {activity}\r\n{} to {} (end exclusive)",
                    utc_date(event.start_unix),
                    utc_date(event.end_unix),
                ));
            }
            if let Some(cake) = &state.cake {
                text.push_str(&cake_text(cake));
            }
            text
        },
    )
}

fn cake_text(cake: &oteryn_client_runtime::world_event::CakeProgress) -> String {
    let now = cake.observed_unix;
    let phase = match cake.stage {
        0 => "event finished",
        1 => "preparing the cake",
        2 => "the cake bridge is open",
        3 => "celebration — the cake bridge is open",
        _ => unreachable!("validated world-event state"),
    };
    let mut text = format!("\r\n\r\nA Piece of Cake: {phase}");
    if cake.stage == 1 {
        text.push_str(&format!(
            "\r\nCake substances: {}/200; cake golems: {}/200",
            cake.substances, cake.golems
        ));
    }
    if cake.stage == 2 {
        text.push_str(&format!(
            "\r\nCake layer {}/4: {}/{} bites",
            cake.layer, cake.layer_progress, cake.layer_goal
        ));
        if let Some(end) = cake.window_end {
            text.push_str(&format!("\r\nLayer deadline: {}", utc_date(end)));
        }
    }
    text.push_str(&format!(
        "\r\nOccurrence: {} to {}\r\nServer observation: {}",
        utc_date(cake.occurrence_start),
        utc_date(cake.occurrence_end),
        utc_date(now)
    ));
    if let Some(end) = cake.bonus_until {
        let start = end - 604800;
        if now < start {
            text.push_str(&format!(
                "\r\nEarned world bonus pending the next server save: begins {}",
                utc_date(start)
            ));
        } else if now < end {
            text.push_str(&format!("\r\nWorld bonus active at server observation: Premium experience +50%; bonus regeneration +25%\r\nUntil {}", utc_date(end)));
        } else {
            text.push_str(&format!("\r\nEarned world bonus ended {}", utc_date(end)));
        }
    }
    text
}
