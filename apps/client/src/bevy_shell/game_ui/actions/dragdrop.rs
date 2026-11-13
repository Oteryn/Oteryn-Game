//! Upstream Bevy drop events route to the existing locked-row swap owner.
use super::*;
use bevy::picking::{events::PointerDragDrop, pointer::PointerButton};

pub(super) fn drop_slot(
    event: On<PointerDragDrop>,
    slots: Query<(&ActionControl, &Generation)>,
    items: Query<&super::super::ItemDragSource>,
    owner: NonSend<HostOwner>,
    editor: Res<editor::EditorState>,
) {
    if event.button != PointerButton::Primary || owner.modal_active() || editor.active() {
        return;
    }
    let Ok((ActionControl::Slot(to), target)) = slots.get(event.entity) else {
        return;
    };
    if target.0 != owner.generation() {
        return;
    }
    if let Ok(source) = items.get(event.dropped) {
        let Some(item) = super::super::inventory::current_drag_item(&owner, *source) else {
            return;
        };
        if let Some(play) = &owner.play {
            let layout = play.link.bar_layout();
            let Some(row) = layout.rows.get(*to / ROW_SLOTS) else {
                return;
            };
            if row.enabled && !row.locked && *to % ROW_SLOTS < usize::from(row.count) {
                play.link.assign_bar_action(
                    *to,
                    crate::play::BarAction::Use {
                        definition: item.item_definition_ref,
                        mode: crate::play::ItemUseMode::Cursor,
                    },
                );
                play.link.item_drag_abort();
            }
        }
        return;
    }
    let Ok((ActionControl::Slot(from), source)) = slots.get(event.dropped) else {
        return;
    };
    if source.0 != owner.generation() {
        return;
    }
    if let Some(play) = &owner.play {
        play.link.bar_gesture(BarGesture::Swap {
            from: *from,
            to: *to,
        });
    }
}
