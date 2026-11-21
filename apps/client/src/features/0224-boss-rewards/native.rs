//! Windows personal boss reward panel; HWND access remains on its creating thread.
#![allow(unsafe_code)]
use oteryn_client_ui::boss_rewards::BossRewardsPanel;
pub(super) fn initial_fields(panel: &BossRewardsPanel) -> [String; 2] {
    [
        panel
            .death
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        panel.selected_ordinal.to_string(),
    ]
}

pub(super) fn parse_death(text: &str) -> Option<[u8; 16]> {
    let text = text.trim();
    if text.len() != 32 || !text.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut out = [0; 16];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}
