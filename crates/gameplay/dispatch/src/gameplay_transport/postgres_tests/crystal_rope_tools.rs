use super::TestResult;

#[test]
fn crystal_agent_tools_and_elvenhair_rope_ascend_through_live_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(|| {
            super::fixture::run(async |admin| {
                for (id, spot) in [(9594, 386), (9596, 21967), (9598, 386), (646, 386)] {
                    let key = format!("oteryn:item.tibia.i{id}");
                    super::vertical_move_fixtures::physical_rope(admin.clone(), &key, spot).await?;
                    if id != 646 {
                        super::vertical_move_fixtures::jammed_rope(admin.clone(), &key, spot).await?;
                    }
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Crystal rope dispatch panicked")?
        .map_err(Into::into)
}
