use super::*;

pub(super) async fn readmit_after_release<'a>(
    clients: &SeamClients<'a>,
    _exact: &TlsConnector,
    next_grant: &mut impl FnMut(&str, [u8; 16], u64) -> OwnedGrant<'a>,
) -> TestResult<()> {
    let &SeamClients {
        address,
        accounts,
        characters,
        descriptor,
        certificate,
        url,
        ..
    } = clients;
    // character[0]'s own final re-admission (#822): the Channel removes the exact actor only
    // after the TERMINAL fact (already proven above), so a fresh entry is refused until then;
    // retry with a fresh grant for a bounded time. Retain the admitted connection through the
    // receipt read, then drop it with no intervening await before the caller requests shutdown.
    // `exchange` waits for liveness loss and therefore cannot prove the final controlled actor.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let retained = loop {
        let generation = platform_generation(descriptor, &accounts[0]).await?;
        let again = next_grant(&accounts[0], characters[0], generation);
        let token = sign_grant(&again.borrowed(), now_seconds()?);
        let reply = oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
            address,
            server_name: "localhost",
            root_certificate: certificate,
            schema_revision: 1,
            character_id: CharacterId::decode(&characters[0])?,
            admission_material: token.as_bytes(),
            client_build_id: "oteryn-dev-client/readmission",
            deadline: Duration::from_secs(10),
        })
        .await;
        match reply {
            Ok(client) => break client,
            Err(error) if tokio::time::Instant::now() >= deadline => {
                return Err(format!("released character was not admitted again: {error}").into());
            }
            Err(_) => {}
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    };
    if committed_admissions(url).await? != 5 {
        return Err("readmission after release did not commit exactly one GameSession".into());
    }
    evidence("grace_expiry readmitted_after_release=1 admissions=5");
    drop(retained);
    Ok(())
}
