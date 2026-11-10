//! Epoch-fenced weak-mailbox Wheel graph launcher.
use super::*;
pub(in crate::play) fn open(shared: &SharedMailbox, s: &UnboundedSender<PlayCommand>) {
    if s.is_closed() {
        return;
    }
    let epoch = {
        let mut m = lock(shared);
        if m.wheel.diagram.open || !m.wheel.selected || m.wheel_shutdown {
            return;
        }
        m.wheel.diagram.open = true;
        m.wheel.diagram.epoch = m.wheel.diagram.epoch.saturating_add(1);
        m.wheel.diagram.epoch
    };
    let weak = Arc::downgrade(shared);
    let rollback = weak.clone();
    let s = s.clone();
    if std::thread::Builder::new()
        .name("wheel-diagram".into())
        .spawn(move || run(weak, s, epoch))
        .is_err()
        && let Some(m) = rollback.upgrade()
    {
        let mut m = lock(&m);
        if m.wheel.diagram.epoch == epoch {
            m.wheel.diagram.open = false;
            m.wheel.diagram.feedback = "Cannot open the Wheel diagram".into();
        }
    }
}
