use super::ConditionStatusPanel;
/// The icon catalogue is closed and shares its identities with the authoritative projection.
pub fn icon_label(icon: u8) -> (&'static str, &'static str) {
    match icon {
        1 => ("☠", "Poison"),
        2 => ("♨", "Burning"),
        3 => ("ϟ", "Energy"),
        4 => ("♥", "Bleeding"),
        5 => ("≈", "Drowning"),
        6 => ("❄", "Freezing"),
        7 => ("☀", "Dazzled"),
        8 => ("☾", "Cursed"),
        9 => ("◉", "Invisible"),
        10 => ("◌", "Drunk"),
        11 => ("⌖", "Rooted"),
        12 => ("!", "Feared"),
        13 => ("»", "Haste"),
        14 => ("≪", "Paralysed"),
        15 => ("♧", "Food"),
        16 => ("+", "Recovery"),
        17 => ("◈", "Mana shield"),
        18 => ("☼", "Light"),
        19 => ("◆", "Attributes"),
        20 => ("↑", "Skill bonus"),
        21 => ("+", "Regeneration"),
        22 => ("◇", "Outfit"),
        23 => ("✦", "Intense Hex"),
        24 => ("×", "Muted"),
        25 => ("○", "Pacified"),
        26 => ("−", "Powerless"),
        27 => ("!", "Agony"),
        _ => ("?", "Condition"),
    }
}
pub fn panel_text(panel: &ConditionStatusPanel) -> String {
    if !panel.selected {
        return "Conditions unavailable in this session".into();
    }
    if let Some(error) = panel.error {
        return error.into();
    }
    let Some(status) = &panel.snapshot else {
        return "Loading conditions…".into();
    };
    ordered_status_text(status, &panel.order)
}
/// Render an already decoded observation without duplicating the icon catalogue.
pub fn status_text(status: &super::ConditionStatus) -> String {
    ordered_status_text(status, &[])
}

fn ordered_status_text(status: &super::ConditionStatus, order: &[u8]) -> String {
    if status.conditions.is_empty() {
        return "No active conditions".into();
    }
    let mut text = String::new();
    let mut conditions: Vec<_> = status.conditions.iter().collect();
    if !order.is_empty() {
        conditions.sort_by_key(|condition| {
            order
                .iter()
                .position(|id| *id == condition.icon)
                .unwrap_or(order.len() + usize::from(condition.icon))
        });
    }
    for condition in conditions {
        let (icon, label) = icon_label(condition.icon);
        let duration = if condition.remaining_ms == -1 {
            "Permanent".into()
        } else {
            format!(
                "{}s",
                (condition.remaining_ms / 1_000 + i64::from(condition.remaining_ms % 1_000 != 0))
            )
        };
        text.push_str(&format!("{icon} {label}: {duration}\r\n"));
    }
    text
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::condition_status::{ActiveCondition, ConditionStatus, request};
    #[test]
    fn condition_status_panel_only_consumes_confirmed_icons_and_duration()
    -> Result<(), &'static str> {
        let mut panel = ConditionStatusPanel::default();
        assert!(request(&mut panel).is_none());
        panel.selected = true;
        assert_eq!(request(&mut panel), Some(vec![]));
        assert!(request(&mut panel).is_none());
        let status = ConditionStatus {
            conditions: vec![ActiveCondition {
                icon: 1,
                sub_id: 0,
                remaining_ms: 4_001,
            }],
        };
        assert!(panel.finish(true, &status.encode().ok_or("encode")?));
        assert!(panel_text(&panel).contains("☠ Poison: 5s"));
        assert!(!panel.finish(true, &[0, 1]));
        assert_eq!(panel.snapshot, Some(status));
        assert!(panel.finish(true, &[0]));
        assert_eq!(panel_text(&panel), "No active conditions");
        Ok(())
    }
}
