//! Snapshot-bound shared desktop controls over the retained Wheel owner.
use super::{enqueue, wheel_presentation};
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::wheel_allocation::{Intent, WheelPanel, diagram};

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

/// Match the 0148 root/full-parent traversal, including disconnected cycles and ring budgets.
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
    if reply
        .nodes
        .iter()
        .enumerate()
        .any(|(i, node)| points[i] > 0 && (!open[i] || reply.unlock_points < node.minimum_points))
    {
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
                enqueue(&mut m, &self.commands, Intent::Reset);
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

#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_wheel_state(&self, panel: WheelPanel) {
        lock(&self.0).wheel = panel;
    }
}
