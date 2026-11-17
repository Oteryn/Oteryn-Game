use oteryn_client_runtime::ambient_sound::PresentationEventKind;

/// Preserve zero as an explicit stop; positional effects never replace the private loop.
pub fn cue(kind: PresentationEventKind) -> Option<u32> {
    match kind {
        PresentationEventKind::AmbientSound { cue_id } => Some(cue_id),
        _ => None,
    }
}
