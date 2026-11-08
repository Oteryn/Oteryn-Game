use oteryn_client::about::{Survey, about_text};
#[test]
fn about_identifies_release_and_survey_is_opt_in() -> Result<(), Box<dyn std::error::Error>> {
    assert!(about_text().contains(oteryn_client::RELEASE_ID));
    assert!(about_text().contains("MPL-2.0"));
    let path = std::env::temp_dir().join(format!("cg-login-survey-{}.txt", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let survey = Survey::new(true);
    assert!(!survey.save(&path, false)?);
    assert!(!path.exists());
    assert!(survey.save(&path, true)?);
    assert!(std::fs::read_to_string(&path)?.contains("Easy"));
    std::fs::remove_file(path)?;
    Ok(())
}
