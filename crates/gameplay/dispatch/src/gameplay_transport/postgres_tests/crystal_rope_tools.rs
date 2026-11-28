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
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Crystal rope dispatch panicked")?
        .map_err(Into::into)
}

#[test]
fn crystal_rope_tools_resolve_a_durable_ground_item_to_its_tile() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(|| {
            super::fixture::run(async |admin| {
                for (id, spot) in [(3003, 386), (646, 386), (9594, 386), (9596, 21967), (9598, 386)] {
                    let key = format!("oteryn:item.tibia.i{id}");
                    for source in [
                        super::vertical_move_fixtures::RopeUse::GroundHandle,
                        super::vertical_move_fixtures::RopeUse::GroundHandleExplicit,
                    ] {
                        super::vertical_move_fixtures::agent_rope(admin.clone(), &key, spot, source).await?;
                    }
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Crystal Ground item rope dispatch panicked")?
        .map_err(Into::into)
}

#[test]
fn secret_agent_ropes_from_a_covered_spot_by_object() -> TestResult {
    agent_rope_case(super::vertical_move_fixtures::RopeUse::CoveredObject)
}

#[test]
fn secret_agent_ropes_from_a_covered_spot_by_handle() -> TestResult {
    agent_rope_case(super::vertical_move_fixtures::RopeUse::CoveredHandle)
}

#[test]
fn secret_agent_jams_require_object_reach() -> TestResult {
    agent_rope_case(super::vertical_move_fixtures::RopeUse::DistantObject)
}

#[test]
fn secret_agent_jams_require_map_handle_reach() -> TestResult {
    agent_rope_case(super::vertical_move_fixtures::RopeUse::DistantHandle)
}

#[test]
fn secret_agent_jams_award_bad_timing_and_send_feedback() -> TestResult {
    agent_rope_case(super::vertical_move_fixtures::RopeUse::Jam)
}

fn agent_rope_case(action: super::vertical_move_fixtures::RopeUse) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || {
            super::fixture::run(async |admin| {
                for (id, spot) in [(9594, 386), (9596, 21967), (9598, 386)] {
                    let key = format!("oteryn:item.tibia.i{id}");
                    super::vertical_move_fixtures::agent_rope(admin.clone(), &key, spot, action).await?;
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Crystal rope dispatch panicked")?
        .map_err(Into::into)
}
