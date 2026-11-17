//! Exact source rewards through the existing production chest catalogue harness.
use super::TestResult;
use super::quest_catalogue_chests::{cases, execute};

fn claim_case(
    suffix: &str,
    position: (u16, u16, i8),
    unique: u16,
    appearance: u16,
    item: u16,
) -> TestResult {
    let claim = format!("oteryn:reward-claim.quest.{suffix}");
    let selected: Vec<_> = cases()?
        .into_iter()
        .filter(|case| case.claim == claim)
        .collect();
    let [case] = selected.as_slice() else {
        return Err("exact qaitems claim missing or duplicated".into());
    };
    assert_eq!(case.position, position);
    assert_eq!(case.unique, unique);
    assert_eq!(case.appearance, appearance);
    assert_eq!(case.root, format!("oteryn:item.tibia.i{item}"));
    assert_eq!(case.quantity, 1);
    assert!(case.servable);
    assert!(case.contents.is_empty());
    if item == 5956 {
        assert_eq!(case.texts.len(), 1);
        assert_eq!(case.texts[0].0, case.root);
        assert_eq!(
            case.texts[0].1,
            "Damn you Thais! Damn you TBI! I am no longer an agent or a noble but a black knight. A dark avenger that will have his revenge!"
        );
    } else {
        assert!(case.texts.is_empty());
    }
    // This shared harness drives real USE, inventory/text persistence, second
    // command refusal and durable replay; no separate reward MINT is injected.
    execute(selected)
}

#[test]
fn qaitems_boots_source_use_inventory_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    let result = claim_case(
        "u8_4.the_hidden_city_of_beregar.firewalker_boots",
        (32582, 31405, -15),
        6284,
        2472,
        9019,
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "QAITEMS_DISPATCH_EXECUTED boots duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qaitems_hammer_source_use_inventory_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    let result = claim_case(
        "u7_6.explorer_society.giant_smith_hammer.hammer",
        (32776, 32253, -8),
        6078,
        2472,
        3208,
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "QAITEMS_DISPATCH_EXECUTED hammer duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qaitems_lottery_source_use_inventory_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    let result = claim_case(
        "u8_1.secret_service.lottery_ticket",
        (32872, 31958, -11),
        6250,
        2469,
        5956,
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "QAITEMS_DISPATCH_EXECUTED lottery duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qaitems_poem_source_use_inventory_and_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    let result = claim_case(
        "u8_1.secret_service.poem_scroll",
        (32643, 32733, -7),
        6251,
        2472,
        347,
    );
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!(
        "QAITEMS_DISPATCH_EXECUTED poem duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
