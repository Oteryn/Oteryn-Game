use oteryn_client_runtime::chat::ChatLine;
use oteryn_client_ui::chat::{ChatPanel, write_log};
#[test]
fn channel_log_writes_and_appends_utf8_and_propagates_io_errors() -> std::io::Result<()> {
    let panel = ChatPanel {
        lines: vec![ChatLine::Private {
            speaker_name: "Żółw".into(),
            text: "Zażółć gęślą jaźń 🐉".into(),
        }],
        ..ChatPanel::default()
    };
    let mut bytes = Vec::new();
    write_log(&panel, &mut bytes)?;
    assert!(
        std::str::from_utf8(&bytes).is_ok_and(|text| text.contains("Żółw: Zażółć gęślą jaźń 🐉"))
    );
    let once = bytes.clone();
    write_log(&panel, &mut bytes)?;
    assert_eq!(bytes, once.repeat(2));
    assert!(write_log(&panel, &mut std::io::Cursor::new(&mut [0u8; 1][..])).is_err());
    Ok(())
}
