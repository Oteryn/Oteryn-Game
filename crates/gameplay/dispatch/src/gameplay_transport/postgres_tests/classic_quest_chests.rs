use super::TestResult;
use super::quest_catalogue_chests_fixtures::{cases_including, execute_with_regions};

#[test]
fn classic_quest_chests_mint_crystal_rewards_from_retained_placements() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }

    let names = [
        "kv.ironhelmet",
        "kv.katanacorpse1",
        "kv.katanacorpse2",
        "kv.longsword1",
        "kv.longsword2",
        "kv.longsword3",
        "kv.madmageroom1",
        "kv.madmageroom2",
        "kv.madmageroom3",
        "kv.minohell1",
        "kv.minohell2",
        "kv.minohell3",
        "quest.key.id3620",
        "quest.key.id3667",
        "quest.key.id4603",
        "quest.u7_3.the_medusa_quest",
        "quest.u7_8.knight_outfits.ramsays_helmet_chest",
        "quest.u8_0.shardsof_ancient_winters.formorgar_mines",
    ];
    let selected = |claim: &str| {
        claim
            .strip_prefix("oteryn:reward-claim.")
            .is_some_and(|name| names.contains(&name))
    };
    let cases: Vec<_> = cases_including(selected)?
        .into_iter()
        .filter(|case| selected(&case.claim))
        .collect();
    assert_eq!(cases.len(), names.len());
    execute_with_regions(cases, true)
}
