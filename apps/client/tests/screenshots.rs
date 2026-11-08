use oteryn_client::screenshots::{CapturePolicy, Event, Rect, save_bmp};
#[test]
fn screenshot_is_saved_with_selected_crop() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!("cg-login-shot-{}.bmp", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let pixels = [1, 2, 3, 255, 4, 5, 6, 255];
    save_bmp(
        &path,
        2,
        1,
        &pixels,
        Rect {
            x: 1,
            y: 0,
            width: 1,
            height: 1,
        },
    )?;
    let bytes = std::fs::read(&path)?;
    assert_eq!(&bytes[..2], b"BM");
    assert_eq!(&bytes[54..], &[4, 5, 6, 255]);
    assert!(
        save_bmp(
            &path,
            2,
            1,
            &pixels,
            Rect {
                x: 0,
                y: 0,
                width: 2,
                height: 1
            }
        )
        .is_err()
    );
    std::fs::remove_file(path)?;
    Ok(())
}
#[test]
fn automatic_events_are_opt_in_and_deduplicated() {
    let mut policy = CapturePolicy::default();
    assert!(!policy.capture(Event::Level, 1));
    policy.level = true;
    assert!(policy.capture(Event::Level, 1));
    assert!(!policy.capture(Event::Level, 1));
    assert!(policy.capture(Event::Manual, 2));
}
