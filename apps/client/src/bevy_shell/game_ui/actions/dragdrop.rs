//! Upstream Bevy drop events route to the existing locked-row swap owner.
use super::*;
use bevy::picking::{events::PointerDragDrop, pointer::PointerButton};

pub(super) fn drop_slot(
    event: On<PointerDragDrop>,
    slots: Query<(&ActionControl, &Generation)>,
    owner: NonSend<HostOwner>,
    editor: Res<editor::EditorState>,
) {
    if event.button != PointerButton::Primary || owner.modal_active() || editor.active() {
        return;
    }
    let (Ok((ActionControl::Slot(from), source)), Ok((ActionControl::Slot(to), target))) =
        (slots.get(event.dropped), slots.get(event.entity))
    else {
        return;
    };
    if source.0 != owner.generation() || target.0 != owner.generation() {
        return;
    }
    if let Some(play) = &owner.play {
        play.link.bar_gesture(BarGesture::Swap {
            from: *from,
            to: *to,
        });
    }
}
