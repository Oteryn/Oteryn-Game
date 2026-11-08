use oteryn_client::{GameplayEntryError, native_login::LoginForm};

#[test]
fn failed_login_offers_a_fresh_attempt() {
    let mut form = LoginForm::new(Default::default());
    assert_eq!(form.submit_caption(), "Save and sign in");
    form.finish(Err(GameplayEntryError::NativeProtocolUnavailable));
    assert_eq!(form.submit_caption(), "Retry sign-in");
    form.finish(Ok(()));
    assert_eq!(form.submit_caption(), "Save and sign in");
}
