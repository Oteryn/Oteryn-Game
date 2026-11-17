use super::{protocol::cue, status};
use oteryn_client_runtime::ambient_sound::PresentationEventKind;

#[test]
fn replacement_and_silence_are_explicit() {
    assert_eq!(
        cue(PresentationEventKind::AmbientSound { cue_id: 25 }),
        Some(25)
    );
    assert_eq!(
        cue(PresentationEventKind::AmbientSound { cue_id: 0 }),
        Some(0)
    );
    assert_eq!(status(true), "ambience on");
    assert_eq!(status(false), "ambience off");
}
