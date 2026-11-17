//! Epoch-fenced launcher for the weak-mailbox Gem window.
use super::*;
pub(in crate::play) fn open(shared: &SharedMailbox, sender: &UnboundedSender<PlayCommand>) {
    if sender.is_closed() {
        return;
    }
    let epoch = {
        let mut m = lock(shared);
        if !m.wheel.selected || m.wheel.gems.open || m.wheel_shutdown {
            return;
        }
        m.wheel.gems.open = true;
        m.wheel.gems.window_epoch = m.wheel.gems.window_epoch.saturating_add(1);
        m.wheel.gems.window_epoch
    };
    let weak = Arc::downgrade(shared);
    let rollback = weak.clone();
    let sender = sender.clone();
    if std::thread::Builder::new()
        .name("visual-gem-atelier".into())
        .spawn(move || run(weak, sender, epoch))
        .is_err()
        && let Some(m) = rollback.upgrade()
    {
        let mut m = lock(&m);
        if m.wheel.gems.window_epoch != epoch {
            return;
        }
        m.wheel.gems.open = false;
        m.wheel.gems.review = None;
        m.wheel.gems.feedback = Some("Could not open the Gem Atelier".into());
    }
}
