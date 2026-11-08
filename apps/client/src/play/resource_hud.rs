//! Read-only HUD observations reuse the admitted condition, rest and character-stat wires.
use super::{Outbox, PlayLink, PlayView, mailbox::lock};
use crate::combat_input::PlayCommand;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ResourceHudOptions {
    pub combo: bool,
    pub values: bool,
    pub soul: bool,
    pub stamina: bool,
    pub conditions: bool,
    pub rest: bool,
}
impl Default for ResourceHudOptions {
    fn default() -> Self {
        Self {
            combo: false,
            values: true,
            soul: true,
            stamina: true,
            conditions: true,
            rest: true,
        }
    }
}
#[derive(Debug, Default)]
pub(super) struct MailboxState {
    selected: bool,
    pending: bool,
    stamina: Option<u16>,
}
#[derive(Debug, Clone, Default)]
pub(super) struct ViewState {
    active: bool,
    refreshed: Option<Instant>,
    stamina_requested: Option<Instant>,
    stamina: Option<u16>,
    conditions: Option<oteryn_client_ui::condition_status::ConditionStatus>,
    rest: Option<oteryn_client_ui::rest_status::RestStatus>,
}

pub(super) fn stamina_command() -> oteryn_client_runtime::raw_feature::FeatureCommand {
    oteryn_client_runtime::raw_feature::FeatureCommand {
        capability: 199,
        kind: oteryn_session::cyclopedia_character::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
        payload: vec![
            oteryn_session::cyclopedia_character::CharacterTab::GeneralStats as u8,
            0,
            0,
            1,
        ],
    }
}
impl PlayLink {
    fn request_hud_stamina(&self) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let state = &mut mailbox.hud_resource;
        if !state.selected || state.pending {
            return false;
        }
        state.pending = true;
        if self.commands.send(PlayCommand::HudStamina).is_err() {
            state.pending = false;
            state.stamina = None;
            return false;
        }
        true
    }
}
impl Outbox {
    pub(super) fn hud_stamina_initialize(&self, selected: bool) {
        lock(&self.0).hud_resource = MailboxState {
            selected,
            ..Default::default()
        };
    }
    pub(super) fn hud_stamina_result(&self, result: &oteryn_session::GatedCommandResult) {
        use oteryn_client_ui::cyclopedia_character::{CyclopediaCharacterPanel, apply_result};
        let mut panel = CyclopediaCharacterPanel {
            selected: true,
            pending: true,
            command_kind: stamina_command().kind,
            requested_tab: Some(oteryn_session::cyclopedia_character::CharacterTab::GeneralStats),
            limit: 1,
            ..Default::default()
        };
        let stamina = apply_result(&mut panel, result)
            .then_some(panel.general)
            .flatten()
            .map(|stats| stats.stamina_minutes);
        let mut mailbox = lock(&self.0);
        mailbox.hud_resource.pending = false;
        mailbox.hud_resource.stamina = stamina;
    }
}
impl PlayView {
    pub fn enable_resource_hud(&mut self, active: bool) {
        self.resource_hud.active = active;
    }

    pub(super) fn sync_resource_hud(&mut self, link: &PlayLink) {
        let conditions = link.condition_status_panel();
        let rest = link.rest_status_panel();
        self.resource_hud.conditions = conditions
            .error
            .is_none()
            .then_some(conditions.snapshot)
            .flatten();
        self.resource_hud.rest = rest.error.is_none().then_some(rest.snapshot).flatten();
        self.resource_hud.stamina = lock(&link.mailbox).hud_resource.stamina;
        if !self.resource_hud.active {
            return;
        }
        let now = Instant::now();
        if self
            .resource_hud
            .refreshed
            .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(2))
        {
            let options = &self.preferences().resource_hud;
            if options.conditions && conditions.selected && !conditions.pending {
                link.condition_status_request();
            }
            if options.rest && rest.selected && !rest.pending {
                link.rest_status_request();
            }
            self.resource_hud.refreshed = Some(now);
        }
        if self.preferences().resource_hud.stamina
            && self
                .resource_hud
                .stamina_requested
                .is_none_or(|last| now.duration_since(last) >= Duration::from_secs(30))
            && link.request_hud_stamina()
        {
            self.resource_hud.stamina_requested = Some(now);
        }
    }

    #[must_use]
    pub fn resource_status(&self) -> String {
        let options = &self.preferences().resource_hud;
        let mut parts = Vec::new();
        if options.values {
            parts.push(self.vitals.as_ref().map_or_else(
                || "HP —   MP —".into(),
                |v| {
                    format!(
                        "HP {}/{}   MP {}/{}",
                        v.health, v.max_health, v.mana, v.max_mana
                    )
                },
            ));
        }
        if options.soul {
            parts.push(
                self.vitals
                    .as_ref()
                    .map_or_else(|| "Soul: —".into(), |v| format!("◇ Soul: {}", v.soul)),
            );
        }
        if options.stamina {
            parts.push(self.resource_hud.stamina.map_or_else(
                || "◷ Stamina: —".into(),
                |minutes| format!("◷ Stamina: {:02}:{:02}", minutes / 60, minutes % 60),
            ));
        }
        if self.preferences().show_harmony {
            parts.push(self.vitals.as_ref().map_or_else(
                || "Harmony: —".into(),
                |v| format!("Harmony: {}/5", v.harmony),
            ));
        }
        if options.combo {
            parts.push(self.vitals.as_ref().map_or_else(
                || "Combo charges: —".into(),
                |v| format!("Combo charges: {}/5", v.harmony),
            ));
        }
        if self.preferences().show_serene {
            parts.push(self.vitals.as_ref().map_or_else(
                || "Serene: —".into(),
                |v| format!("Serene: {}", if v.serene { "active" } else { "inactive" }),
            ));
        }
        if options.rest {
            parts.push(self.resource_hud.rest.map_or_else(
                || "Rest: —".into(),
                |rest| {
                    format!(
                        "☾ Rest {}   Food: {}   Stamina regen: {}   Soul regen: {}",
                        if rest.inside_resting_area {
                            "active"
                        } else {
                            "inactive"
                        },
                        if rest.food_remaining_ms == -1 {
                            "unlimited".into()
                        } else {
                            format!(
                                "{}s",
                                rest.food_remaining_ms.div_euclid(1000)
                                    + i32::from(rest.food_remaining_ms % 1000 != 0)
                            )
                        },
                        if rest.stamina { "active" } else { "inactive" },
                        if rest.soul { "active" } else { "inactive" }
                    )
                },
            ));
        }
        if options.conditions {
            parts.push(self.resource_hud.conditions.as_ref().map_or_else(
                || "Conditions: —".into(),
                oteryn_client_ui::condition_status::status_text,
            ));
        }
        parts.join("   ")
    }
}
