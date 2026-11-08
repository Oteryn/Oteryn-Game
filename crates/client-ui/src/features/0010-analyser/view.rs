use super::{Panel, Tool};
use std::fmt::Write;
#[must_use]
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return "Analysers unavailable in this session".into();
    }
    let mut out = format!("{} analyser\r\n", panel.tool.title());
    let all = panel.tool == Tool::Overview;
    if all || panel.tool == Tool::Hunting {
        let _ = write!(out, "Kills: {}\r\n", panel.kills);
    }
    if all || panel.tool == Tool::Experience {
        let _ = write!(
            out,
            "Experience: {} raw / {} gained\r\n",
            panel.raw_xp, panel.gained_xp
        );
    }
    if all || panel.tool == Tool::Loot {
        let _ = write!(
            out,
            "Loot: {} items / {} gold\r\n",
            panel.loot_items, panel.gold
        );
    }
    if all || panel.tool == Tool::Supplies {
        let _ = write!(out, "Supplies used: {}\r\n", panel.supplies);
    }
    if all || panel.tool == Tool::Damage {
        let _ = write!(
            out,
            "Damage dealt: {} | Damage received: {}\r\n",
            panel.damage, panel.received
        );
    }
    if all || panel.tool == Tool::Healing {
        let _ = write!(out, "Healing: {}\r\n", panel.healing);
    }
    let _ = write!(
        out,
        "Dropped facts: {} (totals may be incomplete)",
        panel.dropped
    );
    out
}
