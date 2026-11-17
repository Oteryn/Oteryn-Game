use oteryn_client_ui::bulletin::Bulletin;
#[test]
fn feed_retains_descriptions_and_filters_event_month() -> Result<(), String> {
    let feed = Bulletin::parse(
        r#"{"news":[{"title":"New client","body":"Use browser sign-in"}],"events":[{"year":2026,"month":11,"day":8,"title":"Test world","body":"Sandbox event"}]}"#,
    )?;
    assert!(feed.news_text().contains("Use browser sign-in"));
    assert!(
        feed.month_text(2026, 11)
            .contains("8: Test world — Sandbox event")
    );
    assert!(!feed.month_text(2026, 12).contains("Sandbox event"));
    assert!(
        Bulletin::parse(
            r#"{"news":[],"events":[{"year":2026,"month":2,"day":30,"title":"x","body":"y"}]}"#
        )
        .is_err()
    );
    Ok(())
}
