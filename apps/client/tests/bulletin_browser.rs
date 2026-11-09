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

#[test]
fn invalid_live_replacement_retains_the_previous_publication() -> Result<(), String> {
    let mut browser = oteryn_client::bulletin::Browser::new(
        2027,
        1,
        r#"{"news":[{"title":"Published","body":"Keep this"}],"events":[]}"#,
    )?;
    assert!(browser.replace("invalid JSON").is_err());
    assert!(browser.news().contains("Keep this"));
    browser.replace(r#"{"news":[],"events":[{"year":2027,"month":2,"day":1,"title":"Future event","body":"Description"}]}"#)?;
    browser.next_month();
    assert!(browser.month().contains("Future event"));
    Ok(())
}

#[test]
fn connected_bulletin_form_cannot_submit_a_second_login() {
    let mut form = oteryn_client::native_login::LoginForm::new(Default::default());
    form.finish(Ok(()));
    assert!(!form.can_submit());
    assert!(form.submit().is_err());
    form.offline();
    assert!(form.can_submit());
}
