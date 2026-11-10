{
    #[path = "full_e2ef4.rs"]
    mod full_e2ef4;
    // Run before profile_journey provisions Character 0 through the root fixture.
    // Both peers here must retain their ordinary Platform-created starter state.
    let economy_social: TestResult = async {
        let mut client = connect_world(clients, 0x80).await?;
        full_e2ef4::run(&mut client, clients, &world, &metadata.npcs).await;
        let (result, _, _, _) = logout_retrying_busy(&mut client).await?;
        if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err("E2EF4 final logout refused".into());
        }
        Ok(())
    }.await;
    if let Err(error) = economy_social {
        evidence(&format!("E2EF4_GAP lifecycle {error}"));
    }
}
