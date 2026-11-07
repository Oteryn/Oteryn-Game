use super::*;

#[test]
fn gameplay_entry_fails_before_any_route_or_credential() -> Result<(), RuntimeError> {
    let client = ClientBootstrap::new()?;
    assert!(matches!(
        client.request_gameplay_entry(),
        Err(GameplayEntryError::NativeProtocolUnavailable)
    ));
    assert!(!client.availability().is_available());
    client.shutdown();
    Ok(())
}

#[test]
fn migrated_input_contract_remains_available() -> Result<(), InputError> {
    assert_eq!(
        ClientBootstrap::validate_action_id("client.menu")?.as_str(),
        "client.menu"
    );
    Ok(())
}
