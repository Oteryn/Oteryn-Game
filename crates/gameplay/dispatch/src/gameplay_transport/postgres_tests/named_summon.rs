//! C13: spoken summon through the production chat and spell dispatch path.
use super::TestResult;
use super::parameter_spell_fixtures::{NAMED_SUMMON, scenario};

#[test]
fn named_summon_uses_canonical_rat_when_donor_alias_is_also_admitted() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    scenario(NAMED_SUMMON)
}
