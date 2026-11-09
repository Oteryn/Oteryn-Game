use super::ConditionStatusPanel;
/// The icon catalogue is closed and shares its identities with the authoritative projection.
pub fn icon_label(icon: u8) -> (&'static str, &'static str) {
    match icon {
        1 => ("☠", tr!("Poison")),
        2 => ("♨", tr!("Burning")),
        3 => ("ϟ", tr!("Energy")),
        4 => ("♥", tr!("Bleeding")),
        5 => ("≈", tr!("Drowning")),
        6 => ("❄", tr!("Freezing")),
        7 => ("☀", tr!("Dazzled")),
        8 => ("☾", tr!("Cursed")),
        9 => ("◉", tr!("Invisible")),
        10 => ("◌", tr!("Drunk")),
        11 => ("⌖", tr!("Rooted")),
        12 => ("!", tr!("Feared")),
        13 => ("»", tr!("Haste")),
        14 => ("≪", tr!("Paralysed")),
        15 => ("♧", tr!("Food")),
        16 => ("+", tr!("Recovery")),
        17 => ("◈", tr!("Mana shield")),
        18 => ("☼", tr!("Light")),
        19 => ("◆", tr!("Attributes")),
        20 => ("↑", tr!("Skill bonus")),
        21 => ("+", tr!("Regeneration")),
        22 => ("◇", tr!("Outfit")),
        23 => ("✦", tr!("Intense Hex")),
        24 => ("×", tr!("Muted")),
        25 => ("○", tr!("Pacified")),
        26 => ("−", tr!("Powerless")),
        27 => ("!", tr!("Agony")),
        _ => ("?", tr!("Condition")),
    }
}
pub fn panel_text(panel: &ConditionStatusPanel) -> String {
    if !panel.selected {
        return tr!("Conditions unavailable in this session").into();
    }
    if let Some(error) = panel.error {
        return error.into();
    }
    let Some(status) = &panel.snapshot else {
        return tr!("Loading conditions…").into();
    };
    ordered_status_text(status, &panel.order)
}
/// Render an already decoded observation without duplicating the icon catalogue.
pub fn status_text(status: &super::ConditionStatus) -> String {
    ordered_status_text(status, &[])
}

fn ordered_status_text(status: &super::ConditionStatus, order: &[u8]) -> String {
    if status.conditions.is_empty() {
        return tr!("No active conditions").into();
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
            tr!("Permanent").into()
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
