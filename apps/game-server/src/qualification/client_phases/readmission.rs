use super::*;

pub(super) async fn readmit_after_release<'a>(
    clients: &SeamClients<'a>,
    exact: &TlsConnector,
    next_grant: &mut impl FnMut(&str, [u8; 16], u64) -> OwnedGrant<'a>,
) -> TestResult<()> {
    let &SeamClients {
        address,
        accounts,
        characters,
        descriptor,
        url,
        ..
    } = clients;
    // character[0]'s own final re-admission (#822): the Channel removes the exact actor only
    // after the TERMINAL fact (already proven above), so a fresh entry is refused until then;
    // retry with a fresh grant for a bounded time. This is deliberately the *last* action before
    // this function returns: the connection below is a plain disconnect (never a command-gap
    // close), so its own control-loss timer is racing the `shutdown` this function's caller
    // fires immediately once every client case is done, and it must win that race — exactly as
    // it did before this stage existed — to remain the one committed actor `seam_flow`'s final
    // invariant expects.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let generation = platform_generation(descriptor, &accounts[0]).await?;
        let again = next_grant(&accounts[0], characters[0], generation);
        let token = sign_grant(&again.borrowed(), now_seconds()?);
        let reply = exchange(
            address,
            exact,
            &framed(&bootstrap(1, 1, &characters[0], &token)),
        )
        .await?;
        if accepted_session(&reply).is_some() {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("released character was not admitted again: {reply:?}").into());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    if committed_admissions(url).await? != 5 {
        return Err("readmission after release did not commit exactly one GameSession".into());
    }
    evidence("grace_expiry readmitted_after_release=1 admissions=5");
    Ok(())
}
