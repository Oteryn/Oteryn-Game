#[test]
fn party_list_uses_existing_party_query() -> Result<(), &'static str> {
    let mut panel = crate::party::PartyPanel {
        selected: true,
        ..Default::default()
    };
    let command =
        crate::party::request(&mut panel, crate::party::Intent::Refresh).ok_or("query")?;
    assert_eq!(command, oteryn_client_runtime::party::PartyCommand::query());
    Ok(())
}
