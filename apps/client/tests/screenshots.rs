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

#[test]
fn shell_capture_observes_live_facts_and_retries_failed_saves()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client::screenshots::{CaptureController, Facts};
    let mut capture = CaptureController::default();
    capture.policy.level = true;
    capture.policy.death = true;
    capture.policy.achievement = true;
    capture.observe(Facts {
        level: Some(8),
        deaths: 0,
        achievement: Some(4),
    });
    assert!(!capture.pending());
    capture.observe(Facts {
        level: Some(9),
        deaths: 1,
        achievement: Some(5),
    });
    assert!(capture.pending());
    let path = std::env::temp_dir().join(format!("capture-caller-{}.bmp", std::process::id()));
    let rect = Rect {
        x: 0,
        y: 0,
        width: 1,
        height: 1,
    };
    assert!(capture.save(&path, 1, 1, &[], rect).is_err());
    assert!(capture.pending());
    capture.save(&path, 1, 1, &[1, 2, 3, 255], rect)?;
    assert!(!capture.pending());
    capture.observe(Facts {
        level: Some(9),
        deaths: 1,
        achievement: Some(5),
    });
    assert!(!capture.pending());
    assert_eq!(&std::fs::read(&path)?[54..], &[1, 2, 3, 255]);
    std::fs::remove_file(path)?;
    Ok(())
}
