use super::protocol::Submission;
use oteryn_client_runtime::server_modal::input::DialogInput;
use oteryn_client_ui::server_modal::Dialog;
use oteryn_client_ui::server_modal_input::Entry;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Sender;
pub(super) fn open(
    dialog: Dialog,
    input: Option<DialogInput>,
    remember: bool,
    (sender, entry): (Sender<Submission>, Arc<Mutex<Entry>>),
) {
    #[cfg(all(windows, not(test)))]
    {
        let _ = std::thread::Builder::new()
            .name("server-modal".into())
            .spawn(move || {
                ACTIVE.store(true, std::sync::atomic::Ordering::Release);
                native::run(dialog, input, remember, sender, entry);
                ACTIVE.store(false, std::sync::atomic::Ordering::Release);
            });
    }
    #[cfg(any(not(windows), test))]
    {
        let _ = (dialog, input, remember, sender, entry);
    }
}
#[cfg(all(windows, not(test)))]
#[path = "view/native.rs"]
mod native;

#[cfg(all(windows, not(test)))]
static ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub(in crate::play) fn active() -> bool {
    #[cfg(all(windows, not(test)))]
    {
        ACTIVE.load(std::sync::atomic::Ordering::Acquire)
    }
    #[cfg(any(not(windows), test))]
    {
        false
    }
}

// Use the same control construction on native windows and in the headless layout test.
#[cfg(any(windows, test))]
fn input_control<T>(input: &DialogInput, create: impl FnOnce(&str, [i32; 4], u32) -> T) -> T {
    use oteryn_client_runtime::server_modal::input::InputKind;
    // Win32 window/edit style values; keep construction testable without a desktop.
    const WS_BORDER: u32 = 0x0080_0000;
    const WS_TABSTOP: u32 = 0x0001_0000;
    const WS_VSCROLL: u32 = 0x0020_0000;
    const ES_MULTILINE: u32 = 0x0004;
    const ES_AUTOVSCROLL: u32 = 0x0040;
    const ES_AUTOHSCROLL: u32 = 0x0080;
    const ES_WANTRETURN: u32 = 0x1000;
    let multiline = input.kind == InputKind::Edit;
    create(
        &oteryn_client_ui::server_modal_input::initial_text(input),
        [120, 326, 572, if multiline { 100 } else { 26 }],
        WS_BORDER
            | WS_TABSTOP
            | if multiline {
                WS_VSCROLL | ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN
            } else {
                ES_AUTOHSCROLL
            },
    )
}

#[cfg(any(windows, test))]
fn footer_controls<T>(
    remember: bool,
    checkbox: impl FnOnce([i32; 4]) -> T,
    feedback: impl FnOnce([i32; 4]) -> T,
) -> (Option<T>, T) {
    (
        remember.then(|| checkbox([420, 576, 272, 24])),
        feedback([12, 535, 680, 40]),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn server_modal_edit_control_requests_enter_and_preserves_multiline_text() {
        use oteryn_client_runtime::server_modal::input::{DialogInput, InputKind, InputValue};
        for kind in [InputKind::Edit, InputKind::Text, InputKind::Amount] {
            let input = DialogInput {
                kind,
                minimum: 0,
                maximum: 120,
                initial: Some(if kind == InputKind::Amount {
                    InputValue::Amount(5)
                } else if kind == InputKind::Edit {
                    InputValue::Text("First\r\nSecond".into())
                } else {
                    InputValue::Text("Single line".into())
                }),
            };
            assert!(input.valid());
            super::input_control(&input, |text, rect, style| {
                let multiline = kind == InputKind::Edit;
                assert_eq!(style & 0x1000 != 0, multiline, "ES_WANTRETURN");
                assert_eq!(style & 0x0004 != 0, multiline, "ES_MULTILINE");
                assert_eq!(style & 0x0040 != 0, multiline, "ES_AUTOVSCROLL");
                assert_eq!(style & 0x0080 != 0, !multiline, "ES_AUTOHSCROLL");
                assert_eq!(rect, [120, 326, 572, if multiline { 100 } else { 26 }]);
                assert_eq!(
                    text,
                    match kind {
                        InputKind::Amount => "5",
                        InputKind::Edit => "First\r\nSecond",
                        _ => "Single line",
                    }
                );
            });
        }
    }

    #[test]
    fn server_modal_remember_and_feedback_controls_do_not_overlap() {
        let (checkbox, feedback) = super::footer_controls(true, |rect| rect, |rect| rect);
        let Some([rx, ry, rw, rh]) = checkbox else {
            unreachable!("remember checkbox requested");
        };
        let [fx, fy, fw, fh] = feedback;
        assert!(rx + rw <= fx || fx + fw <= rx || ry + rh <= fy || fy + fh <= ry);
        assert!(rx >= 0 && ry >= 0 && rx + rw <= 700 && ry + rh <= 600);
        assert!(fx >= 0 && fy >= 0 && fx + fw <= 700 && fy + fh <= 600);
        let (checkbox, plain_feedback) = super::footer_controls(false, |rect| rect, |rect| rect);
        assert!(checkbox.is_none());
        assert_eq!(plain_feedback, feedback);
    }
}
