//! TibiaWiki spoiler curid 12355 oldid 891457, missions 4 and 5.
use super::{TestResult, quest_catalogue_chests};

#[test]
fn guardian_reward_bags_and_lost_heroes_book_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }

    let started = std::time::Instant::now();
    let mut cases = quest_catalogue_chests::cases()?;
    cases.retain(|chest| {
        chest.claim.ends_with(".dreamers_challenge.chest_book")
            || chest.claim.ends_with(".dreamers_challenge.chests_wine")
    });
    assert_eq!(cases.len(), 5);
    for chest in &cases {
        if chest.unique == 6196 {
            assert_eq!(chest.root, "oteryn:item.tibia.i2821");
            assert_eq!(chest.quantity, 1);
            assert_eq!(
                chest.texts,
                vec![(
                    "oteryn:item.tibia.i2821".to_owned(),
                    "Askuro\nMantar\nIldoni\nMaxmof\nXemodo\nOxanti".to_owned(),
                )]
            );
        } else {
            assert!((6197..=6200).contains(&chest.unique));
            assert_eq!(chest.root, "oteryn:item.tibia.i2853");
            assert_eq!(chest.quantity, 1);
            let mut contents = chest.contents.clone();
            contents.sort();
            assert_eq!(
                contents,
                vec![
                    ("oteryn:item.tibia.i2874".to_owned(), 1, 0),
                    ("oteryn:item.tibia.i3602".to_owned(), 1, 0),
                    ("oteryn:item.tibia.i6498".to_owned(), 1, 0),
                ]
            );
            assert_eq!(
                chest.fluids,
                vec![("oteryn:item.tibia.i2874".to_owned(), 2)]
            );
        }
    }
    quest_catalogue_chests::execute(cases)?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED guardian_reward_bags_and_lost_heroes_book_dispatch duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn northwest_lever_requires_the_previous_three() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }

    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("qcx-dreamer-wall-order", || {
        super::quest_world_callbacks::scenario(
            "m07/dreamer-lever-north",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 32761,
                y: 32289,
                floor: -14,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED northwest_lever_requires_the_previous_three duration={:?}",
        started.elapsed()
    );
    Ok(())
}
