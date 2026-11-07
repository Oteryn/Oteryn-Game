use super::*;
#[test]
fn spectate_rate_window_does_not_reset_at_calendar_boundary() {
    let start = Instant::now();
    let mut window = RateWindow::default();
    for millis in [0, 59_000, 59_100, 59_200, 59_300] {
        assert!(window.attempt(start + Duration::from_millis(millis)));
    }
    assert!(!window.attempt(start + Duration::from_millis(59_999)));
    assert!(window.attempt(start + Duration::from_secs(60)));
    assert!(!window.attempt(start + Duration::from_millis(60_001)));
    assert!(window.expire(start + Duration::from_secs(120)));
    assert_eq!(window.0.len(), 0);
}
