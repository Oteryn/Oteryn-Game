use oteryn_client::bulletin::Browser;
#[test]
fn calendar_browses_year_boundaries_and_retains_news() -> Result<(), String> {
    let mut browser = Browser::new(
        2026,
        12,
        r#"{"news":[{"title":"Client","body":"Browser sign-in"}],"events":[]}"#,
    )?;
    assert!(browser.news().contains("Browser sign-in"));
    assert!(browser.month().starts_with("Events 2026-12"));
    browser.next_month();
    assert!(browser.month().starts_with("Events 2027-01"));
    browser.previous_month();
    assert!(browser.month().starts_with("Events 2026-12"));
    assert!(Browser::new(2026, 0, "{}").is_err());
    Ok(())
}
