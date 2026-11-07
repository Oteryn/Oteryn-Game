//! Strict complete-skills decoder used by the existing skills panel.
pub fn decode(payload: &[u8]) -> Option<oteryn_client_runtime::skill_display::SkillDisplay> {
    oteryn_client_runtime::skill_display::SkillDisplay::decode(payload)
}
