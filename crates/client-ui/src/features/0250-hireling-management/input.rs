use super::Panel;
pub fn select(p: &mut Panel, at: usize) -> bool {
    if p.pending.is_some() {
        return false;
    }
    let Some(entry) = p.entries.get(at) else {
        return false;
    };
    p.identity = Some(entry.hireling);
    true
}
