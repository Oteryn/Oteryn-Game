{
    // F4 uses both ordinary starter characters before the profile fixture changes one.
    let mut client = None;
    let (pending, restored) = full_e2ef4::run(&mut client, clients, world, &metadata.npcs).await;
    let watch = verdict::Watch::new("E2EF4-lifecycle", verdict::Harness::global());
    let economy_social: TestResult = watch.bound(
        Duration::from_secs(180),
        "E2EF4 terminal release deadline",
        async {
            let mut client = client.ok_or("E2EF4 lost its final session; logout unproven")?;
            let (result, logout, attempts, busy_timed_out) = logout_retrying_busy(&mut client).await?;
            if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
                return Err(format!("E2EF4 final logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
            }
            drop(client);
            release_row_characters(clients).await
        },
    ).await;
    let lifecycle = restored.and(economy_social);
    for (row, result) in pending {
        row.finish(verdict::release_affects_verdict(&result, &lifecycle));
    }
    if let Err(error) = lifecycle {
        evidence(&format!("E2EF4_GAP lifecycle {error}"));
    }
}
