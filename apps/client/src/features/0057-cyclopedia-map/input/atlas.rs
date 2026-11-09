use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use oteryn_client_runtime::{
    cyclopedia_map::atlas as wire,
    raw_feature::{FeatureCommand, GatedCommandResult},
};
use oteryn_client_ui::cyclopedia_map::Panel;
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn cyclopedia_atlas_open(&self) {
        self.cyclopedia_atlas_action(0);
        #[cfg(windows)]
        super::super::view::atlas::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).properties.cyclopedia_map.atlas.open = true;
        }
    }
    pub fn cyclopedia_atlas_action(&self, action: usize) {
        submit(&mut lock(&self.mailbox), &self.commands, action);
    }
    pub fn cyclopedia_atlas_pump(&self) {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.properties.cyclopedia_map;
        if panel.pending || !panel.atlas.open || !panel.atlas.auto_load {
            return;
        }
        let Some(after) = panel.atlas.next_geometry else {
            return;
        };
        let Some(area) = panel.atlas.selected else {
            return;
        };
        let floor = panel.atlas.floor;
        send(panel, &self.commands, wire::Request { area, floor, after });
    }
    pub fn cyclopedia_atlas_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
        let modifiers = Modifiers::one(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift);
        if !events.iter().any(|e| matches!(e, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers: actual, repeat: false } if code.get() == 66 && *actual == modifiers)) { return false; }
        self.cyclopedia_atlas_open();
        true
    }
}
fn send(panel: &mut Panel, commands: &UnboundedSender<PlayCommand>, request: wire::Request) {
    let Ok(payload) = wire::encode_request(request) else {
        panel.atlas.feedback = "Invalid atlas selection".into();
        return;
    };
    if !panel.begin(72) {
        return;
    }
    panel.atlas.pending = Some(request);
    if commands
        .send(PlayCommand::Feature(FeatureCommand {
            capability: 57,
            kind: 72,
            payload,
        }))
        .is_err()
    {
        panel.atlas.pending = None;
        panel.atlas.auto_load = false;
        panel.failed("The session has ended");
    }
}
pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    action: usize,
) {
    let panel = &mut mailbox.properties.cyclopedia_map;
    if !panel.selected || panel.pending {
        return;
    }
    if action == 10 {
        super::submit(
            mailbox,
            commands,
            oteryn_client_ui::cyclopedia_map::Intent::Query,
        );
        return;
    }
    if action == 11 {
        #[cfg(windows)]
        super::super::view::atlas::show_map(panel, commands);
        return;
    }
    if action == 8 || action == 9 {
        if action == 8 {
            panel.atlas.hide_geometry = !panel.atlas.hide_geometry;
        } else {
            panel.hide_pois = !panel.hide_pois;
        }
        panel.atlas.revision = panel.atlas.revision.wrapping_add(1);
        return;
    }
    let request = match action {
        0 | 4 => wire::Request {
            area: 0,
            floor: 1,
            after: 0,
        },
        1 | 2 => {
            let a = &mut panel.atlas;
            if a.areas.is_empty() {
                return;
            }
            let i = a
                .areas
                .iter()
                .position(|area| Some(area.id) == a.selected)
                .unwrap_or(0);
            let i = if action == 1 {
                (i + a.areas.len() - 1) % a.areas.len()
            } else {
                (i + 1) % a.areas.len()
            };
            a.selected = Some(a.areas[i].id);
            a.clear_geometry();
            return;
        }
        3 => {
            let Some(area) = panel.atlas.area() else {
                return;
            };
            if area.parent != 0 {
                panel.atlas.feedback = "This is already a subarea".into();
                return;
            }
            wire::Request {
                area: area.id,
                floor: 1,
                after: 0,
            }
        }
        5..=7 => {
            let Some(area) = panel.atlas.selected else {
                panel.atlas.feedback = "Choose an area first".into();
                return;
            };
            if action == 6 {
                panel.atlas.floor = if panel.atlas.floor == 0 {
                    -15
                } else {
                    panel.atlas.floor + 1
                };
            }
            if action == 7 {
                panel.atlas.floor = if panel.atlas.floor == -15 {
                    0
                } else {
                    panel.atlas.floor - 1
                };
            }
            panel.atlas.clear_geometry();
            panel.atlas.auto_load = true;
            wire::Request {
                area,
                floor: panel.atlas.floor,
                after: 0,
            }
        }
        12 => {
            let Some(after) = panel.atlas.next_areas else {
                panel.atlas.feedback = "End of area list".into();
                return;
            };
            wire::Request {
                area: panel.atlas.parent,
                floor: 1,
                after,
            }
        }
        13 => wire::Request {
            area: panel.atlas.parent,
            floor: 1,
            after: panel.atlas.after.saturating_sub(wire::AREA_PAGE as u16),
        },
        _ => return,
    };
    if request.floor == 1 {
        panel.atlas.auto_load = false;
    }
    send(panel, commands, request);
}
pub(in crate::play) fn apply(
    panel: &mut Panel,
    command: &FeatureCommand,
    result: &GatedCommandResult,
) -> bool {
    if command.kind != 72 || command.payload.first() != Some(&255) {
        return panel.atlas.pending.is_some();
    }
    let Ok(request) = wire::decode_request(&command.payload) else {
        return true;
    };
    if !panel.pending || panel.atlas.pending != Some(request) {
        return true;
    }
    panel.pending = false;
    panel.atlas.pending = None;
    if !crate::play::feature_0115_mail::protocol::accepted(result) {
        panel.atlas.auto_load = false;
        panel.atlas.feedback = "Atlas unavailable in this scope".into();
        return true;
    }
    let Ok(reply) = wire::decode_reply(&result.payload) else {
        panel.atlas.auto_load = false;
        panel.atlas.feedback = "Invalid atlas reply".into();
        return true;
    };
    let a = &mut panel.atlas;
    match reply {
        wire::Reply::Areas(page)
            if request.floor == 1 && page.parent == request.area && page.after == request.after =>
        {
            a.clear_geometry();
            a.parent = page.parent;
            a.after = page.after;
            a.next_areas = page.next;
            a.selected = page.areas.first().map(|area| area.id);
            a.areas = page.areas;
            a.feedback = if a.areas.is_empty() {
                "No mapped areas in this group"
            } else {
                "Choose an area, then Load map"
            }
            .into();
        }
        wire::Reply::Geometry(page)
            if request.floor != 1
                && page.area == request.area
                && page.floor == request.floor
                && page.page == request.after =>
        {
            let offset = usize::from(page.page) * wire::RUN_PAGE;
            if a.runs.len() != offset
                || (offset > 0 && (a.total != page.total || a.bounds != Some(page.bounds)))
                || a.runs
                    .last()
                    .zip(page.runs.first())
                    .is_some_and(|(old, new)| {
                        old.y > new.y || (old.y == new.y && old.end > new.begin)
                    })
            {
                a.auto_load = false;
                a.feedback = "Invalid atlas continuation".into();
                return true;
            }
            a.bounds = (page.total > 0).then_some(page.bounds);
            a.total = page.total;
            a.runs.extend(page.runs);
            a.next_geometry = page.next;
            a.auto_load = page.next.is_some();
            a.feedback = if a.total == 0 {
                "No mapped footprint on this floor"
            } else if a.auto_load {
                "Loading area geometry"
            } else {
                "Area geometry loaded"
            }
            .into();
        }
        _ => {
            a.auto_load = false;
            a.feedback = "Mismatched atlas reply".into();
        }
    }
    a.revision = a.revision.wrapping_add(1);
    true
}
