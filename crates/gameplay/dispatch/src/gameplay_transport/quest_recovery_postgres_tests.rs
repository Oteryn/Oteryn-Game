//! A scenario tag may be shared; its retained recovery authority must not be.

use super::TestResult;
use super::fixture::{Harness, debug, run};

#[test]
fn same_tag_fixtures_preserve_each_others_retained_recovery_authority() -> TestResult {
    run(async |admin| {
        let first = Harness::create(admin.clone(), "quest_recovery_isolation", true).await?;
        let second = Harness::create(admin, "quest_recovery_isolation", true).await?;
        for harness in [&first, &second] {
            let seal = harness.recovery.seal_current().map_err(debug)?;
            harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
        }
        first.cleanup().await?;
        // Cleaning one fixture must leave the other's sealed authority usable.
        {
            let seal = second.recovery.seal_current().map_err(debug)?;
            second
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
        }
        second.cleanup().await
    })
}
