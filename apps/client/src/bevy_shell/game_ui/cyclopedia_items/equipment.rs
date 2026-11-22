use super::*;

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub(super) struct Filters {
    pub hands: u8,
    pub class: Option<u8>,
    pub level: bool,
    pub vocation: bool,
}

impl Filters {
    pub fn project(self, panel: &mut Panel, link: &crate::play::PlayLink) {
        if self == Self::default() {
            return;
        }
        link.request_equipment_details();
        let character = link.cyclopedia_character_panel();
        let level = character.general.map(|stats| stats.level);
        let profession = character
            .vocation_id
            .and_then(oteryn_client_ui::cyclopedia_character::vocation_title);
        panel.catalogue.retain(|id, row| {
            let Some(Some(item)) = panel.equipment_details.get(id) else {
                return false;
            };
            for (key, value) in &item.attributes {
                if !row.attributes.iter().any(|(name, _)| name == key) {
                    row.attributes.push((key.clone(), value.clone()));
                }
            }
            let field = |name| {
                item.attributes
                    .iter()
                    .find(|(key, _)| key == name)
                    .map(|(_, value)| value.as_str())
            };
            let position = field("Body Position");
            (self.hands == 0
                || self.hands == 1 && position == Some("weapon hand")
                || self.hands == 2 && position == Some("both hands"))
                && self.class.is_none_or(|wanted| {
                    field("Classification").map_or(Some(0), |v| v.parse::<u8>().ok())
                        == Some(wanted)
                })
                && (!self.level
                    || field("Required Level").is_none_or(|v| {
                        v.parse::<u32>()
                            .ok()
                            .zip(level)
                            .is_some_and(|(required, current)| current >= required)
                    }))
                && (!self.vocation
                    || field("Professions").is_none_or(|v| {
                        profession.is_some_and(|title| {
                            v.split(", ").any(|allowed| {
                                title
                                    .split_whitespace()
                                    .last()
                                    .is_some_and(|base| base.eq_ignore_ascii_case(allowed))
                                    || title.replace(' ', "").eq_ignore_ascii_case(allowed)
                            })
                        })
                    }))
        });
        if !panel
            .catalogue_definition
            .parse::<u32>()
            .ok()
            .is_some_and(|id| panel.catalogue.contains_key(&id))
        {
            panel.catalogue_definition.clear();
        }
    }
}
