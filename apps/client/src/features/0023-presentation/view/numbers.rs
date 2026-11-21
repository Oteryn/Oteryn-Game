//! Numbers merge only within the initial 40% of the original cue's lifetime.
use super::*;
pub(super) fn merge(cues: &mut [VisualCue], kind: PresentationEventKind, now: Instant) -> bool {
    let PresentationEventKind::ValueText { position, primary, secondary, .. } = kind else { return false; };
    for cue in cues.iter_mut().rev() {
        if now.saturating_duration_since(cue.received) > std::time::Duration::from_millis(400) { continue; }
        let PresentationEventKind::ValueText { position: at, primary: old, secondary: extra, .. } = &mut cue.kind else { continue; };
        if *at != position || old.color != primary.color { continue; }
        let Some(sum) = old.value.checked_add(primary.value) else { continue; };
        let merged_extra = match (*extra, secondary) {
            (None, None) => None,
            (Some(a), Some(b)) if a.color == b.color => {
                let Some(value) = a.value.checked_add(b.value) else { continue; };
                Some(PresentationValue { value, color: a.color })
            }
            _ => continue,
        };
        old.value = sum;
        *extra = merged_extra;
        return true;
    }
    false
}
