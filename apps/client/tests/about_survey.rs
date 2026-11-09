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

#[test]
fn survey_invitation_reviews_answer_before_local_export() -> Result<(), Box<dyn std::error::Error>>
{
    use oteryn_client::about::review_survey_at;
    let path = std::env::temp_dir().join(format!("survey-review-{}.txt", std::process::id()));
    let mut calls = 0;
    assert!(!review_survey_at(&path, |_| {
        calls += 1;
        false
    })?);
    assert_eq!(calls, 1);
    assert!(!path.exists());
    calls = 0;
    assert!(!review_survey_at(&path, |text| {
        calls += 1;
        if calls == 3 {
            assert!(text.contains("Answer: Difficult"));
        }
        calls == 1
    })?);
    assert_eq!(calls, 3);
    assert!(!path.exists());
    calls = 0;
    assert!(review_survey_at(&path, |text| {
        calls += 1;
        if calls == 3 {
            assert!(text.contains("Answer: Easy"));
        }
        true
    })?);
    assert!(std::fs::read_to_string(&path)?.contains("Answer: Easy"));
    std::fs::remove_file(path)?;
    Ok(())
}
