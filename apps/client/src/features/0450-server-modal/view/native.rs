//! Native controls use local indices; only server identities enter the answer wire.
#![allow(
    unsafe_code,
    reason = "Window and controls remain on their creating thread"
)]
#[derive(Default)]
struct Action {
    button: Option<usize>,
    close: bool,
    confirm: bool,
}
impl Action {
    fn command(&mut self, command: usize) -> bool {
        // Win32 LBN_DBLCLK is 2 and the choice list's control ID is 11.
        if (command >> 16) & 0xffff == 2 && command & 0xffff == 11 {
            self.confirm = true;
            return true;
        }
        // BN_CLICKED is zero; keep notification decoding testable without Win32.
        if (command >> 16) & 0xffff == 0 {
            self.button = (command & 0xffff).checked_sub(100);
            return true;
        }
        false
    }

    fn take_button_id(
        &mut self,
        dialog: &oteryn_client_ui::server_modal::Dialog,
        selected: Option<usize>,
    ) -> Option<u8> {
        let clicked = self.button.take();
        let confirm = std::mem::take(&mut self.confirm);
        if std::mem::take(&mut self.close) {
            return dialog.cancel_button;
        }
        let index = if confirm {
            match key_action(dialog, 13, selected, true) {
                KeyAction::Confirm(index) => Some(index),
                _ => None,
            }
        } else {
            clicked
        }?;
        dialog.buttons.get(index).map(|(id, _)| *id)
    }
}

fn fill_choices(
    dialog: &oteryn_client_ui::server_modal::Dialog,
    mut add: impl FnMut(&str),
    select: impl FnOnce(usize),
) {
    for (_, label) in &dialog.choices {
        add(label);
    }
    if !dialog.choices.is_empty() {
        select(0);
    }
}

#[derive(Debug, PartialEq, Eq)]
enum KeyAction {
    Close,
    Confirm(usize),
    Dispatch,
}

fn key_action(
    dialog: &oteryn_client_ui::server_modal::Dialog,
    key: usize,
    selected: Option<usize>,
    allow_confirm: bool,
) -> KeyAction {
    if key == 27 {
        return KeyAction::Close;
    }
    if key == 13
        && allow_confirm
        && let Some(index) = dialog.default_button
            .and_then(|button| dialog.buttons.iter().position(|(id, _)| *id == button))
            .or_else(|| dialog.buttons.iter().position(|(id, _)| Some(*id) != dialog.cancel_button))
    {
        let answer = oteryn_client_ui::server_modal::Answer {
            dialog_id: dialog.id,
            button_id: dialog.buttons[index].0,
            choice_id: if Some(dialog.buttons[index].0) == dialog.cancel_button {
                None
            } else {
                selected.and_then(|row| dialog.choices.get(row).map(|(id, _)| *id))
            },
        };
        if dialog.accepts(answer) {
            return KeyAction::Confirm(index);
        }
    }
    KeyAction::Dispatch
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_client_ui::server_modal::Dialog;
    fn dialog() -> Dialog {
        Dialog {
            id: 73,
            title: "Choose".into(),
            message: "Server options".into(),
            choices: vec![(9, "Nine".into())],
            buttons: vec![(8, "Cancel".into()), (4, "Accept".into())],
            default_button: None,
            cancel_button: Some(8),
            image: None,
        }
    }
    #[test]
    fn review_native_modal_enter_submits_the_server_default_even_when_it_is_close() {
        let mut dialog = dialog();
        dialog.choices.clear();
        dialog.buttons = vec![(1, "Close".into())];
        dialog.default_button = Some(1);
        dialog.cancel_button = Some(1);
        assert_eq!(key_action(&dialog, 13, None, true), KeyAction::Confirm(0));
        assert_eq!(key_action(&dialog, 27, None, true), KeyAction::Close);
        let mut action = Action { button: Some(0), ..Default::default() };
        assert_eq!(action.take_button_id(&dialog, None), Some(1));
    }

    #[test]
    fn review_native_modal_choice_double_click_submits_the_server_default() {
        let mut dialog = dialog();
        dialog.buttons.push((7, "Apply".into()));
        dialog.default_button = Some(7);
        let mut action = Action::default();
        assert!(action.command(11 | (2 << 16)));
        assert_eq!(action.take_button_id(&dialog, Some(0)), Some(7));
        assert_eq!(action.take_button_id(&dialog, Some(0)), None);
        assert_eq!(key_action(&dialog, 27, Some(0), true), KeyAction::Close);
        action.close = true;
        assert_eq!(action.take_button_id(&dialog, Some(0)), Some(8));
    }
    #[test]
    fn native_modal_initially_selects_first_server_choice() {
        let mut labels = Vec::new();
        let mut selected = None;
        fill_choices(
            &dialog(),
            |label| labels.push(label.to_owned()),
            |row| selected = Some(row),
        );
        assert_eq!(labels, ["Nine"]);
        assert_eq!(selected, Some(0));
        selected = None;
        let mut empty = dialog();
        empty.choices.clear();
        fill_choices(
            &empty,
            |label| labels.push(label.to_owned()),
            |row| selected = Some(row),
        );
        assert_eq!(selected, None);
    }
    #[test]
    fn native_modal_enter_validates_confirmation_and_preserves_multiline_editing() {
        let mut dialog = dialog();
        assert_eq!(
            key_action(&dialog, 13, Some(0), true),
            KeyAction::Confirm(1)
        );
        assert_eq!(key_action(&dialog, 13, None, true), KeyAction::Dispatch);
        assert_eq!(key_action(&dialog, 13, Some(3), true), KeyAction::Dispatch);
        assert_eq!(key_action(&dialog, 13, Some(0), false), KeyAction::Dispatch);
        assert_eq!(key_action(&dialog, 27, None, false), KeyAction::Close);
        dialog.choices.clear();
        assert_eq!(key_action(&dialog, 13, None, true), KeyAction::Confirm(1));
        dialog.buttons.pop();
        assert_eq!(key_action(&dialog, 13, None, true), KeyAction::Dispatch);
    }
}
