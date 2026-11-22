//! Snapshot-bound shared desktop controls over the retained Wheel owner.
use super::wheel_presentation;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::wheel_allocation::{WheelPanel, diagram};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WheelControl {
    Close,
    Node(usize),
    Preset(String),
    Refresh,
    Reset,
    Retry,
    Save,
}

pub(crate) fn editable(panel: &WheelPanel) -> bool {
    panel.ready()
        && panel.diagram.selected
        && panel.diagram.pending.is_none()
        && panel.gems.pending.is_none()
        && panel.gems.review.is_none()
        && diagram::metadata(panel).is_some_and(|reply| reply.eligible && reply.nodes.len() == 36)
}

/// Preserve graph connectivity while allowing reductions in level-locked rings.
fn adjust(panel: &mut WheelPanel, index: usize, remove: bool) -> bool {
    if !editable(panel) || index >= 36 {
        return false;
    }
    let Some(reply) = diagram::metadata(panel) else {
        return false;
    };
    if remove && !reply.removal_permitted {
        return false;
    }
    let mut points = panel.points;
    let next = if remove {
        points[index].checked_sub(1)
    } else {
        points[index].checked_add(1)
    };
    let Some(next) = next.filter(|value| *value <= reply.nodes[index].capacity) else {
        return false;
    };
    points[index] = next;
    if !remove && points.iter().map(|n| u32::from(*n)).sum::<u32>() > reply.available {
        return false;
    }
    let mut open = [false; 36];
    loop {
        let mut changed = false;
        for (index, node) in reply.nodes.iter().enumerate() {
            if !open[index]
                && (node.unlock_from.is_empty()
                    || node.unlock_from.iter().any(|from| {
                        let from = usize::from(*from);
                        from < 36 && open[from] && points[from] == reply.nodes[from].capacity
                    }))
            {
                open[index] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    if reply.nodes.iter().enumerate().any(|(i, node)| {
        points[i] > 0 && (!open[i] || (!remove && reply.unlock_points < node.minimum_points))
    }) {
        return false;
    }
    let nonce = reply.request.nonce;
    diagram::slot(panel, nonce, index) && diagram::points(panel, next)
}

impl PlayLink {
    pub(crate) fn wheel_poll_preview(&self) {
        let mut m = lock(&self.mailbox);
        if m.shared_frontend
            && m.wheel.open
            && !m.wheel_shutdown
            && !m.world_input_blocked()
            && !m.server_modal.is_active()
            && !self.commands.is_closed()
            && diagram::needs_query(&m.wheel)
        {
            wheel_presentation::query(&mut m, &self.commands);
        }
    }

    pub(crate) fn wheel_preset_bound(
        &self,
        expected: &WheelPanel,
        action: usize,
        fields: [String; 2],
    ) -> bool {
        let mut m = lock(&self.mailbox);
        if !m.shared_frontend
            || m.wheel_shutdown
            || self.commands.is_closed()
            || !m.wheel.open
            || m.wheel != *expected
            || m.world_input_blocked()
            || m.server_modal.is_active()
            || !editable(&m.wheel)
        {
            return false;
        }
        let selected = m.wheel.preset_fields[0].clone();
        m.wheel.preset_fields = fields;
        super::super::view::presets::submit_named(&self.mailbox, m, action, selected);
        true
    }

    pub(crate) fn wheel_control_bound(
        &self,
        expected: &WheelPanel,
        action: WheelControl,
        remove: bool,
    ) -> bool {
        let mut m = lock(&self.mailbox);
        if !m.shared_frontend
            || m.wheel_shutdown
            || self.commands.is_closed()
            || !m.wheel.open
            || m.wheel != *expected
        {
            return false;
        }
        if action == WheelControl::Close {
            m.wheel.open = false;
            m.wheel.presets_open = false;
            return true;
        }
        if m.world_input_blocked() || m.server_modal.is_active() {
            return false;
        }
        match action {
            WheelControl::Node(index) => return adjust(&mut m.wheel, index, remove),
            WheelControl::Preset(name) => {
                if !editable(&m.wheel) {
                    return false;
                }
                let Some(code) = m.wheel.preset_book.0.get(&name).cloned() else {
                    return false;
                };
                if m.wheel.import_build(&code).is_err() {
                    return false;
                }
                m.wheel.preset_fields[0] = name;
                m.wheel.diagram.reset_requested = false;
            }
            WheelControl::Save => wheel_presentation::activate(
                &mut m,
                &self.commands,
                wheel_presentation::Action::Apply,
            ),
            WheelControl::Reset => {
                if !editable(&m.wheel)
                    || diagram::metadata(&m.wheel).is_none_or(|r| !r.removal_permitted)
                {
                    return false;
                }
                return diagram::reset_draft(&mut m.wheel);
            }
            WheelControl::Refresh => wheel_presentation::activate(
                &mut m,
                &self.commands,
                wheel_presentation::Action::Refresh,
            ),
            WheelControl::Retry => wheel_presentation::activate(
                &mut m,
                &self.commands,
                wheel_presentation::Action::Retry,
            ),
            WheelControl::Close => return false,
        }
        true
    }
}

impl PlayLink {
    pub(crate) fn gem_control_bound(
        &self,
        expected: &WheelPanel,
        action: Option<super::gem_presentation::Action>,
    ) -> bool {
        let mut m = lock(&self.mailbox);
        if m.world_input_blocked() || m.wheel_shutdown || !m.wheel.open || &m.wheel != expected {
            return false;
        }
        if let Some(action) = action {
            super::gem_presentation::activate(&mut m, &self.commands, action);
        } else if m.wheel.gems.last_revision
            != m.wheel.confirmed_atelier.as_ref().map(|a| a.revision)
            || m.wheel
                .gems
                .review
                .as_ref()
                .is_some_and(|review| review.expires <= std::time::Instant::now())
        {
            super::gem_presentation::query(&mut m, &self.commands);
        }
        true
    }
}

#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_wheel_state(&self, panel: WheelPanel) {
        lock(&self.0).wheel = panel;
    }
}

#[cfg(test)]
mod regression_tests {
    use super::*;
    use oteryn_client_runtime::wheel_presentation as wire;
    use oteryn_client_ui::wheel_allocation::{atelier, wheel};
    fn locked_ring_panel() -> WheelPanel {
        let mut points = [0; 36];
        points[0] = 3;
        let request = wire::Request {
            nonce: 1,
            wheel_revision: 2,
            atelier_revision: 3,
            points,
            vessels: [None; 4],
        };
        let mut panel = WheelPanel {
            open: true,
            selected: true,
            synchronized: true,
            points,
            confirmed_wheel: Some(wheel::WheelAllocationReply {
                refused: false,
                revision: 2,
                points,
                vessels: Some([None; 4]),
            }),
            confirmed_atelier: Some(atelier::AtelierReply {
                refused: false,
                revision: 3,
                gems: vec![],
                grades: vec![],
                quality_quotes: vec![],
                grade_quotes: vec![],
            }),
            ..Default::default()
        };
        panel.diagram.selected = true;
        panel.diagram.reply = Some(wire::Reply {
            request: request.clone(),
            catalogue: "test".into(),
            vocation: "knight".into(),
            available: 50,
            unlock_points: 50,
            removal_permitted: true,
            eligible: true,
            draft_valid: false,
            reason: "Ring locked".into(),
            nodes: (0..36)
                .map(|_| wire::Node {
                    domain: 0,
                    capacity: 50,
                    minimum_points: 100,
                    unlock_from: vec![],
                    name: "Locked".into(),
                    per_point: vec![],
                    full_slot: vec![],
                })
                .collect(),
            revelations: vec![],
            current: vec![],
            draft: vec![],
        });
        panel.diagram.last = Some(request);
        panel
    }

    #[test]
    fn locked_ring_allocation_can_be_removed_one_point_at_a_time() {
        let mut panel = locked_ring_panel();
        assert!(!adjust(&mut panel, 0, false));
        for remaining in [2, 1, 0] {
            assert!(adjust(&mut panel, 0, true));
            assert_eq!(panel.points[0], remaining);
        }
        // Editing a proposal does not bypass the authoritative Save preview.
        assert!(diagram::submit(&mut panel, [0; 16]).is_none());
    }

    use super::super::super::view::presets::TEST_PATH;
    use oteryn_client_ui::wheel_allocation::presets::PresetBook;

    fn saved(path: &std::path::Path, name: &str) -> Option<String> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if let Some(code) = PresetBook::load(path)
                .ok()
                .and_then(|book| book.0.get(name).cloned())
            {
                return Some(code);
            }
            if std::time::Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    #[test]
    fn wheel_bound_saves_validated_build() -> std::io::Result<()> {
        let (link, _commands, outbox) = crate::play::play_channel();
        link.use_shared_frontend();
        let panel = locked_ring_panel();
        outbox.bevy_wheel_state(panel.clone());
        let root = tempfile::tempdir()?;
        let path = root.path().join("wheel.json");
        let target = path.clone();
        let mailbox = link.mailbox.clone();
        TEST_PATH.set(Some(Box::new(move || {
            if let Ok(mut m) = mailbox.try_lock() {
                m.wheel.points[0] += 1;
            }
            target
        })));
        assert!(link.wheel_preset_bound(&panel, 0, ["First".into(), String::new()]));
        assert_eq!(saved(&path, "First"), Some(panel.build_code()));
        Ok(())
    }

    #[test]
    fn wheel_bound_saves_second_preset_during_write() -> std::io::Result<()> {
        let (link, _commands, outbox) = crate::play::play_channel();
        link.use_shared_frontend();
        outbox.bevy_wheel_state(locked_ring_panel());
        let root = tempfile::tempdir()?;
        let path = root.path().join("wheel.json");
        let held = crate::atomic_file::lock(&path)?;
        for name in ["First", "Second"] {
            let target = path.clone();
            TEST_PATH.set(Some(Box::new(move || target)));
            assert!(link.wheel_preset_bound(&link.wheel_panel(), 0, [name.into(), String::new()]));
        }
        drop(held);
        let code = link.wheel_panel().build_code();
        assert_eq!(saved(&path, "First"), Some(code.clone()));
        assert_eq!(saved(&path, "Second"), Some(code));
        Ok(())
    }
}
