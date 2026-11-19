//! The source timer policy must be admitted before the ruby can transform a helmet.
use super::TestResult;

#[test]
fn l6_ruby_profile_admits_exact_helmet_absolute_policy() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED l6-ruby-profile");
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::quest_world_callbacks_fixtures::scenario(
                "l6-ruby-profile",
                oteryn_gameplay_quests::quest::world_callbacks::Position {
                    x: 7,
                    y: 1,
                    floor: -7,
                },
                false,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "ruby profile scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED l6-ruby-profile duration={:?}",
        started.elapsed()
    );
    Ok(())
}
