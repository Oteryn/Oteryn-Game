use super::{Panel, Rate, Section, interval_chart, report_elapsed};
pub(crate) fn xp_text(panel: &Panel, seconds: u64) -> String {
    let elapsed = report_elapsed(panel, seconds, super::super::Tool::Xp);
    use super::super::state::{eta_seconds, per_hour};
    let remaining = panel
        .goal_xp
        .map(|goal| goal.saturating_sub(panel.gained_xp));
    let eta = remaining.and_then(|value| eta_seconds(value, panel.gained_xp, elapsed));
    let goal = remaining.map_or_else(
        || tr!("Set the XP still needed for your level goal").into(),
        |value| lfmt!("Level goal remaining: {value} XP", value = value),
    );
    let eta = eta.map_or_else(
        || tr!("Collect experience and set a level goal to estimate time").into(),
        |value| {
            lfmt!(
                "Estimated time to level goal: {value} seconds",
                value = value
            )
        },
    );
    let mut text = lfmt!(
        "XP Analyser\r\nElapsed: {seconds} seconds",
        seconds = elapsed
    );
    let show_raw = panel.display.visible(Section::RawXp);
    text.push_str("\r\n");
    text.push_str(&if show_raw {
        lfmt!(
            "Experience: {} raw / {} gained",
            panel.raw_xp,
            panel.gained_xp
        )
    } else {
        lfmt!("Experience: {} gained", panel.gained_xp)
    });
    let hourly = per_hour(panel.gained_xp, elapsed);
    text.push_str("\r\n");
    text.push_str(&lfmt!("XP/h: {}", hourly));
    if show_raw {
        text.push_str("\r\n");
        text.push_str(&lfmt!("Raw XP/h: {}", per_hour(panel.raw_xp, elapsed)));
    }
    text.push_str(&format!("\r\n{goal}\r\n{eta}"));
    if panel.display.visible(Section::XpGauge) {
        text.push_str(&format!(
            "\r\n{}",
            panel.display.gauge(Rate::Xp, "XP/h", hourly.into())
        ));
    }
    if panel.display.visible(Section::XpGraph) {
        text.push_str(&format!(
            "\r\n{}\r\n{}",
            tr!("Gained XP"),
            interval_chart(&panel.xp_chart.values(seconds), seconds, elapsed)
        ));
        if show_raw {
            text.push_str(&format!(
                "\r\n{}\r\n{}",
                tr!("Raw XP"),
                interval_chart(&panel.raw_xp_chart.values(seconds), seconds, elapsed)
            ));
        }
    }
    if let Some(feedback) = panel.feedback {
        text.push_str(&format!("\r\n{feedback}"));
    }
    text
}
